// Windows 平台打印机列表获取模块

/// 获取 Windows 系统已安装的打印机列表
/// 
/// 使用 PowerShell Get-Printer 命令获取打印机名称列表（纯文本行输出）
/// 返回打印机名称的向量
/// 
/// # 实现说明
/// - 使用 `Get-Printer | Select-Object -ExpandProperty Name` 只输出名称，避免深序列化阻塞
/// - 使用超时机制（5000ms）防止 Ricoh 等打印机驱动导致的卡死
/// - 按行解析 stdout，trim 并过滤空行
pub fn list_printers_windows() -> Result<Vec<String>, String> {
    // 使用 PowerShell 执行 Get-Printer 命令，只输出 Name 字段的纯文本行
    // 不再使用 ConvertTo-Json，避免深序列化触发驱动/Spooler 阻塞
    let script = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-Printer | Select-Object -ExpandProperty Name";
    
    // 使用带超时的 PowerShell 执行（5000ms 超时）
    // 防止 Ricoh 等打印机驱动导致的卡死
    let timeout_ms = 5000u64;
    let output = match super::ps::run_powershell_with_timeout(script, timeout_ms) {
        Ok(output) => output,
        Err(e) => {
            // 超时或执行失败，返回明确错误码
            if e.contains("WIN_LIST_PRINTERS_TIMEOUT") {
                return Err(format!("WIN_LIST_PRINTERS_TIMEOUT: {}", e));
            }
            return Err(format!("WIN_LIST_PRINTERS_FAILED: {}", e));
        }
    };

    // 解码输出（处理中文编码问题）
    let stdout = super::encoding::decode_windows_string(&output.stdout);
    let stderr = super::encoding::decode_windows_string(&output.stderr);

    // 检查命令是否执行成功
    if !output.status.success() {
        return Err(format!(
            "WIN_LIST_PRINTERS_FAILED: 获取打印机列表失败: {}\n错误输出: {}",
            stdout, stderr
        ));
    }

    // 按行解析 stdout，trim 并过滤空行
    let printers: Vec<String> = stdout
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    Ok(printers)
}

