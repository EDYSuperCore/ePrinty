use std::process::Command;

/// macOS 平台打开 URL
/// 
/// 使用 `open` 命令打开 URL
pub fn open_url_macos(url: &str) -> Result<(), String> {
    let output = Command::new("open")
        .arg(url)
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;
    
    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("无法打开 URL: {}", error))
    }
}

/// macOS 平台获取打印机列表
/// 
/// 实现逻辑：
/// 1. 首先尝试使用 `lpstat -p` 获取打印机列表
/// 2. 如果失败，尝试使用 `lpstat -a` 获取可用打印机
/// 3. 如果仍为空，尝试使用 `lpstat -d` 获取默认打印机
pub fn list_printers_macos() -> Result<Vec<String>, String> {
    let mut printers = Vec::new();
    
    // 方法1: 尝试使用 lpstat -p 获取打印机列表
    // lpstat -p 输出格式: "printer PrinterName is idle.  enabled since ..."
    // 或 "printer PrinterName is idle.  enabled since ..."
    match Command::new("lpstat").arg("-p").output() {
        Ok(output) => {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    // 解析格式: "printer PrinterName is ..."
                    if line.starts_with("printer ") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            let printer_name = parts[1].to_string();
                            if !printer_name.is_empty() && !printers.contains(&printer_name) {
                                printers.push(printer_name);
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {
            // lpstat -p 命令执行失败，继续尝试其他方法
        }
    }
    
    // 方法2: 如果方法1没有结果，尝试使用 lpstat -a 获取可用打印机
    // lpstat -a 输出格式: "PrinterName accepting requests since ..."
    if printers.is_empty() {
        match Command::new("lpstat").arg("-a").output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        // 解析格式: "PrinterName accepting requests since ..."
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if !parts.is_empty() && parts[0] != "printer" {
                            let printer_name = parts[0].to_string();
                            if !printer_name.is_empty() && !printers.contains(&printer_name) {
                                printers.push(printer_name);
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // lpstat -a 命令执行失败，继续尝试其他方法
            }
        }
    }
    
    // 方法3: 如果前两种方法都没有结果，尝试使用 lpstat -d 获取默认打印机
    // lpstat -d 输出格式: "system default destination: PrinterName"
    if printers.is_empty() {
        match Command::new("lpstat").arg("-d").output() {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        // 解析格式: "system default destination: PrinterName"
                        if line.contains("system default destination:") {
                            if let Some(colon_pos) = line.find(':') {
                                let printer_name = line[colon_pos + 1..].trim().to_string();
                                if !printer_name.is_empty() && !printers.contains(&printer_name) {
                                    printers.push(printer_name);
                                }
                            }
                        }
                    }
                }
            }
            Err(_) => {
                // lpstat -d 命令执行失败
            }
        }
    }
    
    if printers.is_empty() {
        Err("未找到可用的打印机".to_string())
    } else {
        Ok(printers)
    }
}

