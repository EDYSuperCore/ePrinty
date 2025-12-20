# EasyPrinter Windows 安装驱动选择 - 回归测试清单

## 测试环境要求
- Windows 10/11 系统
- 管理员权限
- 已安装至少一个打印机驱动（用于测试）
- 可访问的打印机 IP 地址

## 测试用例（>=8条）

### 1. 配置缺少 driver_names 字段
**测试步骤：**
1. 修改 `printer_config.json`，移除某个打印机的 `driver_names` 字段
2. 尝试安装该打印机

**预期结果：**
- 安装失败
- 错误信息包含："配置缺少 driver_names，请更新 printer_config.json"
- `InstallResult.success = false`
- `InstallResult.stderr` 包含错误码 `WIN_INVALID_CONFIG`

---

### 2. driver_names 为空数组
**测试步骤：**
1. 修改 `printer_config.json`，将某个打印机的 `driver_names` 设置为 `[]`
2. 尝试安装该打印机

**预期结果：**
- 安装失败
- 错误信息包含："配置缺少 driver_names，请更新 printer_config.json"
- `InstallResult.success = false`
- `InstallResult.stderr` 包含错误码 `WIN_INVALID_CONFIG`

---

### 3. driver_names 元素全为空白
**测试步骤：**
1. 修改 `printer_config.json`，将某个打印机的 `driver_names` 设置为 `["  ", "\t", ""]`
2. 尝试安装该打印机

**预期结果：**
- 安装失败
- 错误信息包含："driver_names 不能为空"
- `InstallResult.success = false`
- `InstallResult.stderr` 包含错误码 `WIN_INVALID_CONFIG`

---

### 4. 候选驱动均未安装
**测试步骤：**
1. 修改 `printer_config.json`，将某个打印机的 `driver_names` 设置为系统中不存在的驱动名，如 `["NonExistentDriver1", "NonExistentDriver2"]`
2. 尝试安装该打印机

**预期结果：**
- 安装失败
- 错误信息包含："系统中没有可用的打印机驱动。请先安装打印机驱动。"
- `InstallResult.success = false`
- `InstallResult.stderr` 包含：
  - 错误码 `WIN_DRIVER_NOT_FOUND`
  - `Candidates: NonExistentDriver1,NonExistentDriver2`
  - 如果 PowerShell 有错误输出，应包含 `PowerShell stderr: ...`

---

### 5. 第二个候选驱动命中
**测试步骤：**
1. 确认系统中已安装驱动 "DriverB"，未安装 "DriverA"
2. 修改 `printer_config.json`，将某个打印机的 `driver_names` 设置为 `["DriverA", "DriverB"]`
3. 尝试安装该打印机

**预期结果：**
- 安装成功或进入端口创建阶段
- 使用 "DriverB" 作为选中的驱动
- 调试日志显示："[DEBUG] 找到已安装的驱动: DriverB"

---

### 6. 驱动名包含单引号
**测试步骤：**
1. 如果系统中有驱动名包含单引号（如 "HP's Driver"），修改 `printer_config.json` 使用该驱动名
2. 尝试安装该打印机

**预期结果：**
- PowerShell 查询能正确处理单引号转义（单引号被转义为两个单引号）
- 安装成功或正确识别驱动

---

### 7. Add-Printer 失败（权限不足）
**测试步骤：**
1. 使用非管理员权限运行应用
2. 配置正确的 `driver_names`（系统中已安装的驱动）
3. 尝试安装打印机

**预期结果：**
- 端口创建可能成功，但 Add-Printer 失败
- 错误信息包含 PowerShell stderr
- `InstallResult.stderr` 包含：
  - 错误码 `WIN_PRINTER_INSTALL_FAILED`
  - `Driver used: <驱动名>`
  - `Port: <端口名>`
  - PowerShell 的具体错误信息

---

### 8. 端口不存在（端口创建失败）
**测试步骤：**
1. 配置正确的 `driver_names`
2. 使用无效的 IP 地址或无法访问的 IP（如 `\\192.168.255.255`）
3. 尝试安装打印机

**预期结果：**
- 端口创建失败
- 错误信息包含端口创建失败的具体原因
- `InstallResult.stderr` 包含错误码 `WIN_PORT_FAILED`
- 如果使用现代方式，错误信息包含 stdout 和 stderr

---

### 9. 正常安装流程（所有条件满足）
**测试步骤：**
1. 配置正确的 `driver_names`（系统中已安装的驱动）
2. 使用有效的打印机 IP 地址
3. 以管理员权限运行应用
4. 尝试安装打印机

**预期结果：**
- 安装成功
- `InstallResult.success = true`
- `InstallResult.message` 包含："打印机 <名称> (<IP>) 安装成功"
- `InstallResult.method` 为 "Add-Printer" 或 "VBS"

---

### 10. 候选列表 trim 和过滤空字符串
**测试步骤：**
1. 修改 `printer_config.json`，将 `driver_names` 设置为 `["  DriverA  ", "", "  ", "DriverB"]`
2. 确认系统中已安装 "DriverA" 或 "DriverB"
3. 尝试安装该打印机

**预期结果：**
- 正确 trim 并过滤空字符串
- 使用 "DriverA" 或 "DriverB"（如果已安装）
- 不会因为空白字符串导致错误

---

## 验证点检查清单

### 错误处理验证
- [ ] `driver_names` 缺失时返回 `InstallError::InvalidConfig`
- [ ] `driver_names` 为空数组时返回 `InstallError::InvalidConfig`
- [ ] `driver_names` 全空白时返回 `InstallError::InvalidConfig`
- [ ] 所有候选未命中时返回 `InstallError::DriverNotFound`
- [ ] 错误信息包含候选列表（`Candidates: ...`）
- [ ] 错误信息包含 PowerShell stderr（如果存在）

### PowerShell 执行验证
- [ ] 使用 `-NoProfile -NonInteractive -WindowStyle Hidden` 参数
- [ ] 驱动名查询使用精确匹配（`-Name '<name>'`）
- [ ] 驱动名中的单引号正确转义（`'` -> `''`）

### 诊断信息验证
- [ ] 失败时 `stderr` 包含使用的驱动名（`Driver used: ...`）
- [ ] 失败时 `stderr` 包含端口名（`Port: ...`）
- [ ] 失败时 `stderr` 包含 PowerShell 错误输出
- [ ] 调试日志输出驱动选择过程

### 功能验证
- [ ] 候选列表正确 trim 和过滤
- [ ] 按顺序检查候选，返回第一个已安装的驱动
- [ ] 所有调用点都传入 `selected_driver`
- [ ] 不再使用旧的驱动搜索逻辑（无兜底）

---

## 测试数据准备

### 测试用 printer_config.json 示例

```json
{
  "version": "1.1.0",
  "areas": [
    {
      "name": "测试区域",
      "printers": [
        {
          "name": "测试打印机1",
          "path": "\\\\192.168.1.100",
          "model": "Test Model",
          "driver_names": ["TestDriver1", "TestDriver2"],
          "driver_path": "drivers/test.inf"
        }
      ]
    }
  ]
}
```

---

## 注意事项

1. **权限要求**：部分测试需要管理员权限
2. **驱动准备**：确保测试环境中有至少一个已安装的打印机驱动
3. **网络要求**：端口创建测试需要有效的网络连接
4. **日志查看**：测试时查看控制台输出，确认调试日志正确
5. **错误信息完整性**：验证所有错误场景的错误信息都包含足够的诊断信息

