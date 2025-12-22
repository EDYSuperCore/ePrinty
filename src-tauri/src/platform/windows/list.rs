// Windows 平台打印机列表获取模块

use std::sync::atomic::{AtomicU64, Ordering};

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

