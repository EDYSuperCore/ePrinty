// Windows 平台打印机列表获取模块

use std::sync::atomic::{AtomicU64, Ordering};
use serde::{Serialize, Deserialize};

// 全局调用计数器，用于区分多次并发调用
static CALL_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 获取 Windows 系统已安装的打印机列表
/// 
/// 使用 Win32 API EnumPrintersW 直接枚举打印机，避免 PowerShell 子进程冷启动延迟
/// 返回打印机名称的向量（兼容现有前端接口）
/// 
/// # 实现说明
/// - 使用 EnumPrintersW API 直接调用，秒级返回
/// - 不再依赖 PowerShell 子进程，避免 20+ 秒冷启动延迟
/// - 使用 PRINTER_INFO_2W 获取完整信息（名称、端口、驱动）
pub fn list_printers_windows() -> Result<Vec<String>, String> {
    // 生成唯一调用 ID
    let call_id = CALL_COUNTER.fetch_add(1, Ordering::SeqCst);
    let start_time = std::time::Instant::now();
    let start_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    
    // 记录到文件日志
    super::log::write_log(&format!("[Backend][PrinterDetect][#{}] START timestamp={} method=EnumPrintersW", call_id, start_timestamp));
    
    eprintln!("[PrinterDetect][Backend] CALL_START call_id={} timestamp={} method=EnumPrintersW", call_id, start_timestamp);
    
    // 使用 Win32 API EnumPrintersW 枚举打印机
    let printer_infos = match super::enum_printers::enum_printers_w() {
        Ok(infos) => infos,
        Err(e) => {
            let elapsed_ms = start_time.elapsed().as_millis();
            
            // 记录到文件日志
            super::log::write_log(&format!("[Backend][PrinterDetect][#{}] ERROR message={} cost={}ms", call_id, e, elapsed_ms));
            
            eprintln!("[PrinterDetect][Backend] CALL_FAILED call_id={} elapsed_ms={} error={}", call_id, elapsed_ms, e);
            return Err(format!("WIN_LIST_PRINTERS_FAILED: {}", e));
        }
    };
    
    // 提取打印机名称列表（兼容现有前端接口）
    let printers: Vec<String> = printer_infos.iter()
        .map(|info| info.name.clone())
        .collect();
    
    let elapsed_ms = start_time.elapsed().as_millis();
    let printers_count = printers.len();
    
    // 记录到文件日志
    super::log::write_log(&format!("[Backend][PrinterDetect][#{}] SUCCESS cost={}ms printers_count={}", 
        call_id, elapsed_ms, printers_count));
    
    eprintln!("[PrinterDetect][Backend] CALL_SUCCESS call_id={} elapsed_ms={} printers_count={}", 
        call_id, elapsed_ms, printers_count);

    Ok(printers)
}

/// 获取 Windows 系统已安装的打印机详细列表（包含 comment 和 location）
/// 
/// 使用 Win32 API EnumPrintersW 枚举名称，然后对每个打印机使用 GetPrinterW(Level=2) 获取完整信息
pub fn list_printers_detailed() -> Result<Vec<super::DetailedPrinterInfo>, String> {
    
    // 生成唯一调用 ID
    let call_id = CALL_COUNTER.fetch_add(1, Ordering::SeqCst);
    let start_time = std::time::Instant::now();
    
    super::log::write_log(&format!("[ListPrintersDetailed] START call_id={}", call_id));
    
    // 第一步：使用 EnumPrintersW 获取所有打印机名称
    let printer_names = match list_printers_windows() {
        Ok(names) => names,
        Err(e) => {
            super::log::write_log(&format!("[ListPrintersDetailed] EnumPrintersW failed: {}", e));
            return Err(format!("枚举打印机失败: {}", e));
        }
    };
    
    // 第二步：对每个打印机使用 GetPrinterW(Level=2) 获取完整信息
    let mut detailed_printers = Vec::new();
    
    for printer_name in &printer_names {
        match get_printer_info_level_2(printer_name) {
            Ok(info) => {
                detailed_printers.push(info);
            }
            Err(e) => {
                // 获取单个打印机信息失败，记录日志但继续处理其他打印机
                super::log::write_log(&format!("[ListPrintersDetailed] GetPrinterW failed for {}: {}", printer_name, e));
            }
        }
    }
    
    // 打印前 10 条详细信息到日志
    let print_count = std::cmp::min(10, detailed_printers.len());
    for i in 0..print_count {
        let info = &detailed_printers[i];
        super::log::write_log(&format!(
            "[ListPrintersDetailed] Printer[{}]: name={} port={:?} driver={:?} comment={:?} location={:?}",
            i, info.name, info.port_name, info.driver_name, info.comment, info.location
        ));
    }
    
    let elapsed_ms = start_time.elapsed().as_millis();
    super::log::write_log(&format!("[ListPrintersDetailed] SUCCESS call_id={} elapsed_ms={} count={}", 
        call_id, elapsed_ms, detailed_printers.len()));
    
    Ok(detailed_printers)
}

/// 使用 GetPrinterW(Level=2) 获取单个打印机的完整信息
fn get_printer_info_level_2(printer_name: &str) -> Result<super::DetailedPrinterInfo, String> {
    use winapi::um::winspool::{OpenPrinterW, GetPrinterW, ClosePrinter, PRINTER_DEFAULTSW, PRINTER_INFO_2W};
    use winapi::um::winnt::LPWSTR;
    use winapi::um::errhandlingapi::GetLastError;
    use winapi::ctypes::c_void;
    use std::ptr;
    
    unsafe {
        // 将 Rust String 转换为 UTF-16 宽字符串（以 null 结尾）
        fn string_to_wide(s: &str) -> Vec<u16> {
            s.encode_utf16().chain(std::iter::once(0)).collect()
        }
        
        // 将打印机名称转换为 UTF-16 宽字符串
        let printer_name_wide = string_to_wide(printer_name);
        
        // 打开打印机
        let mut printer_handle: *mut c_void = ptr::null_mut();
        let mut defaults: PRINTER_DEFAULTSW = PRINTER_DEFAULTSW {
            pDataType: ptr::null_mut(),
            pDevMode: ptr::null_mut(),
            DesiredAccess: 0, // 只读访问
        };
        
        let open_result = OpenPrinterW(
            printer_name_wide.as_ptr() as LPWSTR,
            &mut printer_handle,
            &mut defaults,
        );
        
        if open_result == 0 {
            let error_code = GetLastError();
            let _ = ClosePrinter(printer_handle);
            return Err(format!("OpenPrinterW failed: error_code={}", error_code));
        }
        
        // 获取所需 buffer 大小
        let mut needed: u32 = 0;
        let level: u32 = 2;
        
        let _ = GetPrinterW(
            printer_handle,
            level,
            ptr::null_mut(),
            0,
            &mut needed,
        );
        
        if needed == 0 {
            let _ = ClosePrinter(printer_handle);
            return Err("GetPrinterW needed=0".to_string());
        }
        
        // 分配 buffer
        let mut buffer: Vec<u8> = vec![0; needed as usize];
        let mut returned: u32 = 0;
        
        // 获取打印机信息
        let get_result = GetPrinterW(
            printer_handle,
            level,
            buffer.as_mut_ptr() as *mut _,
            needed,
            &mut returned,
        );
        
        let mut info_result = Err("GetPrinterW failed".to_string());
        
        if get_result != 0 {
            // 解析 PRINTER_INFO_2W
            let info_ptr = buffer.as_mut_ptr() as *mut PRINTER_INFO_2W;
            let info = &*info_ptr;
            
            // 提取字段
            let name = printer_name.to_string();
            
            let port_name = if !info.pPortName.is_null() {
                match String::from_utf16(std::slice::from_raw_parts(
                    info.pPortName,
                    (0..).take_while(|&i| *info.pPortName.add(i) != 0).count(),
                )) {
                    Ok(s) => Some(s),
                    Err(_) => None,
                }
            } else {
                None
            };
            
            let driver_name = if !info.pDriverName.is_null() {
                match String::from_utf16(std::slice::from_raw_parts(
                    info.pDriverName,
                    (0..).take_while(|&i| *info.pDriverName.add(i) != 0).count(),
                )) {
                    Ok(s) => Some(s),
                    Err(_) => None,
                }
            } else {
                None
            };
            
            let comment = if !info.pComment.is_null() {
                match String::from_utf16(std::slice::from_raw_parts(
                    info.pComment,
                    (0..).take_while(|&i| *info.pComment.add(i) != 0).count(),
                )) {
                    Ok(s) if !s.is_empty() => Some(s),
                    _ => None,
                }
            } else {
                None
            };
            
            let location = if !info.pLocation.is_null() {
                match String::from_utf16(std::slice::from_raw_parts(
                    info.pLocation,
                    (0..).take_while(|&i| *info.pLocation.add(i) != 0).count(),
                )) {
                    Ok(s) if !s.is_empty() => Some(s),
                    _ => None,
                }
            } else {
                None
            };
            
            info_result = Ok(super::DetailedPrinterInfo {
                name,
                port_name,
                driver_name,
                comment,
                location,
            });
        }
        
        let _ = ClosePrinter(printer_handle);
        info_result
    }
}

