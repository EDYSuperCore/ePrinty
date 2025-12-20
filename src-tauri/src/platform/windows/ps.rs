// Windows 平台 PowerShell 执行统一封装模块

#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::process::Stdio;
use std::time::{Duration, Instant};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

// PowerShell 命令执行超时时间（秒）
const POWERSHELL_TIMEOUT_SECS: u64 = 120;

// 超时检查轮询间隔（毫秒）
const POLL_INTERVAL_MS: u64 = 100;

/// 统一执行 PowerShell 命令的封装函数
/// 
/// # 参数
/// - `script`: PowerShell 脚本内容（字符串）
/// 
/// # 返回
/// - `Ok(Output)`: 执行成功，返回进程输出
/// - `Err(String)`: 执行失败，返回错误信息
/// 
/// # 特性
/// - 统一设置：-NoProfile, -WindowStyle Hidden
/// - stdout/stderr 管道
/// - 隐藏窗口（CREATE_NO_WINDOW）
/// - 使用 Windows 编码解码方法处理输出
/// - 120 秒超时控制
pub fn run_powershell(script: &str) -> Result<std::process::Output, String> {
    let mut child = Command::new("powershell")
        .args([
            "-NoProfile",
            "-WindowStyle", "Hidden",
            "-Command",
            script
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("执行 PowerShell 命令失败: {}", e))?;
    
    let start_time = Instant::now();
    let timeout = Duration::from_secs(POWERSHELL_TIMEOUT_SECS);
    
    // 轮询检查进程是否完成
    loop {
        // 先检查超时
        if start_time.elapsed() >= timeout {
            // 超时，杀死进程
            let _ = child.kill();
            // 等待进程结束，避免僵尸进程
            let _ = child.wait();
            return Err(format!("执行 PowerShell 超时: {}s", POWERSHELL_TIMEOUT_SECS));
        }
        
        match child.try_wait() {
            Ok(Some(_status)) => {
                // 进程已完成，退出循环
                break;
            }
            Ok(None) => {
                // 进程仍在运行，等待后继续
                std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
            }
            Err(e) => {
                // 检查失败，杀死进程并返回错误
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("检查 PowerShell 进程状态失败: {}", e));
            }
        }
    }
    
    // 循环结束后，获取输出
    let output = child.wait_with_output()
        .map_err(|e| format!("获取 PowerShell 输出失败: {}", e))?;
    Ok(output)
}

