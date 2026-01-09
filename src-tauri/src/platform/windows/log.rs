// Windows 平台日志工具模块
// 用于记录打印机检测等调试信息到文件
//
// 日志封装入口：
// - write_log(): 文件日志（%LOCALAPPDATA%\ePrinty\logs\printer-detect.log）
// - truncate(): 字符串截断辅助函数（用于控制台日志）

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// 获取日志文件路径
/// 
/// 返回：%LOCALAPPDATA%\ePrinty\logs\printer-detect.log
fn get_log_file_path() -> Result<PathBuf, String> {
    // 获取 LOCALAPPDATA 环境变量
    let local_app_data = std::env::var("LOCALAPPDATA")
        .map_err(|e| format!("无法获取 LOCALAPPDATA 环境变量: {}", e))?;
    
    // 构建日志目录路径
    let log_dir = PathBuf::from(local_app_data).join("ePrinty").join("logs");
    
    // 创建目录（如果不存在）
    fs::create_dir_all(&log_dir)
        .map_err(|e| format!("创建日志目录失败: {}", e))?;
    
    // 构建日志文件路径
    let log_file = log_dir.join("printer-detect.log");
    
    Ok(log_file)
}

/// 写入日志到文件
/// 
/// # 参数
/// - `message`: 日志消息
/// 
/// # 说明
/// - 使用 append 模式，每次写入后 flush，确保即使应用崩溃也能保留日志
/// - 如果写入失败，静默忽略（不阻塞业务逻辑）
/// - 每次写入都重新打开文件，避免文件句柄管理问题
pub fn write_log(message: &str) {
    // 获取日志文件路径
    let log_path = match get_log_file_path() {
        Ok(path) => path,
        Err(_) => {
            // 获取路径失败，静默忽略（不阻塞业务逻辑）
            return;
        }
    };
    
    // 打开文件（append 模式）
    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path) {
        Ok(f) => f,
        Err(_) => {
            // 打开文件失败，静默忽略（不阻塞业务逻辑）
            return;
        }
    };
    
    // 获取当前时间戳
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    // 格式化日志行：时间戳 + 消息 + 换行
    let log_line = format!("[{}] {}\n", timestamp, message);
    
    // 写入并立即 flush
    if let Err(_) = file.write_all(log_line.as_bytes()) {
        // 写入失败，静默忽略（不阻塞业务逻辑）
        return;
    }
    
    // 立即 flush，确保日志写入磁盘
    let _ = file.flush();
}

/// 字符串截断辅助函数
/// 
/// # 参数
/// - `s`: 要截断的字符串
/// - `max_len`: 最大长度（默认 2000）
/// 
/// # 返回
/// - 如果长度 <= max_len，返回原字符串
/// - 如果长度 > max_len，返回前 max_len 字符 + "...<truncated>"
pub fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...<truncated>", &s[..max_len])
    }
}

/// 路径脱敏：仅保留文件名或 %TEMP% 相对路径
/// 
/// # 参数
/// - `path`: 完整路径
/// 
/// # 返回
/// - 如果是临时目录下的文件，返回 "%TEMP%\\filename"
/// - 否则返回文件名
pub fn sanitize_path(path: &std::path::Path) -> String {
    let temp_dir = std::env::temp_dir();
    if let Ok(relative) = path.strip_prefix(&temp_dir) {
        format!("%TEMP%\\{}", relative.display())
    } else {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("?")
            .to_string()
    }
}

