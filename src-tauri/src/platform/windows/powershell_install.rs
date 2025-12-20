// PowerShell 脚本执行模块
// 该文件负责 PowerShell 脚本的临时文件创建、执行和清理

use std::fs;
use std::io::Write;
use std::path::Path;

use crate::exec;
use crate::platform::windows::encoding::decode_windows_string;

/// 统一的错误拼装函数
/// 用于格式化命令执行错误信息，包含前缀、标准输出、错误输出和退出码
fn format_command_error(prefix: &str, stdout: &str, stderr: &str, code: Option<i32>) -> String {
    let mut error_msg = prefix.to_string();
    
    if !stdout.is_empty() {
        error_msg.push_str(&format!("\n标准输出: {}", stdout));
    }
    
    if !stderr.is_empty() {
        error_msg.push_str(&format!("\n错误输出: {}", stderr));
    }
    
    if let Some(exit_code) = code {
        error_msg.push_str(&format!("\n退出码: {}", exit_code));
    }
    
    error_msg
}

/// 执行 PowerShell 脚本文件（隐藏窗口，带超时）
/// 返回 (stdout, stderr, exit_code)
pub(crate) async fn run_powershell_file_hidden(
    script_path: &Path,
    timeout_secs: u64,
    prefix: &str,
) -> Result<(String, String, Option<i32>), String> {
    let script_path_str = script_path.to_str().ok_or_else(|| {
        format_command_error(
            &format!("{}: 脚本路径无效", prefix),
            "",
            "无法将路径转换为字符串",
            None,
        )
    })?;

    // 启动 PowerShell 进程
    let child = exec::spawn_hidden_piped("powershell", &[
        "-NoProfile",
        "-WindowStyle",
        "Hidden",
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        script_path_str,
    ])
    .map_err(|e| {
        format_command_error(
            &format!("{}: 启动 PowerShell 脚本失败", prefix),
            "",
            &e.to_string(),
            None,
        )
    })?;

    // 保存子进程 ID 以便在超时时杀死进程
    #[cfg(windows)]
    let child_pid = child.id();

    // 使用 tokio::time::timeout 添加超时控制
    let join_handle = tokio::task::spawn_blocking(move || {
        child.wait_with_output()
    });

    let output_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        join_handle,
    )
    .await;

    // 处理结果
    match output_result {
        Ok(join_result) => {
            match join_result {
                Ok(output_result) => {
                    match output_result {
                        Ok(output) => {
                            let stdout_str = decode_windows_string(&output.stdout);
                            let stderr_str = decode_windows_string(&output.stderr);
                            let exit_code = output.status.code();
                            Ok((stdout_str, stderr_str, exit_code))
                        }
                        Err(e) => Err(format_command_error(
                            &format!("{}: 执行 PowerShell 脚本失败", prefix),
                            "",
                            &e.to_string(),
                            None,
                        )),
                    }
                }
                Err(e) => Err(format_command_error(
                    &format!("{}: PowerShell 脚本任务失败", prefix),
                    "",
                    &e.to_string(),
                    None,
                )),
            }
        }
        Err(_) => {
            // 超时，尝试杀死进程
            #[cfg(windows)]
            {
                let _ = exec::taskkill_pid_hidden(child_pid);
            }
            Err(format_command_error(
                &format!("{}: PowerShell 脚本执行超时({}秒)", prefix, timeout_secs),
                "",
                "",
                None,
            ))
        }
    }
}

/// 通过临时文件执行 PowerShell 脚本
/// 该函数负责：创建临时脚本文件、写入 UTF-8 BOM、写入脚本内容、执行脚本、清理临时文件
/// 
/// # 参数
/// - `ps_script`: PowerShell 脚本内容（字符串）
/// - `timeout_secs`: 执行超时时间（秒）
/// - `prefix`: 错误信息前缀
/// 
/// # 返回
/// 返回 (stdout, stderr, exit_code) 元组，或错误信息
pub(crate) async fn run_install_script_via_temp_file(
    ps_script: &str,
    timeout_secs: u64,
    prefix: &str,
) -> Result<(String, String, Option<i32>), String> {
    // 使用 std::env::temp_dir() + 进程 id 生成唯一脚本名
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join(format!("install_printer_{}.ps1", std::process::id()));
    
    // 创建文件 -> 写 BOM -> 写 ps_script -> flush/sync -> drop(file)
    {
        let mut file = fs::File::create(&script_path)
            .map_err(|e| {
                format_command_error(
                    &format!("{}: 创建临时脚本文件失败", prefix),
                    "",
                    &e.to_string(),
                    None,
                )
            })?;
        
        // 写入 UTF-8 BOM (0xEF, 0xBB, 0xBF)
        file.write_all(&[0xEF, 0xBB, 0xBF])
            .map_err(|e| {
                format_command_error(
                    &format!("{}: 写入 UTF-8 BOM 失败", prefix),
                    "",
                    &e.to_string(),
                    None,
                )
            })?;
        
        // 写入脚本内容（UTF-8 编码）
        file.write_all(ps_script.as_bytes())
            .map_err(|e| {
                format_command_error(
                    &format!("{}: 写入脚本内容失败", prefix),
                    "",
                    &e.to_string(),
                    None,
                )
            })?;
        
        file.sync_all()
            .map_err(|e| {
                format_command_error(
                    &format!("{}: 同步脚本文件失败", prefix),
                    "",
                    &e.to_string(),
                    None,
                )
            })?;
        
        // drop(file) 确保文件已关闭
    }
    
    // 调用 run_powershell_file_hidden 执行脚本
    let result = run_powershell_file_hidden(&script_path, timeout_secs, prefix).await;
    
    // 无论成功失败，都尽力删除临时脚本文件
    if let Err(e) = fs::remove_file(&script_path) {
        eprintln!("[DEBUG] 删除临时脚本文件失败: {:?}, 错误: {}", script_path, e);
    }
    
    result
}

