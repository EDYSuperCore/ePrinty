// 命令执行封装模块

use std::process::{Command, Stdio};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

/// 命令执行输出结果
pub struct ExecOutput {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// 执行外部命令（隐藏窗口，收集输出）
#[cfg(windows)]
pub fn run_hidden(program: &str, args: &[&str]) -> Result<ExecOutput, String> {
    let output = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("执行命令失败: program={}, args={:?}, error={}", program, args, e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let status_code = output.status.code();
    
    Ok(ExecOutput {
        status_code,
        stdout,
        stderr,
    })
}

/// 执行外部命令（非 Windows 平台 stub）
#[cfg(not(windows))]
pub fn run_hidden(_program: &str, _args: &[&str]) -> Result<ExecOutput, String> {
    Err("run_hidden 仅在 Windows 平台支持".to_string())
}

/// 执行外部命令（隐藏窗口，带超时，收集输出）
#[cfg(windows)]
pub fn run_hidden_with_timeout(program: &str, args: &[&str], timeout_secs: u64) -> Result<ExecOutput, String> {
    // 简单实现：使用同步 output，超时参数保留但不实现（由调用侧使用 tokio::timeout 处理）
    run_hidden(program, args)
}

/// 执行外部命令（非 Windows 平台 stub）
#[cfg(not(windows))]
pub fn run_hidden_with_timeout(_program: &str, _args: &[&str], _timeout_secs: u64) -> Result<ExecOutput, String> {
    Err("run_hidden_with_timeout 仅在 Windows 平台支持".to_string())
}

/// 启动外部命令（隐藏窗口，返回 Child 句柄，用于异步等待）
#[cfg(windows)]
pub fn spawn_hidden_piped(program: &str, args: &[&str]) -> Result<std::process::Child, String> {
    use std::process::Stdio;
    
    Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("启动进程失败: program={}, args={:?}, error={}", program, args, e))
}

/// 启动外部命令（非 Windows 平台 stub）
#[cfg(not(windows))]
pub fn spawn_hidden_piped(_program: &str, _args: &[&str]) -> Result<std::process::Child, String> {
    Err("spawn_hidden_piped 仅在 Windows 平台支持".to_string())
}

/// 使用 taskkill 终止进程（隐藏窗口）
#[cfg(windows)]
pub fn taskkill_pid_hidden(pid: u32) -> Result<(), String> {
    use std::process::Stdio;
    
    // 执行 taskkill，失败不影响主流程（保持与原行为一致）
    let _ = Command::new("taskkill")
        .args(["/F", "/PID", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    
    Ok(())
}

/// 使用 taskkill 终止进程（非 Windows 平台 stub）
#[cfg(not(windows))]
pub fn taskkill_pid_hidden(_pid: u32) -> Result<(), String> {
    Ok(()) // 非 Windows 平台，taskkill 无效，直接返回成功
}

