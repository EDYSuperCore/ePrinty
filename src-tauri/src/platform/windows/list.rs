// Windows 平台打印机列表获取模块

/// 获取 Windows 系统已安装的打印机列表
/// 
/// 使用 PowerShell Get-Printer 命令获取打印机列表，并通过 ConvertTo-Json 解析结果
/// 返回打印机名称的向量
pub fn list_printers_windows() -> Result<Vec<String>, String> {
    // 使用 PowerShell 执行 Get-Printer 命令并转换为 JSON
    // 使用 @() 确保始终返回数组格式，即使只有一个打印机
    let script = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; @(Get-Printer) | ConvertTo-Json -Depth 3";
    let output = super::ps::run_powershell(script)?;

    // 解码输出（处理中文编码问题）
    let stdout = super::encoding::decode_windows_string(&output.stdout);
    let stderr = super::encoding::decode_windows_string(&output.stderr);

    // 检查命令是否执行成功
    if !output.status.success() {
        return Err(format!(
            "获取打印机列表失败: {}\n错误输出: {}",
            stdout, stderr
        ));
    }

    // 解析 JSON 输出
    // PowerShell 的 ConvertTo-Json 可能返回单个对象（如果只有一个打印机）或数组（如果有多个）
    let printers: Vec<String> = if stdout.trim().is_empty() {
        // 如果没有输出，返回空列表
        Vec::new()
    } else {
        // 尝试解析为 JSON
        match serde_json::from_str::<serde_json::Value>(&stdout) {
            Ok(json_value) => {
                match json_value {
                    serde_json::Value::Array(arr) => {
                        // 如果是数组，提取每个对象的 Name 字段
                        arr.into_iter()
                            .filter_map(|item| {
                                if let serde_json::Value::Object(obj) = item {
                                    obj.get("Name")
                                        .and_then(|name| name.as_str())
                                        .map(|s| s.to_string())
                                } else {
                                    None
                                }
                            })
                            .collect()
                    }
                    serde_json::Value::Object(obj) => {
                        // 如果是单个对象，提取 Name 字段
                        obj.get("Name")
                            .and_then(|name| name.as_str())
                            .map(|s| vec![s.to_string()])
                            .unwrap_or_default()
                    }
                    _ => {
                        return Err(format!("无法解析 PowerShell 输出为有效的 JSON: {}", stdout));
                    }
                }
            }
            Err(e) => {
                // 如果 JSON 解析失败，尝试按行解析（PowerShell 可能输出多行）
                // 或者尝试直接提取打印机名称
                let lines: Vec<String> = stdout
                    .lines()
                    .filter_map(|line| {
                        let trimmed = line.trim();
                        // 尝试从 JSON 行中提取 Name 字段
                        if trimmed.contains("\"Name\"") {
                            // 简单的正则匹配或字符串提取
                            if let Some(start) = trimmed.find("\"Name\"") {
                                let after_name = &trimmed[start + 6..];
                                if let Some(colon) = after_name.find(':') {
                                    let after_colon = &after_name[colon + 1..];
                                    let name_part = after_colon.trim();
                                    // 提取引号内的内容
                                    if let Some(quote_start) = name_part.find('"') {
                                        let after_quote = &name_part[quote_start + 1..];
                                        if let Some(quote_end) = after_quote.find('"') {
                                            return Some(after_quote[..quote_end].to_string());
                                        }
                                    }
                                }
                            }
                        }
                        None
                    })
                    .collect();

                if !lines.is_empty() {
                    lines
                } else {
                    return Err(format!(
                        "解析 PowerShell 输出失败: {}\n原始输出: {}",
                        e, stdout
                    ));
                }
            }
        }
    };

    Ok(printers)
}

