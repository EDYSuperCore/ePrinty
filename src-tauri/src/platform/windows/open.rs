// Windows 平台打开 URL 模块

use crate::platform::windows::cmd;

/// Windows 平台打开 URL
/// 
/// 使用 `cmd /C start "" <url>` 命令打开 URL
/// 注意：空标题 "" 是必需的，用于避免将 URL 的第一个参数误认为窗口标题
pub fn open_url_windows(url: &str) -> Result<(), String> {
    // 使用 cmd /C start "" <url> 打开 URL
    // /C 表示执行命令后关闭
    // "" 是窗口标题（空标题）
    // url 是要打开的 URL
    let result = cmd::run_command("cmd", &["/C", "start", "", url])?;
    
    if result.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&result.stderr);
        Err(format!("无法打开 URL: {}", error))
    }
}

