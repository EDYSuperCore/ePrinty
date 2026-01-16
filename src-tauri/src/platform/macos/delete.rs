// macOS 平台打印机删除模块
// 使用 lpadmin -x 删除打印机队列

// macOS 平台打印机删除模块
// 使用 lpadmin -x 删除打印机队列

use std::process::Command;

/// 简单的日志函数（macOS 平台）
fn write_log(msg: &str) {
    eprintln!("{}", msg);
}

/// 删除打印机结果（内部使用）
#[derive(Debug)]
pub struct DeletePrinterResultInternal {
    pub success: bool,
    pub removed_queue: bool,
    pub removed_port: bool,
    pub removed_driver: bool,
    pub driver_name: Option<String>,
    pub port_name: Option<String>,
    pub message: String,
    pub evidence: Option<String>,
}

/// 检查打印机是否存在（使用 lpstat）
/// 
/// # 参数
/// - `printer_name`: 打印机队列名称
/// 
/// # 返回
/// - `Ok(bool)`: true 表示存在，false 表示不存在
fn printer_exists(printer_name: &str) -> Result<bool, String> {
    let output = Command::new("lpstat")
        .arg("-p")
        .arg(printer_name)
        .output()
        .map_err(|e| format!("执行 lpstat 失败: {}", e))?;
    
    Ok(output.status.success())
}

/// macOS 平台删除打印机入口
/// 
/// # 参数
/// - `printer_name`: 打印机队列名称
/// - `_remove_port`: macOS 不单独处理端口（CUPS 会管理），此参数被忽略
/// 
/// # 返回
/// - `Ok(DeletePrinterResultInternal)`: 删除结果
pub fn delete_printer_macos(printer_name: &str, _remove_port: bool) -> Result<DeletePrinterResultInternal, String> {
    let start_time = std::time::Instant::now();
    let call_id = format!("delete_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());
    
    write_log(&format!("[DeletePrinter][#{}] START printer_name=\"{}\"", call_id, printer_name));
    
    // 步骤 1: 预检查打印机是否存在（幂等处理）
    match printer_exists(printer_name) {
        Ok(exists) => {
            if !exists {
                write_log(&format!("[DeletePrinter][#{}] DELETE_QUEUE_SKIP printer_name=\"{}\" reason=not_found", call_id, printer_name));
                let elapsed_ms = start_time.elapsed().as_millis();
                write_log(&format!("[DeletePrinter][#{}] DONE elapsed_ms={} success=true removed_queue=false", call_id, elapsed_ms));
                return Ok(DeletePrinterResultInternal {
                    success: true,
                    removed_queue: false,
                    removed_port: false,
                    removed_driver: false,
                    driver_name: None,
                    port_name: None,
                    message: format!("打印机 \"{}\" 不存在（可能已删除）", printer_name),
                    evidence: Some("lpstat -p returned non-zero (not found)".to_string()),
                });
            }
        }
        Err(e) => {
            // 预检查失败，继续尝试删除（可能打印机存在但 lpstat 失败）
            write_log(&format!("[DeletePrinter][#{}] PRECHECK_WARN printer_name=\"{}\" error={}", call_id, printer_name, e));
        }
    }
    
    // 步骤 2: 使用 lpadmin -x 删除打印机
    log::write_log(&format!("[DeletePrinter][#{}] DELETE_QUEUE_START printer_name=\"{}\"", call_id, printer_name));
    
    let output = Command::new("lpadmin")
        .arg("-x")
        .arg(printer_name)
        .output()
        .map_err(|e| format!("执行 lpadmin 失败: {}", e))?;
    
    if output.status.success() {
        write_log(&format!("[DeletePrinter][#{}] DELETE_QUEUE_OK printer_name=\"{}\"", call_id, printer_name));
        let elapsed_ms = start_time.elapsed().as_millis();
        write_log(&format!("[DeletePrinter][#{}] DONE elapsed_ms={} success=true removed_queue=true", call_id, elapsed_ms));
        
        Ok(DeletePrinterResultInternal {
            success: true,
            removed_queue: true,
            removed_port: false, // macOS 不单独处理端口
            removed_driver: false, // macOS 不支持驱动删除
            driver_name: None,
            port_name: None,
            message: format!("已删除打印机队列: {}", printer_name),
            evidence: Some(format!("lpadmin -x ok printer_name=\"{}\"", printer_name)),
        })
    } else {
        // 检查错误输出，判断是否为"不存在"错误（幂等处理）
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // macOS lpadmin 的错误信息可能包含 "Unknown printer" 或 "does not exist" 等
        let not_found_keywords = ["Unknown printer", "does not exist", "not found", "No such file"];
        let is_not_found = not_found_keywords.iter().any(|keyword| {
            stderr.contains(keyword) || stdout.contains(keyword)
        });
        
        if is_not_found {
            write_log(&format!("[DeletePrinter][#{}] DELETE_QUEUE_SKIP printer_name=\"{}\" reason=not_found stderr={}", call_id, printer_name, stderr.chars().take(100).collect::<String>()));
            let elapsed_ms = start_time.elapsed().as_millis();
            write_log(&format!("[DeletePrinter][#{}] DONE elapsed_ms={} success=true removed_queue=false", call_id, elapsed_ms));
            
            Ok(DeletePrinterResultInternal {
                success: true,
                removed_queue: false,
                removed_port: false,
                removed_driver: false,
                driver_name: None,
                port_name: None,
                message: format!("打印机 \"{}\" 不存在（可能已删除）", printer_name),
                evidence: Some(format!("lpadmin -x failed (not found): stderr={}", stderr.chars().take(200).collect::<String>())),
            })
        } else {
            let error_msg = if !stderr.is_empty() {
                format!("lpadmin 执行失败: {}", stderr.chars().take(200).collect::<String>())
            } else {
                format!("lpadmin 执行失败: exit_code={}", output.status.code().unwrap_or(-1))
            };
            
            write_log(&format!("[DeletePrinter][#{}] DELETE_QUEUE_FAIL printer_name=\"{}\" {}", call_id, printer_name, error_msg));
            let elapsed_ms = start_time.elapsed().as_millis();
            write_log(&format!("[DeletePrinter][#{}] FAILED elapsed_ms={} error={}", call_id, elapsed_ms, error_msg));
            
            Err(error_msg)
        }
    }
}
