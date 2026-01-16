// Windows 平台打印机存在性检查模块
// 使用 Win32 Print Spooler API 判断打印机队列是否存在

#[cfg(target_os = "windows")]
use winapi::um::winspool::{OpenPrinterW, ClosePrinter, PRINTER_DEFAULTSW};
#[cfg(target_os = "windows")]
use winapi::um::winnt::LPWSTR;
#[cfg(target_os = "windows")]
use winapi::um::errhandlingapi::GetLastError;
#[cfg(target_os = "windows")]
use winapi::ctypes::c_void;
#[cfg(target_os = "windows")]
use std::ptr;

/// 检查打印机是否存在（Windows 实现）
/// 
/// # 参数
/// - `name`: 打印机队列名称
/// 
/// # 返回值
/// - `(exists, last_error, evidence)`
///   - `exists`: 打印机是否存在
///   - `last_error`: 如果失败，返回 Windows 错误代码（Option<u32>）
///   - `evidence`: 证据字符串，用于日志记录（Option<String>）
#[cfg(target_os = "windows")]
pub fn printer_exists(name: &str) -> (bool, Option<u32>, Option<String>) {
    unsafe {
        // 将 Rust &str 转换为 UTF-16 宽字符串（以 null 结尾）
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        
        // 打开打印机句柄
        let mut printer_handle: *mut c_void = ptr::null_mut();
        let mut defaults: PRINTER_DEFAULTSW = PRINTER_DEFAULTSW {
            pDataType: ptr::null_mut(),
            pDevMode: ptr::null_mut(),
            DesiredAccess: 0, // 只读访问
        };
        
        // 调用 OpenPrinterW 尝试打开打印机
        let open_result = OpenPrinterW(
            wide.as_ptr() as LPWSTR,
            &mut printer_handle,
            &mut defaults,
        );
        
        if open_result != 0 {
            // 成功打开，说明打印机存在
            // 立即关闭句柄
            let _ = ClosePrinter(printer_handle);
            
            let evidence = format!("OpenPrinterW ok printer_name=\"{}\"", name);
            (true, None, Some(evidence))
        } else {
            // 打开失败，获取错误代码
            let error_code = GetLastError();
            
            let evidence = format!("OpenPrinterW failed printer_name=\"{}\" last_error={}", name, error_code);
            (false, Some(error_code), Some(evidence))
        }
    }
}

/// 非 Windows 平台的占位实现（不应被调用）
/// 
/// 注意：此函数仅用于编译通过，实际使用时应通过平台特定路径调用
#[cfg(not(target_os = "windows"))]
pub fn printer_exists(_name: &str) -> (bool, Option<u32>, Option<String>) {
    // 非 Windows 平台不应调用此函数
    // 如果被调用，返回不存在（安全策略）
    (false, None, Some("printer_exists: not implemented on non-Windows platform".to_string()))
}
