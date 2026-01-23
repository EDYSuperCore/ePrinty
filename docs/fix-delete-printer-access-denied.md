# 修复 Windows 删除打印机 Access Denied 错误

## 问题描述

Windows 下删除打印机队列时报错 `error_code=5` (Access Denied)，原因是 `OpenPrinterW` 句柄访问权限不足。

## 修复方案

### 1. 提升访问权限

**文件**: `src-tauri/src/platform/windows/delete.rs`

**修改前**:
```rust
DesiredAccess: 0x00000004, // PRINTER_ACCESS_ADMINISTER (删除权限)
```

**修改后**:
```rust
DesiredAccess: 0x000F000C, // PRINTER_ALL_ACCESS (包含删除权限)
```

**说明**:
- `PRINTER_ACCESS_ADMINISTER` (0x00000004) 在某些情况下可能权限不足
- `PRINTER_ALL_ACCESS` (0x000F000C) 包含所有打印机操作权限，确保有足够权限删除打印机

### 2. 改进错误处理

**OpenPrinterW 失败时**:
- 错误码 5 (ERROR_ACCESS_DENIED) 时，显示详细的错误信息：
  ```
  拒绝访问（5）。可能是：未以管理员权限运行、或被系统策略/权限限制。请确认已提升权限，并联系 IT 检查打印服务策略。
  
  打印机: {printer_name}
  错误码: 5
  ```

**DeletePrinter 失败时**:
- 同样处理错误码 5，显示详细的错误信息

### 3. 保持幂等性

- 错误码 1801 (ERROR_INVALID_PRINTER_NAME) 或 2 (ERROR_FILE_NOT_FOUND) 仍然视为幂等
- 返回 `success=true, removed_queue=false`

## 修改内容

### 文件修改

1. `src-tauri/src/platform/windows/delete.rs` - **修改**
   - `delete_printer_queue()` 函数
   - 将 `DesiredAccess` 从 `0x00000004` 改为 `0x000F000C`
   - 改进错误处理，特别是 error_code=5 的提示信息

### 代码变更

```rust
// 修改前
DesiredAccess: 0x00000004, // PRINTER_ACCESS_ADMINISTER

// 修改后
DesiredAccess: 0x000F000C, // PRINTER_ALL_ACCESS
```

## 测试验证

### Windows 测试

- [ ] **管理员启动后删除打印机**：应成功删除队列
- [ ] **非管理员启动**：预期仍失败，错误码 5，但显示详细错误信息
- [ ] **打印机不存在**：返回 success=true（幂等）

### 错误信息验证

- [ ] error_code=5 时，显示详细的错误提示（包含打印机名称和错误码）
- [ ] 其他错误码时，显示通用错误信息

## 注意事项

1. **权限要求**: 即使使用 `PRINTER_ALL_ACCESS`，删除打印机仍可能需要管理员权限
2. **系统策略**: 某些系统策略可能限制打印机删除操作
3. **不影响其他功能**: 仅修改队列删除的权限设置，不影响端口/驱动删除逻辑
4. **macOS 不受影响**: 此修复仅针对 Windows 平台

## 相关常量值

- `PRINTER_ACCESS_ADMINISTER` = 0x00000004
- `PRINTER_ALL_ACCESS` = 0x000F000C
- `ERROR_ACCESS_DENIED` = 5
- `ERROR_INVALID_PRINTER_NAME` = 1801
- `ERROR_FILE_NOT_FOUND` = 2
