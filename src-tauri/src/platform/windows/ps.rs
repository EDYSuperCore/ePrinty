// Windows 平台 PowerShell 执行统一封装模块
//
// 日志封装入口：
// - run_powershell(): 统一执行 PowerShell 命令，记录关键执行信息

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
    eprintln!("[PowerShell] START script_len={} timeout_secs={}", script.len(), POWERSHELL_TIMEOUT_SECS);
    
    // 禁止在脚本中使用 2>&1，直接执行原始脚本
    // stdout 和 stderr 分别读取，不在脚本层合流
    let mut child = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy", "Bypass",
            "-WindowStyle", "Hidden",
            "-Command",
            script
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| {
            let err_msg = format!("执行 PowerShell 命令失败: {}", e);
            eprintln!("[PowerShell] ERROR step=SPAWN message={}", err_msg);
            err_msg
        })?;
    
    let start_time = Instant::now();
    let timeout = Duration::from_secs(POWERSHELL_TIMEOUT_SECS);
    
    // 轮询检查进程是否完成
    loop {
        // 先检查超时
        if start_time.elapsed() >= timeout {
            // 超时，杀死进程
            let elapsed = start_time.elapsed();
            let _ = child.kill();
            // 等待进程结束，避免僵尸进程
            let _ = child.wait();
            let err_msg = format!("执行 PowerShell 超时: {}s (elapsed: {:.2}s)", POWERSHELL_TIMEOUT_SECS, elapsed.as_secs_f64());
            eprintln!("[PowerShell] ERROR step=TIMEOUT message={}", err_msg);
            return Err(err_msg);
        }
        
        match child.try_wait() {
            Ok(Some(status)) => {
                // 进程已完成，退出循环
                eprintln!("[PowerShell] PROCESS_COMPLETE exit_code={:?} elapsed_secs={:.2}", 
                    status.code(), start_time.elapsed().as_secs_f64());
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
                let err_msg = format!("检查 PowerShell 进程状态失败: {}", e);
                eprintln!("[PowerShell] ERROR step=WAIT message={}", err_msg);
                return Err(err_msg);
            }
        }
    }
    
    // 循环结束后，获取输出
    let output = child.wait_with_output()
        .map_err(|e| {
            let err_msg = format!("获取 PowerShell 输出失败: {}", e);
            eprintln!("[PowerShell] ERROR step=GET_OUTPUT message={}", err_msg);
            err_msg
        })?;
    
    let exit_code = output.status.code();
    let stdout_len = output.stdout.len();
    let stderr_len = output.stderr.len();
    
    // 修复成功判定：只有 exit_code == Some(0) 才记录为 SUCCESS
    if exit_code == Some(0) {
        eprintln!("[PowerShell] SUCCESS exit_code={:?} stdout_len={} stderr_len={}", 
            exit_code, stdout_len, stderr_len);
    } else {
        // exit_code != 0 一律记录为 FAILED
        // 截断脚本用于日志（前 120 字符，脱敏）
        let script_preview = if script.len() > 120 {
            format!("{}... (truncated, total {} chars)", &script[..120], script.len())
        } else {
            script.to_string()
        };
        eprintln!("[PowerShell] FAILED exit_code={:?} stdout_len={} stderr_len={} script_preview=\"{}\"", 
            exit_code, stdout_len, stderr_len, script_preview);
    }
    
    Ok(output)
}

/// 带超时的 PowerShell 命令执行函数
/// 
/// # 参数
/// - `script`: PowerShell 脚本内容（字符串）
/// - `timeout_ms`: 超时时间（毫秒）
/// 
/// # 返回
/// - `Ok(Output)`: 执行成功，返回进程输出
/// - `Err(String)`: 执行失败或超时，返回错误信息（包含错误码）
/// 
/// # 特性
/// - 统一设置：-NoProfile, -NonInteractive, -WindowStyle Hidden
/// - stdout/stderr 管道
/// - 隐藏窗口（CREATE_NO_WINDOW）
/// - 可配置的超时控制
/// - 超时后自动杀死进程并返回明确错误码
pub fn run_powershell_with_timeout(script: &str, timeout_ms: u64) -> Result<std::process::Output, String> {
    let mut child = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
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
    let timeout = Duration::from_millis(timeout_ms);
    
    // 轮询检查进程是否完成
    loop {
        // 先检查超时
        if start_time.elapsed() >= timeout {
            // 超时，杀死进程
            let _ = child.kill();
            // 等待进程结束，避免僵尸进程
            let _ = child.wait();
            let elapsed_ms = start_time.elapsed().as_millis();
            // 提取脚本的前50个字符作为提示
            let script_hint = if script.len() > 50 {
                format!("{}...", &script[..50])
            } else {
                script.to_string()
            };
            return Err(format!(
                "WIN_LIST_PRINTERS_TIMEOUT elapsed_ms={} script_hint={}",
                elapsed_ms, script_hint
            ));
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

