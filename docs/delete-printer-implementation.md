# 删除打印机功能实现总结

## 实现概述

已成功实现删除打印机功能，将原有的"重新安装"功能替换为"删除打印机"，支持 Windows 和 macOS 平台。

## 已完成的修改

### 一、后端实现

#### 1. Windows 平台删除实现 (`src-tauri/src/platform/windows/delete.rs`)

**功能**:
- ✅ 使用 Win32 API `DeletePrinter` 删除打印机队列（必须）
- ✅ 支持删除端口（尽力而为，仅删除 IP_ 前缀端口）
- ✅ 幂等处理：如果队列不存在，返回 success=true（不报错）
- ✅ 端口删除失败不影响队列删除成功

**实现细节**:
- 使用 `OpenPrinterW` 打开打印机（需要 `PRINTER_ACCESS_ADMINISTER` 权限）
- 调用 `DeletePrinter` 删除队列
- 删除队列前先获取端口信息（删除后无法获取）
- 使用 PowerShell `Remove-PrinterPort` 删除端口（尽力而为）

#### 2. macOS 平台删除实现 (`src-tauri/src/platform/macos/delete.rs`)

**功能**:
- ✅ 使用 `lpadmin -x` 删除打印机队列
- ✅ 幂等处理：预检查队列是否存在，不存在时返回 success=true
- ✅ 错误识别：识别 "Unknown printer" / "does not exist" 等错误，视为幂等

**实现细节**:
- 使用 `lpstat -p` 预检查队列是否存在
- 使用 `lpadmin -x` 删除队列
- 识别常见错误信息，实现幂等处理

#### 3. 平台抽象层 (`src-tauri/src/platform/mod.rs`)

**新增**:
- `DeletePrinterResult` 结构体（统一返回格式）
- `delete_printer()` 函数（平台统一入口）

#### 4. Tauri 命令 (`src-tauri/src/main.rs`)

**新增**:
- `delete_printer()` Tauri 命令
- 参数：`printer_name: String`, `remove_port: Option<bool>`
- 返回：`DeletePrinterResult`

**日志**:
- `[DeletePrinter][Command] ENTER ...`
- `[DeletePrinter][Command] EXIT ...`

### 二、前端实现

#### 1. PrinterItem.vue 修改

**修改**:
- ✅ 将"重新安装（不推荐）"按钮改为"删除打印机"
- ✅ 按钮样式改为红色（`text-red-600 hover:bg-red-50`）
- ✅ 添加删除图标
- ✅ `handleReinstall()` 改为 `handleDelete()`
- ✅ 事件从 `@reinstall` 改为 `@delete`

#### 2. App.vue 修改

**新增 `handleDelete()` 方法**:
- ✅ 二次确认对话框（必须）
  - 标题：确认删除打印机
  - 内容：将从系统中移除该打印机队列（不删除驱动）。删除后可重新安装。
- ✅ 调用后端 `delete_printer` 命令
- ✅ 删除成功后执行三件事：
  - **A) 立即更新 runtime 状态**：`detectState = 'not_installed'`
  - **B) 清理 installedKeyMap**：删除该打印机的缓存映射
  - **C) 触发重新检测**：使用 debounce 机制，确保最终状态正确

**删除失败处理**:
- 不改变已知 installed 状态（避免误导）
- 显示错误 toast，包含 message 和 evidence

## 关键特性

### 1. 幂等性

- **Windows**: 如果队列不存在（错误码 1801 或 2），返回 success=true，removed_queue=false
- **macOS**: 如果队列不存在（lpstat 或 lpadmin 返回 not found），返回 success=true，removed_queue=false

### 2. 端口删除（Windows）

- 仅删除 `IP_` 前缀的端口
- 删除队列前先获取端口信息（删除后无法获取）
- 端口删除失败不影响队列删除成功
- 使用 PowerShell `Remove-PrinterPort`（尽力而为）

### 3. 状态收敛保证

删除成功后：
1. 立即更新 UI 状态为 `not_installed`
2. 清理 `installedKeyMap` 缓存
3. 触发重新检测（debounce 机制）

### 4. 错误处理

- 删除失败时显示明确错误信息
- 不影响已加载的配置和状态
- 日志包含详细 evidence（错误码、stderr 等）

## 文件修改清单

### 后端文件

1. `src-tauri/src/platform/windows/delete.rs` - **新建**
   - Windows 平台删除实现

2. `src-tauri/src/platform/windows/mod.rs` - **修改**
   - 添加 `pub mod delete;`

3. `src-tauri/src/platform/macos/delete.rs` - **新建**
   - macOS 平台删除实现

4. `src-tauri/src/platform/macos.rs` - **修改**
   - 添加 `pub mod delete;`

5. `src-tauri/src/platform/mod.rs` - **修改**
   - 添加 `DeletePrinterResult` 结构体
   - 添加 `delete_printer()` 函数

6. `src-tauri/src/main.rs` - **修改**
   - 添加 `delete_printer()` Tauri 命令
   - 在 `invoke_handler` 中注册命令

### 前端文件

1. `src/components/PrinterItem.vue` - **修改**
   - 按钮文案和样式
   - `handleReinstall()` → `handleDelete()`
   - `@reinstall` → `@delete`

2. `src/App.vue` - **修改**
   - `handleReinstall()` → `handleDelete()`
   - `@reinstall` → `@delete`
   - 实现删除逻辑和状态收敛

## 测试检查清单

### Windows 测试

- [ ] 已安装队列：点击删除 → 队列从系统消失，App 状态最终变为未安装
- [ ] 不存在队列：点击删除 → 返回 success=true（幂等），App 状态最终为未安装
- [ ] 端口删除：队列删除后，若 PortName=IP_xxx，尝试删除端口
- [ ] 端口删除失败：不影响队列删除成功

### macOS 测试

- [ ] 已安装队列：删除成功，lpstat 看不到，App 状态最终未安装
- [ ] 不存在队列：删除操作不应报致命错误（幂等），App 状态最终未安装

### 状态收敛测试

- [ ] 删除成功后：installedKeyMap 中该打印机被删除
- [ ] 删除成功后：触发重新检测，最终状态正确
- [ ] 删除失败时：不改变已知 installed 状态

## 注意事项

1. **权限要求**: Windows 删除需要管理员权限（`PRINTER_ACCESS_ADMINISTER`）
2. **端口删除**: 仅删除 `IP_` 前缀端口，其他端口不删除
3. **驱动删除**: 本期不做，后续如需要另做"彻底清理"模式
4. **状态收敛**: 删除后必须触发重新检测，确保最终状态正确
5. **日志**: 所有操作都有 `[DeletePrinter]` 前缀日志，便于调试

## 与现有功能的兼容性

- ✅ 不影响安装功能
- ✅ 不影响检测功能
- ✅ 不影响配置加载
- ✅ 保持 macOS 整体逻辑与权限前提不变
