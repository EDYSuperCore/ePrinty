# 删除打印机清理级别功能实现总结

## 实现概述

已成功实现 Windows 下的"清理级别（单选）"方案，将删除确认弹窗改为三个级别的单选选项，并支持彻底清理的二次确认。macOS 保持简单确认对话框。

## 已完成的修改

### 一、后端实现

#### 1. 扩展 DeletePrinterResult 结构体

**文件**: `src-tauri/src/platform/mod.rs`

**新增字段**:
- `removed_driver: bool` - 是否删除驱动
- `driver_name: Option<String>` - 驱动名称
- `port_name: Option<String>` - 端口名称

#### 2. 扩展 delete_printer 命令参数

**文件**: `src-tauri/src/main.rs`

**新增参数**:
- `remove_driver: Option<bool>` - 是否删除驱动（默认 false）

**默认值**:
- `remove_port`: 默认 `false`（更安全）
- `remove_driver`: 默认 `false`

#### 3. Windows 驱动删除实现

**文件**: `src-tauri/src/platform/windows/delete.rs`

**新增函数**:
- `get_printer_info()` - 获取打印机端口和驱动信息
- `count_driver_usage()` - 检查驱动是否被其他队列使用
- `delete_printer_driver()` - 删除打印机驱动（安全删除）

**安全删除策略**:
1. 获取该队列的 `DriverName`
2. 检查是否存在其他打印机队列仍在使用相同 `DriverName`
   - 如果 `count > 1`：拒绝删除驱动，返回 `removed_driver=false`，并在 message 中说明原因
   - 如果 `count == 1`：尝试删除驱动（使用 PowerShell `Remove-PrinterDriver`）
3. 驱动删除失败不影响队列删除成功

**更新 `delete_printer_windows()` 函数**:
- 支持 `remove_driver` 参数
- 删除队列前先获取端口和驱动信息
- 按顺序执行：队列删除 → 端口删除 → 驱动删除（安全检查）

#### 4. macOS 结构体更新

**文件**: `src-tauri/src/platform/macos/delete.rs`

- 更新 `DeletePrinterResultInternal` 结构体，添加新字段
- macOS 不支持驱动删除，`removed_driver` 始终为 `false`

### 二、前端实现

#### 1. 新增数据属性

**文件**: `src/App.vue`

**新增**:
- `showDeleteConfirmDialog: false` - 显示删除确认对话框（Windows）
- `deleteConfirmDialog` - 删除确认对话框状态（包含 `selectedLevel`）
- `showFullCleanupConfirmDialog: false` - 显示彻底清理二次确认
- `fullCleanupConfirmDialog` - 彻底清理确认对话框状态

#### 2. Windows 清理级别选择弹窗

**模板位置**: `src/App.vue` (在错误提示对话框之前)

**功能**:
- 标题：确认删除打印机？
- 说明：你可以选择删除范围。范围越大，清理越彻底，但可能影响其他打印机。
- 三个单选选项：
  - **A) queue**: 仅删除打印机（推荐）
    - 描述：移除该打印机队列，不影响驱动和其他打印机
  - **B) queue_port**: 删除打印机 + 端口（网络异常时推荐）
    - 描述：同时删除网络端口，下次安装会重新创建
  - **C) full**: 彻底清理（高级）
    - 描述：同时删除驱动。可能影响使用相同驱动的其他打印机
- 默认选中 A（queue）

#### 3. 彻底清理二次确认弹窗

**模板位置**: `src/App.vue` (在 Windows 删除确认对话框之后)

**触发条件**: 用户选择 `full` 级别并点击"删除"按钮

**内容**:
- 标题：确认彻底清理？
- 内容：该操作会删除打印驱动，可能影响系统中使用相同驱动的其他打印机。是否继续？
- 按钮：取消 / 继续清理

#### 4. 修改 handleDelete 方法

**文件**: `src/App.vue`

**逻辑**:
1. **Windows 平台**:
   - 显示清理级别选择对话框
   - 根据选择的级别设置参数：
     - `queue`: `removePort=false, removeDriver=false`
     - `queue_port`: `removePort=true, removeDriver=false`
     - `full`: `removePort=true, removeDriver=true`（需要二次确认）
2. **macOS 平台**:
   - 显示简单确认对话框
   - `removePort=false, removeDriver=false`

**新增方法**:
- `showDeleteConfirmDialogAsync()` - 显示删除确认对话框（Windows）
- `confirmDeleteDialogAction()` - 确认删除（处理 full 的二次确认）
- `cancelDeleteConfirmDialog()` - 取消删除确认
- `showFullCleanupConfirmDialogAsync()` - 显示彻底清理二次确认
- `confirmFullCleanupDialogAction()` - 确认彻底清理
- `cancelFullCleanupConfirmDialog()` - 取消彻底清理确认

#### 5. 删除后状态收敛（保持不变）

删除成功后执行：
- A) 立即更新 runtime 状态为 `not_installed`
- B) 清理 `installedKeyMap` 缓存
- C) 触发重新检测（debounce 机制）

## 关键特性

### 1. 平台差异化

- **Windows**: 显示清理级别选择弹窗（三个级别）
- **macOS**: 显示简单确认对话框（不提供端口/驱动选项）

### 2. 安全删除策略

- **队列删除**: 必须成功（使用 Win32 `DeletePrinter`）
- **端口删除**: 尽力而为（仅删除 IP_ 前缀端口）
- **驱动删除**: 安全删除（仅当没有其他队列使用时才删除）

### 3. 二次确认机制

- 选择 `full` 级别时，弹出二次确认对话框
- 明确提示风险：可能影响其他打印机

### 4. 参数映射

| 清理级别 | removePort | removeDriver |
|---------|-----------|--------------|
| queue | false | false |
| queue_port | true | false |
| full | true | true |

## 文件修改清单

### 后端文件

1. `src-tauri/src/platform/mod.rs` - **修改**
   - 扩展 `DeletePrinterResult` 结构体
   - 更新 `delete_printer()` 函数签名

2. `src-tauri/src/main.rs` - **修改**
   - 扩展 `delete_printer()` 命令参数

3. `src-tauri/src/platform/windows/delete.rs` - **修改**
   - 扩展 `DeletePrinterResultInternal` 结构体
   - 新增 `get_printer_info()` 函数
   - 新增 `count_driver_usage()` 函数
   - 新增 `delete_printer_driver()` 函数
   - 更新 `delete_printer_windows()` 函数

4. `src-tauri/src/platform/macos/delete.rs` - **修改**
   - 扩展 `DeletePrinterResultInternal` 结构体

### 前端文件

1. `src/App.vue` - **修改**
   - 新增数据属性（删除确认对话框状态）
   - 新增 Windows 清理级别选择弹窗模板
   - 新增彻底清理二次确认弹窗模板
   - 修改 `handleDelete()` 方法
   - 新增对话框处理方法

## 测试检查清单

### Windows 测试

- [ ] 选择"仅删除打印机"：
  - [ ] 仅队列消失
  - [ ] 端口/驱动不动
  - [ ] App 状态最终未安装

- [ ] 选择"删除打印机 + 端口"：
  - [ ] 队列消失
  - [ ] 若端口为 IP_ 开头则端口也尝试删除
  - [ ] 失败不阻断

- [ ] 选择"彻底清理"：
  - [ ] 弹出二次确认
  - [ ] 若驱动被其他队列使用：队列删除成功，驱动跳过，提示原因
  - [ ] 若未被其他队列使用：尝试删除驱动；失败不阻断但有 evidence

### macOS 测试

- [ ] 删除流程仍是简单确认
- [ ] 删除后状态收敛正确

## 注意事项

1. **驱动删除安全性**: 仅当没有其他队列使用该驱动时才删除，避免影响其他打印机
2. **端口删除**: 仅删除 `IP_` 前缀端口，避免误删共享端口
3. **失败处理**: 端口和驱动删除失败不影响队列删除成功
4. **日志**: 所有操作都有 `[DeletePrinter]` 前缀日志，包含详细 evidence
5. **状态收敛**: 删除后必须清理缓存并触发重新检测，确保最终状态正确
