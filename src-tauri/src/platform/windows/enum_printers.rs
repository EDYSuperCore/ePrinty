// Windows 平台打印机枚举模块
// 使用 Win32 API EnumPrintersW 直接枚举打印机，避免 PowerShell 子进程冷启动延迟

use windows::{
    core::PCWSTR,
    Win32::Graphics::Printing::*,
    Win32::Foundation::*,
};

/// 打印机信息结构体
#[derive(Debug, Clone)]
pub struct PrinterInfo {
    pub name: String,
    pub port_name: Option<String>,
    pub driver_name: Option<String>,
}

/// 格式化 flags 为可读字符串
fn format_flags(flags: u32) -> String {
    let mut parts = Vec::new();
    if flags & PRINTER_ENUM_LOCAL != 0 {
        parts.push("PRINTER_ENUM_LOCAL");
    }
    if flags & PRINTER_ENUM_CONNECTIONS != 0 {
        parts.push("PRINTER_ENUM_CONNECTIONS");
    }
    if flags & PRINTER_ENUM_NETWORK != 0 {
        parts.push("PRINTER_ENUM_NETWORK");
    }
    if flags & PRINTER_ENUM_SHARED != 0 {
        parts.push("PRINTER_ENUM_SHARED");
    }
    if parts.is_empty() {
        format!("0x{:X}", flags)
    } else {
        format!("0x{:X} ({})", flags, parts.join(" | "))
    }
}

/// 使用 Win32 API EnumPrintersW 枚举打印机（Level 4，包含名称）
/// 
/// # 返回
/// - `Ok(Vec<PrinterInfo>)`: 成功返回打印机信息列表
/// - `Err(String)`: 失败返回错误信息
fn enum_printers_level_4(flags: u32) -> Result<Vec<PrinterInfo>, String> {
    unsafe {
        let level: u32 = 4;
        let mut needed: u32 = 0;
        let mut returned: u32 = 0;
        
        // 记录 flags 和 Level
        super::log::write_log(&format!("[EnumPrinters] Attempting Level={} (PRINTER_INFO_4W) flags={}", level, format_flags(flags)));
        
        // 第一次调用：获取所需 buffer 大小
        let result = EnumPrintersW(
            flags,
            PCWSTR::null(),
            level,
            None,
            &mut needed,
            &mut returned,
        );
        
        let first_call_success = result.is_ok();
        let first_error = if result.is_err() {
            Some(result.unwrap_err())
        } else {
            None
        };
        
        // 记录第一次调用结果
        let first_error_str = if let Some(err) = &first_error {
            format!("{:?}", err)
        } else {
            "None".to_string()
        };
        super::log::write_log(&format!(
            "[EnumPrinters] First call (Level={}): success={} error={} bytesNeeded={} count={}",
            level, first_call_success, first_error_str, needed, returned
        ));
        
        if !first_call_success && needed == 0 {
            return Err(format!("EnumPrintersW Level {} 第一阶段调用失败: {}", level, first_error_str));
        }
        
        if needed == 0 {
            return Ok(Vec::new());
        }
        
        // 第二阶段：分配 buffer 并获取数据
        let mut buffer: Vec<u8> = vec![0; needed as usize];
        let mut returned2: u32 = 0;
        let mut needed2 = needed;
        
        let result = EnumPrintersW(
            flags,
            PCWSTR::null(),
            level,
            Some(buffer.as_mut_slice()),
            &mut needed2,
            &mut returned2,
        );
        
        let second_call_success = result.is_ok();
        let second_error = if result.is_err() {
            Some(result.unwrap_err())
        } else {
            None
        };
        
        // 记录第二次调用结果
        let second_error_str = if let Some(err) = &second_error {
            format!("{:?}", err)
        } else {
            "None".to_string()
        };
        super::log::write_log(&format!(
            "[EnumPrinters] Second call (Level={}): success={} error={} bytesNeeded={} count={}",
            level, second_call_success, second_error_str, needed2, returned2
        ));
        
        if !second_call_success {
            return Err(format!("EnumPrintersW Level {} 第二阶段调用失败: {}", level, second_error_str));
        }
        
        // 解析 PRINTER_INFO_4W 数据
        let mut printers = Vec::new();
        let base_ptr = buffer.as_ptr() as *const PRINTER_INFO_4W;
        
        for i in 0..returned2 {
            let printer_info = base_ptr.add(i as usize);
            let info = &*printer_info;
            
            // 提取打印机名称
            let name = if info.pPrinterName.is_null() {
                super::log::write_log(&format!("[EnumPrinters] Record {}: pPrinterName is null, skipping", i));
                continue;
            } else {
                match info.pPrinterName.to_string() {
                    Ok(s) => s,
                    Err(_) => {
                        super::log::write_log(&format!("[EnumPrinters] Record {}: pPrinterName conversion failed, skipping", i));
                        continue;
                    }
                }
            };
            
            // PRINTER_INFO_4W 只包含名称，端口和驱动设为 None
            printers.push(PrinterInfo {
                name,
                port_name: None,
                driver_name: None,
            });
        }
        
        // 检查解析结果数量是否与 count 一致
        if printers.len() != returned2 as usize {
            super::log::write_log(&format!(
                "[EnumPrinters] 解析异常: count={} 但解析后数量={}, 可能部分记录被跳过",
                returned2, printers.len()
            ));
        }
        
        // 打印前 5 条打印机信息
        let print_count = std::cmp::min(5, printers.len());
        for i in 0..print_count {
            let info = &printers[i];
            super::log::write_log(&format!(
                "[EnumPrinters] Printer[{}]: name={}",
                i, info.name
            ));
        }
        
        Ok(printers)
    }
}

/// 使用 Win32 API EnumPrintersW 枚举打印机
/// 
/// # 返回
/// - `Ok(Vec<PrinterInfo>)`: 成功返回打印机信息列表
/// - `Err(String)`: 失败返回错误信息
/// 
/// # 实现说明
/// - 使用 PRINTER_INFO_4W（Level 4），包含打印机名称
/// - flags 使用 PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS | PRINTER_ENUM_NETWORK
/// - 两阶段调用：先获取所需 buffer size，再分配 buffer 获取数据
/// - 注意：windows-rs 0.52 中 PRINTER_INFO_2W 可能不可用，因此使用 PRINTER_INFO_4W
pub fn enum_printers_w() -> Result<Vec<PrinterInfo>, String> {
    // 设置 flags：包含本地、连接和网络打印机
    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS | PRINTER_ENUM_NETWORK;
    
    // 使用 Level 4（PRINTER_INFO_4W）枚举打印机
    match enum_printers_level_4(flags) {
        Ok(printers) => {
            super::log::write_log(&format!("[EnumPrinters] Successfully enumerated {} printers using Level 4", printers.len()));
            Ok(printers)
        }
        Err(e) => {
            super::log::write_log(&format!("[EnumPrinters] Level 4 failed: {}", e));
            Err(format!("EnumPrintersW 失败: {}", e))
        }
    }
}
