# 配置加载/落盘路径策略重构总结

## 重构目标

按"本地优先 + 异步远程更新（仅版本变更才落盘）"重构配置加载/落盘路径策略：
- Windows: 只允许 exe 同目录配置
- macOS: 使用应用程序库目录（Application Support）
- 启动顺序：先本地，后异步远程
- 只有远程版本更高时才更新本地
- 配置更新后触发打印机状态重新检测

## 已完成的修改

### 1. 统一配置路径策略 (`get_config_path`)

**文件**: `src-tauri/src/main.rs`

- 新增 `get_config_path()` 函数作为统一入口
- **Windows**: 只允许 `exe_dir/printer_config.json`
- **macOS**: 使用 `app_config_dir/printer_config.json` (Application Support)
- **其他平台**: 使用 `app_config_dir/printer_config.json`
- `get_local_config_path()` 保留为别名，保持向后兼容

### 2. 重写 seed 配置复制逻辑 (`seed_config_if_needed`)

**文件**: `src-tauri/src/main.rs`

- **Windows**: 
  - 只允许写入 exe 同目录
  - 写入失败时明确报错，不允许回退到 AppData
  - 错误信息提示用户手动放置配置文件
- **macOS**: 保持现有逻辑，写入 Application Support 目录

### 3. 重构配置加载命令 (`get_cached_config`)

**文件**: `src-tauri/src/main.rs`

**新逻辑**:
1. 如果本地配置存在 → 直接读取并返回 `source="local"`
2. 如果本地不存在 → 尝试从 seed 复制
   - 成功 → 读取并返回 `source="seed"`
   - 失败 → 继续到步骤 3
3. seed 也不可用 → **同步拉取远程配置**（首次启动兜底）
   - 成功 → 保存到本地并返回 `source="remote_bootstrap"`
   - 失败 → 返回明确错误（阻断式）

**关键点**:
- ✅ 禁止"先检查远程，再加载本地"
- ✅ 只有本地不存在时才允许同步远程
- ✅ 远程失败时明确报错，不静默失败

### 4. 实现版本比较的远程刷新 (`refresh_remote_config`)

**文件**: `src-tauri/src/main.rs`

**新逻辑**:
1. 读取本地配置版本（如果存在）
2. 请求远程配置（3秒超时）
3. **版本比较策略**:
   - `local_version` 存在且 `remote_version <= local_version` → **不更新，不落盘，不发送事件**
   - `local_version` 不存在（但文件存在）→ 默认需要更新（引入版本字段）
   - `local_version` 不存在且文件不存在 → 允许更新
4. 如果需要更新:
   - 使用 `save_config_to_local()` 原子替换（先写 tmp，再 rename）
   - 发送 `config_updated` 事件（包含 `updated=true`）
5. 如果远程失败:
   - 发送 `config_refresh_failed` 事件
   - **不影响已加载的本地配置**

### 5. 前端配置更新事件处理

**文件**: `src/App.vue`

**修改**:
- 监听 `config_updated` 事件
- 只有 `payload.updated === true` 时才真正更新配置
- **触发重新检测打印机状态**（确保最终一致性）:
  - 如果检测正在运行 → debounce，等待完成后立即再跑一次
  - 如果检测未运行 → 立即触发 `startDetectInstalledPrinters()`

### 6. 更新无 AppHandle 场景的配置加载 (`load_local_config`)

**文件**: `src-tauri/src/main.rs`

- **Windows**: 只搜索 exe 同目录，不再搜索多个路径
- **macOS/其他**: 保持多路径搜索（开发模式兼容）

### 7. 更新兼容函数 (`load_config`)

**文件**: `src-tauri/src/main.rs`

- 使用新的 `get_config_path()` 统一路径策略
- 保持向后兼容逻辑

### 8. 删除 AppData 相关代码

- ✅ 删除了 `get_local_config_dir()` 函数（使用 AppData）
- ✅ 删除了 `get_cache_config_path()` 函数（已废弃）
- ✅ 所有 Windows 路径逻辑统一到 exe 同目录

## 关键约束满足情况

| 约束 | 状态 | 说明 |
|------|------|------|
| A) Windows 只允许 exe 同目录 | ✅ | `get_config_path()` Windows 分支只返回 exe 同目录 |
| B) macOS 使用 Application Support | ✅ | `get_config_path()` macOS 分支使用 `app_config_dir` |
| C) 启动顺序：先本地，后异步远程 | ✅ | `get_cached_config` 先读本地，`refresh_remote_config` 后台执行 |
| D) 只有版本更高才更新 | ✅ | `refresh_remote_config` 实现版本比较逻辑 |
| E) 禁止先远程后本地 | ✅ | `get_cached_config` 只有本地不存在才同步远程 |
| F) 打印机检测最终一致性 | ✅ | `config_updated` 事件触发重新检测 |

## 测试检查清单

### Windows 测试

- [ ] 确认不再创建/读取 `%APPDATA%\easyPrinter\config\printer_config.json`
- [ ] 把 `printer_config.json` 放在 exe 同目录，启动必须能读取并渲染
- [ ] 删除 exe 同目录配置后：
  - [ ] 若 exe_dir 可写且有 seed：应复制 seed 并启动成功
  - [ ] 若无 seed 且远程可用：应 remote_bootstrap 后落盘到 exe_dir 并启动成功
  - [ ] 若无 seed 且远程不可用：应明确失败（阻断式错误）
- [ ] 远程配置版本未提升：不落盘、不触发 `config_updated`
- [ ] 远程配置版本提升：先写 tmp 再替换，触发 `config_updated`

### macOS 测试

- [ ] 本地优先读取 Application Support（或 app_config_dir）
- [ ] 远程更新可写入该目录并替换旧版本
- [ ] 行为顺序仍为：先本地渲染，后异步远程刷新

### 打印机状态测试

- [ ] 启动检测一次
- [ ] 若远程更新触发 `config_updated`：必须再检测一次或确保重新匹配，最终状态正确

## 文件修改清单

1. `src-tauri/src/main.rs`
   - 新增 `get_config_path()` 函数
   - 修改 `seed_config_if_needed()` 函数
   - 修改 `get_cached_config()` 函数
   - 修改 `refresh_remote_config()` 函数
   - 修改 `load_local_config()` 函数
   - 修改 `load_config()` 函数
   - 删除 `get_local_config_dir()` 函数
   - 删除 `get_cache_config_path()` 函数

2. `src/App.vue`
   - 修改 `setupConfigUpdateListener()` 方法

## 注意事项

1. **版本字段**: 配置文件的 `version` 字段是 `Option<String>`，版本比较使用简单的字符串比较。如需语义化版本比较，可以使用 `semver` crate。

2. **原子替换**: `save_config_to_local()` 已实现原子替换（先写 tmp，再 rename），确保不会出现半写入状态。

3. **错误处理**: Windows 下如果 exe 目录无写权限，会明确报错，不允许回退到 AppData。

4. **向后兼容**: `get_local_config_path()` 保留为 `get_config_path()` 的别名，确保现有代码不受影响。

5. **打印机检测**: 配置更新后通过 debounce 机制确保重新检测，避免并发检测导致的状态不一致。
