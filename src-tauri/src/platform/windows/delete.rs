// Windows 平台打印机删除模块
// 使用 Win32 Print Spooler API 删除打印机队列和端口

use super::log;
use super::list::list_printers_detailed;
use super::DetailedPrinterInfo;
use winapi::um::winspool::{OpenPrinterW, DeletePrinter, ClosePrinter, PRINTER_DEFAULTSW, GetPrinterW, PRINTER_INFO_2W};
use winapi::um::winnt::LPWSTR;
use winapi::um::errhandlingapi::GetLastError;
use winapi::ctypes::c_void;
use std::ptr;

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

/// 将 Rust String 转换为 UTF-16 宽字符串（以 null 结尾）
fn string_to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 删除打印机队列（使用 Win32 API DeletePrinter）
/// 
/// # 参数
/// - `printer_name`: 打印机队列名称
/// 
/// # 返回
/// - `Ok(DeletePrinterResultInternal)`: 删除结果
fn delete_printer_queue(printer_name: &str) -> Result<DeletePrinterResultInternal, String> {
    log::write_log(&format!("[DeletePrinter] DELETE_QUEUE_START printer_name=\"{}\"", printer_name));
    
    unsafe {
        // 将打印机名称转换为 UTF-16 宽字符串
        let printer_name_wide = string_to_wide(printer_name);
        
        // 打开打印机（需要删除权限）
        // 使用 PRINTER_ALL_ACCESS (0x000F000C) 确保有足够权限删除打印机
        // 如果 PRINTER_ACCESS_ADMINISTER (0x00000004) 不够，PRINTER_ALL_ACCESS 应该足够
        let mut printer_handle: *mut c_void = ptr::null_mut();
        let mut defaults: PRINTER_DEFAULTSW = PRINTER_DEFAULTSW {
            pDataType: ptr::null_mut(),
            pDevMode: ptr::null_mut(),
            DesiredAccess: 0x000F000C, // PRINTER_ALL_ACCESS (包含删除权限)
        };
        
        let open_result = OpenPrinterW(
            printer_name_wide.as_ptr() as LPWSTR,
            &mut printer_handle,
            &mut defaults,
        );
        
        if open_result == 0 {
            let error_code = GetLastError();
            
            // 错误码 1801 (ERROR_INVALID_PRINTER_NAME) 或 2 (ERROR_FILE_NOT_FOUND) 表示打印机不存在
            // 视为幂等：已删除/无需删除
            if error_code == 1801 || error_code == 2 {
                log::write_log(&format!("[DeletePrinter] DELETE_QUEUE_SKIP printer_name=\"{}\" reason=not_found error_code={}", printer_name, error_code));
                return Ok(DeletePrinterResultInternal {
                    success: true,
                    removed_queue: false,
                    removed_port: false,
                    removed_driver: false,
                    driver_name: None,
                    port_name: None,
                    message: format!("打印机 \"{}\" 不存在（可能已删除）", printer_name),
                    evidence: Some(format!("OpenPrinterW failed: error_code={} (not found)", error_code)),
                });
            }
            
            // 错误码 5 (ERROR_ACCESS_DENIED) 表示权限不足
            let error_msg = if error_code == 5 {
                format!(
                    "拒绝访问（5）。可能是：未以管理员权限运行、或被系统策略/权限限制。请确认已提升权限，并联系 IT 检查打印服务策略。\n\n打印机: {}\n错误码: {}",
                    printer_name, error_code
                )
            } else {
                format!("打开打印机失败: error_code={}", error_code)
            };
            
            log::write_log(&format!("[DeletePrinter] DELETE_QUEUE_FAIL printer_name=\"{}\" step=OpenPrinterW error_code={}", printer_name, error_code));
            return Err(error_msg);
        }
        
        // 调用 DeletePrinter 删除打印机
        let delete_result = DeletePrinter(printer_handle);
        
        // 关闭句柄（无论删除是否成功）
        let _ = ClosePrinter(printer_handle);
        
        if delete_result == 0 {
            let error_code = GetLastError();
            
            // 错误码 5 (ERROR_ACCESS_DENIED) 表示权限不足
            let error_msg = if error_code == 5 {
                format!(
                    "拒绝访问（5）。可能是：未以管理员权限运行、或被系统策略/权限限制。请确认已提升权限，并联系 IT 检查打印服务策略。\n\n打印机: {}\n错误码: {}",
                    printer_name, error_code
                )
            } else {
                format!("删除打印机失败: error_code={}", error_code)
            };
            
            log::write_log(&format!("[DeletePrinter] DELETE_QUEUE_FAIL printer_name=\"{}\" step=DeletePrinter error_code={}", printer_name, error_code));
            return Err(error_msg);
        }
        
        log::write_log(&format!("[DeletePrinter] DELETE_QUEUE_OK printer_name=\"{}\"", printer_name));
        Ok(DeletePrinterResultInternal {
            success: true,
            removed_queue: true,
            removed_port: false,
            removed_driver: false,
            driver_name: None,
            port_name: None,
            message: format!("已删除打印机队列: {}", printer_name),
            evidence: Some(format!("DeletePrinter ok printer_name=\"{}\"", printer_name)),
        })
    }
}

/// 删除打印机端口（尽力而为，失败不影响队列删除）
/// 
/// # 参数
/// - `port_name`: 端口名称
/// 
/// # 返回
/// - `Ok(bool)`: true 表示删除成功，false 表示未删除（不存在或失败）
fn delete_printer_port(port_name: &str) -> Result<bool, String> {
    log::write_log(&format!("[DeletePrinter] DELETE_PORT_START port_name=\"{}\"", port_name));
    
    // 使用 PowerShell Remove-PrinterPort 删除端口
    // 注意：这是"尽力而为"的实现，失败不影响队列删除的成功
    use super::cmd;
    
    let ps_command = format!(
        "Remove-PrinterPort -Name '{}' -ErrorAction SilentlyContinue; if ($?) {{ Write-Output 'SUCCESS' }} else {{ Write-Output 'FAILED' }}",
        port_name.replace("'", "''") // 转义单引号
    );
    
    match cmd::run_command("powershell.exe", &[
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &ps_command
    ]) {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            
            if output.status.success() && stdout.contains("SUCCESS") {
                log::write_log(&format!("[DeletePrinter] DELETE_PORT_OK port_name=\"{}\"", port_name));
                Ok(true)
            } else {
                // 端口删除失败，但不影响整体成功
                let evidence = if !stderr.is_empty() {
                    format!("Remove-PrinterPort failed: stderr={}", stderr.chars().take(200).collect::<String>())
                } else {
                    format!("Remove-PrinterPort failed: exit_code={} stdout={}", 
                        output.status.code().unwrap_or(-1),
                        stdout.chars().take(200).collect::<String>())
                };
                log::write_log(&format!("[DeletePrinter] DELETE_PORT_FAIL port_name=\"{}\" {}", port_name, evidence));
                Ok(false)
            }
        }
        Err(e) => {
            let evidence = format!("Remove-PrinterPort exec failed: {}", e);
            log::write_log(&format!("[DeletePrinter] DELETE_PORT_FAIL port_name=\"{}\" {}", port_name, evidence));
            Ok(false) // 失败但不影响整体
        }
    }
}

/// 获取打印机的端口名称和驱动名称
/// 
/// # 参数
/// - `printer_name`: 打印机队列名称
/// 
/// # 返回
/// - `Ok((Option<String>, Option<String>))`: (端口名称, 驱动名称)
fn get_printer_info(printer_name: &str) -> Result<(Option<String>, Option<String>), String> {
    // 使用 list_printers_detailed 获取打印机信息
    match list_printers_detailed() {
        Ok(printers) => {
            for printer in printers {
                if printer.name == printer_name {
                    return Ok((printer.port_name, printer.driver_name));
                }
            }
            Ok((None, None))
        }
        Err(e) => {
            log::write_log(&format!("[DeletePrinter] GET_INFO_FAIL printer_name=\"{}\" error={}", printer_name, e));
            Err(format!("获取打印机信息失败: {}", e))
        }
    }
}

/// 检查驱动是否被其他打印机队列使用
/// 
/// # 参数
/// - `driver_name`: 驱动名称
/// - `exclude_printer`: 排除的打印机名称（当前要删除的）
/// 
/// # 返回
/// - `Ok(usize)`: 使用该驱动的队列数量（不包括 exclude_printer）
fn count_driver_usage(driver_name: &str, exclude_printer: &str) -> Result<usize, String> {
    match list_printers_detailed() {
        Ok(printers) => {
            let count = printers.iter()
                .filter(|p| {
                    p.name != exclude_printer && 
                    p.driver_name.as_ref().map(|d| d == driver_name).unwrap_or(false)
                })
                .count();
            Ok(count)
        }
        Err(e) => {
            log::write_log(&format!("[DeletePrinter] COUNT_DRIVER_USAGE_FAIL driver_name=\"{}\" error={}", driver_name, e));
            Err(format!("检查驱动使用情况失败: {}", e))
        }
    }
}

/// 删除打印机驱动（安全删除：仅当没有其他队列使用时）
/// 
/// # 参数
/// - `driver_name`: 驱动名称
/// 
/// # 返回
/// - `Ok(bool)`: true 表示删除成功，false 表示未删除（失败或不应删除）
fn delete_printer_driver(driver_name: &str) -> Result<bool, String> {
    log::write_log(&format!("[DeletePrinter] DELETE_DRIVER_START driver_name=\"{}\"", driver_name));
    
    // 使用 PowerShell Remove-PrinterDriver 删除驱动
    // 注意：这是"尽力而为"的实现，失败不影响队列删除的成功
    use super::cmd;
    
    let ps_command = format!(
        "Remove-PrinterDriver -Name '{}' -ErrorAction SilentlyContinue; if ($?) {{ Write-Output 'SUCCESS' }} else {{ Write-Output 'FAILED' }}",
        driver_name.replace("'", "''") // 转义单引号
    );
    
    match cmd::run_command("powershell.exe", &[
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        &ps_command
    ]) {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            
            if output.status.success() && stdout.contains("SUCCESS") {
                log::write_log(&format!("[DeletePrinter] DELETE_DRIVER_OK driver_name=\"{}\"", driver_name));
                Ok(true)
            } else {
                // 驱动删除失败，但不影响整体成功
                let evidence = if !stderr.is_empty() {
                    format!("Remove-PrinterDriver failed: stderr={}", stderr.chars().take(200).collect::<String>())
                } else {
                    format!("Remove-PrinterDriver failed: exit_code={} stdout={}", 
                        output.status.code().unwrap_or(-1),
                        stdout.chars().take(200).collect::<String>())
                };
                log::write_log(&format!("[DeletePrinter] DELETE_DRIVER_FAIL driver_name=\"{}\" {}", driver_name, evidence));
                Ok(false)
            }
        }
        Err(e) => {
            let evidence = format!("Remove-PrinterDriver exec failed: {}", e);
            log::write_log(&format!("[DeletePrinter] DELETE_DRIVER_FAIL driver_name=\"{}\" {}", driver_name, evidence));
            Ok(false) // 失败但不影响整体
        }
    }
}

/// Windows 平台删除打印机入口
/// 
/// # 参数
/// - `printer_name`: 打印机队列名称
/// - `remove_port`: 是否删除端口（仅删除 IP_ 前缀端口）
/// - `remove_driver`: 是否删除驱动（仅当没有其他队列使用时）
/// 
/// # 返回
/// - `Ok(DeletePrinterResultInternal)`: 删除结果
pub fn delete_printer_windows(printer_name: &str, remove_port: bool, remove_driver: bool) -> Result<DeletePrinterResultInternal, String> {
    let start_time = std::time::Instant::now();
    let call_id = format!("delete_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());
    
    log::write_log(&format!("[DeletePrinter][#{}] START printer_name=\"{}\" remove_port={} remove_driver={}", 
        call_id, printer_name, remove_port, remove_driver));
    
    // 步骤 1: 如果需要删除端口或驱动，先获取打印机信息（删除队列后无法获取）
    let (port_name_opt, driver_name_opt) = if remove_port || remove_driver {
        match get_printer_info(printer_name) {
            Ok((port_name, driver_name)) => {
                let port = if remove_port {
                    if let Some(ref p) = port_name {
                        if p.starts_with("IP_") {
                            Some(p.clone())
                        } else {
                            log::write_log(&format!("[DeletePrinter][#{}] DELETE_PORT_SKIP port_name=\"{}\" reason=not_ip_prefix", call_id, p));
                            None
                        }
                    } else {
                        log::write_log(&format!("[DeletePrinter][#{}] DELETE_PORT_SKIP printer_name=\"{}\" reason=port_not_found", call_id, printer_name));
                        None
                    }
                } else {
                    None
                };
                (port, driver_name)
            }
            Err(e) => {
                // 获取信息失败不影响队列删除
                log::write_log(&format!("[DeletePrinter][#{}] GET_INFO_ERROR printer_name=\"{}\" error={}", call_id, printer_name, e));
                (None, None)
            }
        }
    } else {
        (None, None)
    };
    
    // 步骤 2: 删除打印机队列（必须成功）
    let mut result = match delete_printer_queue(printer_name) {
        Ok(mut r) => {
            // 设置获取到的端口和驱动名称
            r.port_name = port_name_opt.clone();
            r.driver_name = driver_name_opt.clone();
            r
        }
        Err(e) => {
            let elapsed_ms = start_time.elapsed().as_millis();
            log::write_log(&format!("[DeletePrinter][#{}] FAILED elapsed_ms={} error={}", call_id, elapsed_ms, e));
            return Err(e);
        }
    };
    
    // 步骤 3: 如果队列删除成功且需要删除端口，尝试删除端口
    if result.removed_queue && remove_port {
        if let Some(port_name) = &port_name_opt {
            match delete_printer_port(port_name) {
                Ok(port_removed) => {
                    result.removed_port = port_removed;
                    if port_removed {
                        result.message.push_str(&format!("，已删除端口: {}", port_name));
                    } else {
                        result.message.push_str(&format!("（端口删除失败: {}）", port_name));
                    }
                }
                Err(e) => {
                    // 端口删除失败不影响整体成功
                    let evidence = format!("端口删除失败: {}", e);
                    result.evidence = Some(format!("{}; {}", result.evidence.as_deref().unwrap_or(""), evidence));
                    log::write_log(&format!("[DeletePrinter][#{}] DELETE_PORT_ERROR port_name=\"{}\" error={}", call_id, port_name, e));
                }
            }
        }
    }
    
    // 步骤 4: 如果队列删除成功且需要删除驱动，尝试删除驱动（安全删除）
    if result.removed_queue && remove_driver {
        if let Some(driver_name) = &driver_name_opt {
            log::write_log(&format!("[DeletePrinter][#{}] CHECK_DRIVER_USAGE driver_name=\"{}\"", call_id, driver_name));
            
            // 检查驱动是否被其他队列使用
            match count_driver_usage(driver_name, printer_name) {
                Ok(count) => {
                    if count > 0 {
                        // 有其他队列使用该驱动，拒绝删除
                        log::write_log(&format!("[DeletePrinter][#{}] DELETE_DRIVER_SKIP driver_name=\"{}\" reason=used_by_other_queues count={}", call_id, driver_name, count));
                        result.message.push_str(&format!("（该驱动仍被其他 {} 个打印机使用，已跳过删除驱动）", count));
                        result.removed_driver = false;
                    } else {
                        // 没有其他队列使用，尝试删除
                        match delete_printer_driver(driver_name) {
                            Ok(driver_removed) => {
                                result.removed_driver = driver_removed;
                                if driver_removed {
                                    result.message.push_str(&format!("，已删除驱动: {}", driver_name));
                                } else {
                                    result.message.push_str(&format!("（驱动删除失败: {}）", driver_name));
                                }
                            }
                            Err(e) => {
                                // 驱动删除失败不影响整体成功
                                let evidence = format!("驱动删除失败: {}", e);
                                result.evidence = Some(format!("{}; {}", result.evidence.as_deref().unwrap_or(""), evidence));
                                log::write_log(&format!("[DeletePrinter][#{}] DELETE_DRIVER_ERROR driver_name=\"{}\" error={}", call_id, driver_name, e));
                            }
                        }
                    }
                }
                Err(e) => {
                    // 检查失败，为安全起见不删除驱动
                    log::write_log(&format!("[DeletePrinter][#{}] CHECK_DRIVER_USAGE_ERROR driver_name=\"{}\" error={}", call_id, driver_name, e));
                    result.message.push_str(&format!("（无法检查驱动使用情况，已跳过删除驱动）"));
                    result.removed_driver = false;
                }
            }
        } else {
            log::write_log(&format!("[DeletePrinter][#{}] DELETE_DRIVER_SKIP printer_name=\"{}\" reason=driver_not_found", call_id, printer_name));
            result.message.push_str("（未找到驱动信息，已跳过删除驱动）");
            result.removed_driver = false;
        }
    }
    
    let elapsed_ms = start_time.elapsed().as_millis();
    log::write_log(&format!("[DeletePrinter][#{}] DONE elapsed_ms={} success={} removed_queue={} removed_port={} removed_driver={}", 
        call_id, elapsed_ms, result.success, result.removed_queue, result.removed_port, result.removed_driver));
    
    Ok(result)
}
