# eprinty 架构与代码合理性 Review 报告

## 0. Executive Summary
- 结论：当前可运行但分层缺失、状态机/事件契约松散，易出现“假成功/卡死”与维护成本高的风险；综合评分 **6.5/10**（介于可维护与勉强可维护之间）。
- Top 5 风险（按严重度）：
  1. **自检事件误占用活跃任务槽位**：启动 800ms 即发送 `job.init` 自检事件，前端监听器把 `selfcheck` 当真实任务并弹出安装对话框，后续真实安装可能与之竞争，存在 UI 错乱/丢激活风险（P0，见 [src-tauri/src/main.rs#L1340-L1395](src-tauri/src/main.rs#L1340-L1395)、[src/App.vue#L2480-L2595](src/App.vue#L2480-L2595)）。
  2. **后端输入契约缺失导致“静默兜底”**：`install_printer` 仅校验名称，`installMode` 非法值回退 `auto`，`dryRun` 默认 true；缺乏路径/策略校验，调用方缺参时被默认值掩盖，易产生与期望不符的行为且前端无感知（P1，见 [src-tauri/src/main.rs#L900-L986](src-tauri/src/main.rs#L900-L986)）。
  3. **前端 finalize 强制成功掩盖缺失事件**：`finalizeFromInvoke` 将缺少终态事件的任务一律补齐为 success/skip，使事件流缺失或后端异常被“伪成功”覆盖，导致状态观测不可信（P1，见 [src/stores/installProgress.ts#L283-L360](src/stores/installProgress.ts#L283-L360)）。
  4. **单文件巨石与重复状态源**：`App.vue` ~3000 行，集成配置加载/检测/安装/日志/更新/设置，且保留 legacy 安装进度与 Pinia store 并存，状态可能分叉且难测（P1，见 [src/App.vue#L1405-L2595](src/App.vue#L1405-L2595)）。
  5. **事件 schema 缺乏 runtime 校验与乱序治理**：前端仅做字段 rename，不校验 `jobId/stepId/state` 有效性，也无去重/序列保障；未知 step 直接写入 steps，可能导致 UI 一直 running 或混入噪声（P1，见 [src/App.vue#L2480-L2595](src/App.vue#L2480-L2595)、[src/stores/installProgress.ts#L95-L210](src/stores/installProgress.ts#L95-L210)）。
- 优先级建议：P0 立即收敛（自检事件与监听逻辑）；P1 在下一次迭代前完成（输入契约、状态机终态、拆分 UI/状态）；P2 作为后续优化（目录整理、命名一致性、日志结构化）。

## 1. 项目结构与分层评估
- 当前 UI/Store/Domain/Infra 未形成单向依赖：`App.vue` 直接调用 Tauri 命令、处理业务判定与 UI 绑定，Pinia store 仅被动更新；缺少独立 domain/service 层。
- Store 未被视为单一真相源：组件保留 legacy `installProgress` / `reinstallProgress` 本地状态，与 Pinia `installProgressStore` 并行，存在多源事实。
- Infra（Tauri commands）暴露到 UI，无 service 封装或输入校验；UI 逻辑难测且重复（检测/安装/更新等流程都在 `App.vue`）。
- 建议：引入 `domain/install` 服务（封装 invoke、事件 listener、schema 校验），组件只消费 store 派生状态；将检测/安装/更新拆到独立 composable 或模块化 view。

## 2. 状态管理与状态机合理性
- 多源状态：Pinia `jobs` 与 `App.vue` 中的本地 `installProgress`/`reinstallProgress`/`printerRuntime` 同时描述安装与检测，未有统一判定函数，易出现视图与真实状态不一致（见 [src/App.vue#L946-L1064](src/App.vue#L946-L1064)、[src/App.vue#L1925-L2085](src/App.vue#L1925-L2085)）。
- 终态确定性不足：前端依赖 `finalizeFromInvoke` 人为补终态（P1），若后端漏发事件则强制成功，失去“事件即真相”的确定性（[src/stores/installProgress.ts#L283-L360](src/stores/installProgress.ts#L283-L360)）。
- Job/Step schema 运行时未校验：`applyEvent` 直接写入，未限制 state/stepId 枚举，未知 step 仍加入 `steps`，但 `jobState` 只看 `STEPS_PLAN`，造成 jobState 与 steps 不一致的潜在悬挂（[src/stores/installProgress.ts#L95-L210](src/stores/installProgress.ts#L95-L210)）。

## 3. 事件与进度模型
- 事件 schema 无统一验证：前端仅做字段兼容，无必填校验/去重/序列控制；乱序或重复事件可能覆盖终态，缺少 `tsMs`/序列号判定。
- 自检事件干扰真实任务：后端启动自发 `job.init`（selfcheck），前端未过滤激活逻辑，可能把 selfcheck 置为 activeJob 并打开安装弹窗（P0，见 [src-tauri/src/main.rs#L1340-L1395](src-tauri/src/main.rs#L1340-L1395)、[src/App.vue#L2480-L2595](src/App.vue#L2480-L2595)）。
- StepId 魔法字符串仍存在：前端 hardcode `DISPLAY_STEP_IDS`，后端在 driver/port/queue 之外若新增 step 会被归为“未知”但仍入 steps map，UI 不展示且无告警。
- 乱序/丢事件策略缺失：未使用 `tsMs` 进行幂等/幂序，`updateJobState` 也不考虑 job.done 之外的终态覆盖，可能出现长期 running。

## 4. API/IPC 边界与契约
- `install_printer` 输入校验薄弱：仅检查 `name` 非空，`installMode` 非法值静默回退，`dryRun` 默认 true（掩盖缺参），`driverPath/model` 未校验存在/可访问性（[src-tauri/src/main.rs#L900-L986](src-tauri/src/main.rs#L900-L986)）。
- `reinstall_printer`/`list_printers` 等命令未暴露类型守卫/错误分类；前端直接 `invoke`，无 service 层做重试/可重试分类。
- 默认值掩盖错误：前端 install 调用缺 jobId 也会用 `activeJobId` 兜底，后端结果 success 会强制补齐步骤，用户难以察觉缺事件或失败。

## 5. 可测试性与可观测性
- Domain 不可单测：逻辑集中在 `App.vue`（UI 组件），缺乏纯函数或 service；难以在无 Tauri 环境下模拟事件流。
- 日志未结构化：前端覆盖 console 收集文本，缺少统一字段（jobId/stepId/state/elapsed/errorCode）；后端事件无 traceId/seq；难以重放和定位。
- 错误分类缺失：后端返回多为字符串，未区分可重试/权限/输入错误；前端也不根据错误类型给出分支处理。

## 6. 代码异味清单
- 巨石组件：`App.vue` 聚合配置加载、检测、安装、更新、调试窗口、设置等，>3000 行，认知负担高（[src/App.vue#L1-L2595](src/App.vue#L1-L2595)）。
- 重复/遗留状态：本地 `installProgress`/`reinstallProgress` 与 Pinia store 并存；legacy phase 兼容逻辑仍保留但 UI 不再使用。
- 魔法字符串：stepId/phase/installMode 驱动散落常量，缺集中枚举；driver install strategy `'always'/'reuse_if_installed'` 与 `'always_install_inf'/'skip_if_driver_exists'` 并存，易混淆（[src/settings/appSettings.ts#L4-L86](src/settings/appSettings.ts#L4-L86)、[src/App.vue#L2600-L2670](src/App.vue#L2600-L2670)）。
- 输入校验缺失：Tauri commands 宽松签名，前端未在 service 层校验参数可用性。
- 事件监听副作用：listener 内直接修改 UI 状态（打开弹窗），耦合事件与展示。

## 7. 重构建议与路线图
- P0（立即）：
  1) 过滤自检事件：listener 在设 activeJob 前检查 `jobId !== 'selfcheck'`（或给自检单独 channel），避免弹窗与占坑（改动点： [src/App.vue#L2480-L2595](src/App.vue#L2480-L2595)）。
  2) 在 install listener 中若 `jobId` 为空直接忽略并告警；保留 selfcheck 仅做日志，不入 store。
- P1（下次迭代前）：
  1) 抽象 install service：在 `src/services/install.ts` 封装 `invoke`/listener，集中 schema 校验（`jobId/stepId/state` 枚举、必填字段、tsMs 数值），并做去重/序列检查。
  2) 精简 `App.vue`：拆分为视图组件 + composable（如 `useInstallFlow`、`usePrinterDetect`），Pinia store 成为唯一真相源，移除 legacy `installProgress`/`reinstallProgress` 本地状态。
  3) 收敛终态：改 `finalizeFromInvoke` 为“只在缺终态时补失败”，避免强制成功；在后端 StepReporter + job.done 双保，缺事件则表面失败而非成功（改动点： [src/stores/installProgress.ts#L283-L360](src/stores/installProgress.ts#L283-L360)）。
  4) 强化命令校验：后端 `install_printer` 校验 `installMode` 枚举、`driverPath` 存在/可读、`driverInstallPolicy` 合法；默认值仅用于缺省而非纠错（[src-tauri/src/main.rs#L900-L986](src-tauri/src/main.rs#L900-L986)）。
  5) 统一策略枚举：在 shared model 定义 install mode / driver strategy 枚举，前后端共享，移除魔法字符串。
- P2（可选优化）：
  1) 引入事件 schema 校验库（zod/io-ts）生成类型守卫；事件流增加 `seq` 与去重缓存。
  2) 日志结构化：统一字段 `{jobId, stepId, state, ts, message, error.code}`，前端调试窗口按字段过滤。
  3) 增加单元测试：service 层对事件归并、终态判定、乱序处理编写无 UI 的测试；后端 StepReporter/job.done 端到端测试。

## 附录A：关键问题定位表
| 问题 | 严重级别 | 文件:行 | 证据片段 | 建议 |
| --- | --- | --- | --- | --- |
| 自检事件被当作真实任务激活弹窗 | P0 | [src-tauri/src/main.rs#L1340-L1395](src-tauri/src/main.rs#L1340-L1395)、[src/App.vue#L2480-L2595](src/App.vue#L2480-L2595) | 后端发 `job.init selfcheck`，前端 listener 在无 activeJob 时设为 active 并打开安装弹窗 | 过滤 selfcheck 或改用独立 channel；设 active 前校验 jobId/printerName 非 selfcheck |
| install_printer 输入契约缺失 | P1 | [src-tauri/src/main.rs#L900-L986](src-tauri/src/main.rs#L900-L986) | installMode 非法回退 auto，dryRun 默认 true，driverPath/model 未校验 | 在命令层做枚举校验与路径检查；默认值仅兜底不掩错，返回错误码分类 |
| finalizeFromInvoke 强制成功掩盖缺事件 | P1 | [src/stores/installProgress.ts#L283-L360](src/stores/installProgress.ts#L283-L360) | 缺事件时将 pending/running 步骤补为 skipped/success，jobState=success | 改为缺事件则标记 failed 并提示“事件缺失”，保持事件驱动一致性 |
| 巨石组件 + 重复状态源 | P1 | [src/App.vue#L1-L2595](src/App.vue#L1-L2595) | 单文件包含检测/安装/更新/设置/日志；本地 installProgress 与 Pinia 并存 | 拆分视图与逻辑，封装 composable/service，Pinia 成为唯一状态源 |
| 事件 schema 缺运行时校验 | P1 | [src/App.vue#L2480-L2595](src/App.vue#L2480-L2595)、[src/stores/installProgress.ts#L95-L210](src/stores/installProgress.ts#L95-L210) | listener 未校验必填字段/枚举，未知 step 直接入 map，缺去重/乱序处理 | 使用 schema 守卫校验 payload；基于 tsMs/seq 去重与终态保护，未知 step 仅日志不入 steps |

## 附录B：建议的目标目录结构
```
src/
  main.js
  app/
    App.vue (仅布局与路由壳)
  components/
    ...（纯展示组件，无业务）
  services/
    installService.ts   // 封装 invoke、事件监听、schema 校验
    detectService.ts    // 打印机检测与重试策略
    settingsService.ts  // 设置读写
  stores/
    installProgress.ts  // 单一真相源，纯数据，不含 UI 文案
    printerDetect.ts
  domain/
    models/             // StepId/State/InstallMode/DriverStrategy 枚举与类型
    stateMachines/      // 安装/检测状态机与判定函数
  utils/
    logging.ts          // 结构化日志、traceId、seq 生成
    format.ts
  views/
    InstallView.vue
    SettingsView.vue
    UpdateView.vue
```
