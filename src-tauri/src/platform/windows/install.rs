// Windows 平台打印机安装模块
// 该文件是 Windows 安装入口实现，分为 Add-Printer 与 VBS 分支

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;


// ============================================================================
// 常量定义
// ============================================================================

// PowerShell 命令执行超时时间（秒）
// 注意：当前实现中未使用，保留以备将来需要超时控制时使用
#[allow(dead_code)]
const POWERSHELL_TIMEOUT_SECS: u64 = 120; // 2分钟，用于打印机安装相关命令

// 嵌入的 prnport.vbs 脚本内容（在编译时打包进 exe）
// 注意：VBS 文件可能是 GBK/ANSI 编码，使用 include_bytes! 直接嵌入原始字节
// 写入文件时保持原始编码，因为 VBScript 需要 ANSI/GBK 编码才能正确解析
const PRNPORT_VBS_BYTES: &[u8] = include_bytes!("../../../scripts/prnport.vbs");

// ============================================================================
// 数据结构
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct InstallResult {
    pub success: bool,
    pub message: String,
    pub method: Option<String>, // 安装方式："VBS" 或 "Add-Printer"
    pub stdout: Option<String>,  // PowerShell 标准输出
    pub stderr: Option<String>,  // PowerShell 错误输出
}

// ============================================================================
// 错误类型
// ============================================================================

/// 打印机安装过程中的结构化错误类型
#[derive(Debug)]
enum InstallError {
    /// 外部命令执行失败
    CommandFailed {
        step: &'static str,
        command: String,
        stderr: String,
    },
    /// PowerShell 命令执行失败
    PowerShellFailed {
        step: &'static str,
        stderr: String,
    },
    /// 输入参数无效
    InvalidInput {
        field: &'static str,
        reason: String,
    },
    /// 配置无效
    InvalidConfig {
        reason: String,
    },
    /// 未找到打印机驱动
    DriverNotFound {
        step: &'static str,
    },
    /// 端口创建超时
    PortCreateTimeout {
        port_name: String,
    },
    /// 文件操作失败
    FileOperationFailed {
        step: &'static str,
        operation: &'static str,
        error: String,
    },
    /// 端口添加失败（现代方式）
    PortAddFailedModern {
        stdout: String,
        stderr: String,
    },
    /// 端口添加失败（VBS 方式）
    PortAddFailedVbs {
        error_details: String,
        exit_code: i32,
        stdout: String,
        stderr: String,
    },
    /// VBS 脚本执行失败
    VbsScriptFailed {
        error: String,
        script_info: String,
    },
    /// 打印机安装失败（现代方式）
    PrinterInstallFailedModern {
        stderr: String,
    },
    /// 打印机安装失败（VBS 方式）
    PrinterInstallFailedVbs {
        stderr: String,
    },
    /// INF 驱动安装失败
    InfInstallFailed {
        inf_path: String,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
    },
    /// PrintUIEntry /if 安装失败
    PrintUIInfInstallFailed {
        printer_name: String,
        inf_path: String,
        port_name: String,
        model: String,
        exit_code: Option<i32>,
        stdout: String,
        stderr: String,
    },
}

impl InstallError {
    /// 获取错误的稳定错误码
    fn code(&self) -> &'static str {
        match self {
            InstallError::CommandFailed { .. } => "WIN_CMD_FAILED",
            InstallError::PowerShellFailed { .. } => "WIN_PS_FAILED",
            InstallError::InvalidInput { .. } => "WIN_INVALID_INPUT",
            InstallError::InvalidConfig { .. } => "WIN_INVALID_CONFIG",
            InstallError::DriverNotFound { .. } => "WIN_DRIVER_NOT_FOUND",
            InstallError::PortCreateTimeout { .. } => "WIN_PORT_TIMEOUT",
            InstallError::FileOperationFailed { .. } => "WIN_FILE_FAILED",
            InstallError::PortAddFailedModern { .. } => "WIN_PORT_FAILED",
            InstallError::PortAddFailedVbs { .. } => "WIN_PORT_FAILED",
            InstallError::VbsScriptFailed { .. } => "WIN_VBS_FAILED",
            InstallError::PrinterInstallFailedModern { .. } => "WIN_PRINTER_INSTALL_FAILED",
            InstallError::PrinterInstallFailedVbs { .. } => "WIN_PRINTER_INSTALL_FAILED",
            InstallError::InfInstallFailed { .. } => "WIN_INF_INSTALL_FAILED",
            InstallError::PrintUIInfInstallFailed { .. } => "WIN_PRINTUI_INF_INSTALL_FAILED",
        }
    }

    /// 获取错误的 stdout 和 stderr（如果存在）
    fn get_output(&self) -> (Option<String>, Option<String>) {
        match self {
            InstallError::PortAddFailedModern { stdout, stderr } => {
                (Some(stdout.clone()), Some(stderr.clone()))
            }
            InstallError::PortAddFailedVbs { stdout, stderr, .. } => {
                (Some(stdout.clone()), Some(stderr.clone()))
            }
            InstallError::InfInstallFailed { stdout, stderr, .. } => {
                (Some(stdout.clone()), Some(stderr.clone()))
            }
            InstallError::PrintUIInfInstallFailed { stdout, stderr, .. } => {
                (Some(stdout.clone()), Some(stderr.clone()))
            }
            _ => (None, None),
        }
    }

    /// 为 stderr 添加错误码前缀
    fn format_stderr_with_code(&self, stderr: Option<String>) -> Option<String> {
        let code = self.code();
        match stderr {
            Some(s) if !s.trim().is_empty() => {
                Some(format!("[EASYPRINTER_CODE={}] {}", code, s))
            }
            _ => Some(format!("[EASYPRINTER_CODE={}]", code)),
        }
    }

    /// 将错误转换为用户友好的错误消息
    /// 返回与当前实现完全一致的错误文案（逐字一致）
    fn to_user_message(&self) -> String {
        match self {
            InstallError::CommandFailed { step: _, command, stderr } => {
                format!("执行 {} 命令失败: {}", command, stderr)
            }
            InstallError::PowerShellFailed { step, stderr } => {
                // 保持与原始错误消息格式一致
                // 原始代码中，run_powershell 返回的 Err(String) 已经是完整错误消息
                // 但在某些地方会再次包装，需要根据 step 来决定格式
                match *step {
                    "add_printer_port_modern" => format!("执行 Add-PrinterPort 命令失败: {}", stderr),
                    "add_printer_with_driver_modern" => format!("端口添加成功，但执行 Add-Printer 命令失败: {}", stderr),
                    "add_printer_with_driver_vbs" => format!("端口添加成功，但执行 PowerShell 命令失败: {}", stderr),
                    _ => stderr.clone(),
                }
            }
            InstallError::InvalidInput { field, reason } => {
                format!("输入参数 {} 无效: {}", field, reason)
            }
            InstallError::InvalidConfig { reason } => {
                reason.clone()
            }
            InstallError::DriverNotFound { step: _ } => {
                "系统中没有可用的打印机驱动。请先安装打印机驱动。".to_string()
            }
            InstallError::PortCreateTimeout { port_name: _ } => {
                "端口创建超时".to_string()
            }
            InstallError::FileOperationFailed { step: _, operation, error } => {
                match *operation {
                    "创建临时脚本文件" => format!("创建临时脚本文件失败: {}", error),
                    "写入脚本内容" => format!("写入脚本内容失败: {}", error),
                    "同步脚本文件" => format!("同步脚本文件失败: {}", error),
                    _ => format!("文件操作失败: {}", error),
                }
            }
            InstallError::PortAddFailedModern { stdout, stderr } => {
                format!("添加打印机端口失败。标准输出: {}。错误信息: {}。请确保有管理员权限。", stdout, stderr)
            }
            InstallError::PortAddFailedVbs { error_details, exit_code, stdout: _, stderr: _ } => {
                format!("添加打印机端口失败。{} | 退出代码: {}", error_details, exit_code)
            }
            InstallError::VbsScriptFailed { error, script_info } => {
                format!("执行 prnport.vbs 脚本失败: {} | {}", error, script_info)
            }
            InstallError::PrinterInstallFailedModern { stderr } => {
                format!("端口添加成功，但打印机安装失败。错误信息: {}。请确保系统中已安装打印机驱动，或联系管理员安装驱动。", stderr)
            }
            InstallError::PrinterInstallFailedVbs { stderr } => {
                format!("端口添加成功，但打印机安装失败。错误信息: {}。请确保系统中已安装打印机驱动，或联系管理员安装驱动。", stderr)
            }
            InstallError::InfInstallFailed { inf_path, exit_code, stdout, stderr } => {
                let exit_msg = match exit_code {
                    Some(code) => format!("退出代码: {}", code),
                    None => "无法获取退出代码".to_string(),
                };
                format!("从配置文件安装 INF 驱动失败。文件: {}。{}。标准输出: {}。错误输出: {}", 
                    inf_path, exit_msg, stdout, stderr)
            }
            InstallError::PrintUIInfInstallFailed { printer_name, inf_path, port_name, model, exit_code, stdout, stderr } => {
                let exit_msg = match exit_code {
                    Some(code) => format!("退出代码: {}", code),
                    None => "无法获取退出代码".to_string(),
                };
                format!("使用 PrintUIEntry 安装打印机失败。打印机: {}，驱动: {}，端口: {}，型号: {}。{}。标准输出: {}。错误输出: {}", 
                    printer_name, inf_path, port_name, model, exit_msg, stdout, stderr)
            }
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

// 使用 encoding 模块的 decode_windows_string
use crate::platform::windows::encoding::decode_windows_string;

// 注意：以下辅助函数在当前实现中未使用，但保留以备将来需要时使用
// format_command_error 和 run_cscript_hidden 函数已移除，因为当前实现直接使用 Command 和 decode_windows_string

/// 检测 Windows 版本（返回构建号，用于判断是否支持 Add-PrinterPort）
/// 注意：GetVersionExW API 在 Windows 10+ 可能返回兼容版本信息（如 9200），不准确
/// 因此优先使用 PowerShell 获取真实版本信息
fn get_windows_build_number() -> Result<u32, String> {
    // 优先使用 PowerShell 检测真实构建号（更可靠）
    // 使用 Get-CimInstance 获取真实的操作系统版本信息
    match super::ps::run_powershell("(Get-CimInstance Win32_OperatingSystem).BuildNumber") {
        Ok(output) => {
            let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            
            if !version_str.is_empty() {
                if let Ok(build_number) = version_str.parse::<u32>() {
                    return Ok(build_number);
                }
            }
        }
        Err(_) => {}
    }
    
    // 备用方案：使用 Environment.OSVersion
    match super::ps::run_powershell("[System.Environment]::OSVersion.Version.Build") {
        Ok(output) => {
            let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            
            if !version_str.is_empty() {
                if let Ok(build_number) = version_str.parse::<u32>() {
                    return Ok(build_number);
                }
            }
        }
        Err(_) => {}
    }
    
    // 最后的备用方案：使用 GetVersionExW（但可能不准确）
    use winapi::um::sysinfoapi::GetVersionExW;
    use winapi::um::winnt::OSVERSIONINFOW;
    
    unsafe {
        let mut os_info: OSVERSIONINFOW = std::mem::zeroed();
        os_info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
        
        if GetVersionExW(&mut os_info) != 0 {
            Ok(os_info.dwBuildNumber)
        } else {
            Err("无法检测 Windows 构建号".to_string())
        }
    }
}

// ============================================================================
// 私有辅助类型
// ============================================================================

/// 端口添加结果
#[derive(Debug, Clone, Copy)]
enum PortAddOutcome {
    /// 端口已创建
    Created,
    /// 端口已存在
    AlreadyExists,
}

// ============================================================================
// 私有辅助函数
// ============================================================================

/// 删除已存在的打印机（如果存在）
/// 静默模式，忽略错误，隐藏窗口
fn delete_existing_printer(name: &str) {
    // rundll32 的 /q 参数会静默执行，不会显示窗口
    // 使用 CREATE_NO_WINDOW 标志确保在打包后也不显示窗口
    let printer_name_arg = format!("\"{}\"", name);
    let _ = super::cmd::run_command("rundll32", &[
        "printui.dll,PrintUIEntry",
        "/dl",  // 删除本地打印机
        "/n",   // 打印机名称
        &printer_name_arg,
        "/q"    // 静默模式，不显示确认对话框
    ]);
}

/// 使用 PrintUIEntry /if 安装打印机（同时导入驱动）
/// 
/// # 参数
/// - `printer_name`: 打印机名称
/// - `inf_path`: INF 驱动文件路径
/// - `port_name`: 端口名称
/// - `model`: 打印机型号
/// 
/// # 返回
/// - `Ok(InstallResult)`: 安装成功
/// - `Err(InstallError)`: 安装失败
/// 
/// # 实现说明
/// 使用 rundll32 printui.dll,PrintUIEntry /if 安装打印机
/// 该命令会同时导入驱动并创建打印机队列
/// 命令格式：rundll32 printui.dll,PrintUIEntry /if /b "<printer_name>" /f "<inf_path>" /r "<port_name>" /m "<model>" /z
fn install_printer_with_printui(
    printer_name: &str,
    inf_path: &std::path::Path,
    port_name: &str,
    model: &str,
) -> Result<InstallResult, InstallError> {
    eprintln!("[DEBUG] 使用 PrintUIEntry /if 安装打印机: {}", printer_name);
    eprintln!("[DEBUG] INF 路径: {}", inf_path.display());
    eprintln!("[DEBUG] 端口: {}", port_name);
    eprintln!("[DEBUG] 型号: {}", model);
    
    // 检查 INF 文件是否存在
    if !inf_path.exists() {
        return Err(InstallError::FileOperationFailed {
            step: "install_printer_with_printui",
            operation: "检查 INF 文件",
            error: format!("INF 文件不存在: {}", inf_path.display()),
        });
    }
    
    // 将路径转换为绝对路径
    let inf_path_abs = match inf_path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            return Err(InstallError::FileOperationFailed {
                step: "install_printer_with_printui",
                operation: "解析 INF 文件路径",
                error: format!("无法解析 INF 文件路径: {}", e),
            });
        }
    };
    
    // 将路径转换为字符串
    let inf_path_str = inf_path_abs.to_string_lossy().to_string();
    
    // 剥离 "\\?\" 扩展路径前缀（PrintUIEntry 不支持此前缀）
    // canonicalize() 可能返回 "\\?\E:\..." 格式，需要转换为标准 Win32 路径 "E:\..."
    let inf_path_normalized = if inf_path_str.starts_with(r"\\?\") {
        // 剥离 "\\?\" 前缀
        let without_prefix = &inf_path_str[4..];
        // 如果是 UNC 路径 "\\?\UNC\server\share"，转换为 "\\server\share"
        if without_prefix.starts_with(r"UNC\") {
            format!(r"\\{}", &without_prefix[4..])
        } else {
            without_prefix.to_string()
        }
    } else {
        inf_path_str
    };
    
    // 确保路径使用反斜杠（Windows 标准）
    // 虽然 Rust 的 Path 通常会自动处理，但为了确保一致性，我们显式转换
    let inf_path_final = inf_path_normalized.replace("/", "\\");
    
    eprintln!("[DEBUG] 规范化后的 inf_path: {}", inf_path_final);
    eprintln!("[DEBUG] 执行 PrintUIEntry: /if /b \"{}\" /f \"{}\" /r \"{}\" /m \"{}\" /z", 
        printer_name, inf_path_final, port_name, model);
    
    // 使用 rundll32 printui.dll,PrintUIEntry /if 安装打印机
    // 注意：Command::args() 会自动处理参数转义，不需要手动添加引号
    // /if: 安装打印机（从 INF 文件）
    // /b: 打印机名称
    // /f: INF 文件路径
    // /r: 端口名称
    // /m: 打印机型号
    // /z: 静默模式（不显示确认对话框）
    match super::cmd::run_command("rundll32.exe", &[
        "printui.dll,PrintUIEntry",
        "/if",
        "/b",
        printer_name,
        "/f",
        &inf_path_final,
        "/r",
        port_name,
        "/m",
        model,
        "/z"
    ]) {
        Ok(output) => {
            let stdout = decode_windows_string(&output.stdout);
            let stderr = decode_windows_string(&output.stderr);
            let exit_code = output.status.code();
            
            eprintln!("[DEBUG] PrintUIEntry exit code: {:?}, stdout length: {}, stderr length: {}", 
                exit_code, stdout.len(), stderr.len());
            
            if output.status.success() {
                eprintln!("[DEBUG] PrintUIEntry 执行成功，打印机已安装");
                Ok(InstallResult {
                    success: true,
                    message: format!("打印机 {} 安装成功（使用 PrintUIEntry）", printer_name),
                    method: Some("PrintUIEntry".to_string()),
                    stdout: Some(stdout),
                    stderr: Some(stderr),
                })
            } else {
                eprintln!("[ERROR] PrintUIEntry 执行失败，exit code: {:?}", exit_code);
                let error = InstallError::PrintUIInfInstallFailed {
                    printer_name: printer_name.to_string(),
                    inf_path: inf_path_final.clone(),
                    port_name: port_name.to_string(),
                    model: model.to_string(),
                    exit_code,
                    stdout,
                    stderr,
                };
                Err(error)
            }
        }
        Err(e) => {
            eprintln!("[ERROR] PrintUIEntry 命令执行失败: {}", e);
            Err(InstallError::CommandFailed {
                step: "install_printer_with_printui",
                command: format!("rundll32.exe printui.dll,PrintUIEntry /if /b \"{}\" /f \"{}\" /r \"{}\" /m \"{}\" /z", 
                    printer_name, inf_path_final, port_name, model),
                stderr: e,
            })
        }
    }
}

/// 安装 INF 驱动文件
/// 
/// # 参数
/// - `inf_path`: INF 文件的完整路径
/// - `driver_names`: 驱动名称候选列表，用于验证安装是否成功
/// 
/// # 返回
/// - `Ok(())`: 安装成功且驱动已注册
/// - `Err(InstallError)`: 安装失败或驱动未注册
/// 
/// # 实现说明
/// 使用 pnputil.exe 安装 INF 驱动
/// pnputil 是 Windows 推荐的驱动安装工具，比 Add-PrinterDriver 更可靠
fn install_inf_driver(inf_path: &std::path::Path, driver_names: &[String]) -> Result<(), InstallError> {
    eprintln!("[DEBUG] 开始安装 INF 驱动: {}", inf_path.display());
    
    // 检查 INF 文件是否存在
    if !inf_path.exists() {
        return Err(InstallError::FileOperationFailed {
            step: "install_inf_driver",
            operation: "检查 INF 文件",
            error: format!("INF 文件不存在: {}", inf_path.display()),
        });
    }
    
    // 将路径转换为绝对路径（使用原生 Windows 路径格式）
    let inf_path_abs = match inf_path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            return Err(InstallError::FileOperationFailed {
                step: "install_inf_driver",
                operation: "解析 INF 文件路径",
                error: format!("无法解析 INF 文件路径: {}", e),
            });
        }
    };
    
    // 将路径转换为字符串，使用引号包裹以处理包含空格/中文的路径
    let inf_path_str = inf_path_abs.to_string_lossy();
    let inf_path_quoted = format!("\"{}\"", inf_path_str);
    
    eprintln!("[DEBUG] 执行 pnputil: /add-driver {} /install", inf_path_quoted);
    
    // 使用 pnputil.exe 安装 INF 驱动
    // pnputil.exe /add-driver "<inf_path>" /install
    match super::cmd::run_command("pnputil.exe", &[
        "/add-driver",
        &inf_path_quoted,
        "/install"
    ]) {
        Ok(output) => {
            let stdout = decode_windows_string(&output.stdout);
            let stderr = decode_windows_string(&output.stderr);
            let exit_code = output.status.code();
            
            eprintln!("[DEBUG] pnputil exit code: {:?}, stdout length: {}, stderr length: {}", 
                exit_code, stdout.len(), stderr.len());
            
            // pnputil 成功时 exit code 为 0
            if output.status.success() {
                eprintln!("[DEBUG] pnputil 执行成功");
                
                // 安装成功后，立即验证 driver_names 中是否有已安装驱动可用
                match select_installed_driver_name(driver_names) {
                    Ok(driver_name) => {
                        eprintln!("[DEBUG] INF 驱动安装成功，找到已注册驱动: {}", driver_name);
                        Ok(())
                    }
                    Err((e, _)) => {
                        // INF 安装完成但 driver_names 不可用
                        let candidates_str = driver_names.join(", ");
                        eprintln!("[WARN] INF 安装完成但 driver_names 中未找到已注册驱动。候选: {}", candidates_str);
                        
                        Err(InstallError::InfInstallFailed {
                            inf_path: inf_path_str.to_string(),
                            exit_code: Some(0), // pnputil 成功，但驱动未注册
                            stdout: format!("pnputil 执行成功，但 driver_names 中未找到已安装驱动。候选列表: {}", candidates_str),
                            stderr: format!("请检查 driver_names 是否与系统 DriverName 一致。候选: {}", candidates_str),
                        })
                    }
                }
            } else {
                // pnputil 执行失败
                eprintln!("[ERROR] pnputil 执行失败，exit code: {:?}", exit_code);
                Err(InstallError::InfInstallFailed {
                    inf_path: inf_path_str.to_string(),
                    exit_code,
                    stdout,
                    stderr,
                })
            }
        }
        Err(e) => {
            // 命令执行失败（如进程启动失败）
            eprintln!("[ERROR] pnputil 命令执行失败: {}", e);
            Err(InstallError::CommandFailed {
                step: "install_inf_driver",
                command: format!("pnputil.exe /add-driver {} /install", inf_path_quoted),
                stderr: e,
            })
        }
    }
}

/// 获取应用目录（可执行文件所在目录）
fn get_app_directory() -> Result<std::path::PathBuf, String> {
    std::env::current_exe()
        .and_then(|exe_path| {
            exe_path.parent()
                .ok_or_else(|| std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "无法获取可执行文件目录"
                ))
                .map(|dir| dir.to_path_buf())
        })
        .map_err(|e| format!("获取应用目录失败: {}", e))
}

/// 解析 driver_path（相对于应用目录）
fn resolve_driver_path(driver_path: &str) -> Result<std::path::PathBuf, InstallError> {
    let app_dir = get_app_directory()
        .map_err(|e| InstallError::FileOperationFailed {
            step: "resolve_driver_path",
            operation: "获取应用目录",
            error: e,
        })?;
    
    // driver_path 是相对于应用目录的路径
    let full_path = app_dir.join(driver_path);
    
    Ok(full_path)
}

/// 按候选列表选择已安装驱动名
/// 对每个候选驱动名执行 PowerShell 查询，返回第一个已安装的驱动名
fn select_installed_driver_name(candidates: &[String]) -> Result<String, (InstallError, Option<String>)> {
    // 过滤并 trim 候选列表
    let filtered_candidates: Vec<String> = candidates
        .iter()
        .map(|c| c.trim().to_string())
        .filter(|c| !c.is_empty())
        .collect();
    
    if filtered_candidates.is_empty() {
        return Err((
            InstallError::DriverNotFound {
                step: "select_driver",
            },
            None,
        ));
    }
    
    // 收集所有检查失败的 stderr 信息（用于诊断）
    let mut last_stderr: Option<String> = None;
    
    // 逐个检查候选驱动是否已安装
    for candidate in &filtered_candidates {
        // 使用 PowerShell 精确匹配驱动名
        let check_script = format!(
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-PrinterDriver -Name '{}' -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Name",
            candidate.replace("'", "''")
        );
        
        match super::ps::run_powershell(&check_script) {
            Ok(output) => {
                let stdout = decode_windows_string(&output.stdout);
                let trimmed_stdout = stdout.trim();
                
                // 如果 stdout 非空，说明驱动已安装
                if !trimmed_stdout.is_empty() {
                    eprintln!("[DEBUG] 找到已安装的驱动: {}", trimmed_stdout);
                    return Ok(trimmed_stdout.to_string());
                }
                
                // 记录 stderr（即使 stdout 为空，也可能有错误信息）
                let stderr = decode_windows_string(&output.stderr);
                if !stderr.trim().is_empty() {
                    last_stderr = Some(stderr);
                }
            }
            Err(e) => {
                // PowerShell 执行失败，记录错误信息
                last_stderr = Some(e);
                // 继续检查下一个候选
            }
        }
    }
    
    // 所有候选都未命中，返回错误和最后的 stderr
    Err((
        InstallError::DriverNotFound {
            step: "select_driver",
        },
        last_stderr,
    ))
}

/// 验证打印机端口是否存在
/// 重试几次，因为端口创建可能需要时间
fn verify_printer_port(port_name: &str) -> Result<bool, InstallError> {
    let mut port_verified = false;
    for attempt in 1..=3 {
        eprintln!("[DEBUG] 验证端口存在（尝试 {}/3）", attempt);
        
        let verify_port_script = format!(
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $port = Get-PrinterPort -Name '{}' -ErrorAction SilentlyContinue; if ($port) {{ Write-Output 'PortVerified' }} else {{ Write-Error 'Port not found' }}",
            port_name.replace("'", "''")
        );
        let verify_port = super::ps::run_powershell(&verify_port_script);
        
        match verify_port {
            Ok(verify_result) => {
                let verify_stdout = decode_windows_string(&verify_result.stdout);
                if verify_stdout.contains("PortVerified") {
                    eprintln!("[DEBUG] 端口验证成功");
                    port_verified = true;
                    break;
                } else {
                    eprintln!("[DEBUG] 端口验证失败，等待后重试...");
                    if attempt < 3 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
            }
            Err(e) => {
                eprintln!("[DEBUG] 无法验证端口，等待后重试...");
                // 如果是最后一次尝试，返回错误
                if attempt == 3 {
                    return Err(InstallError::PowerShellFailed {
                        step: "verify_printer_port",
                        stderr: e,
                    });
                }
                if attempt < 3 {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                }
            }
        }
    }
    Ok(port_verified)
}

/// 使用现代方式（Add-PrinterPort）添加打印机端口
fn add_printer_port_modern(port_name: &str, ip_address: &str) -> Result<PortAddOutcome, InstallError> {
    eprintln!("[DEBUG] 添加打印机端口 {}", port_name);
    let port_add_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; try {{ Add-PrinterPort -Name '{}' -PrinterHostAddress '{}' -ErrorAction Stop; Write-Output 'PortSuccess' }} catch {{ if ($_.Exception.Message -like '*already exists*' -or $_.Exception.Message -like '*已存在*') {{ Write-Output 'PortExists' }} else {{ Write-Error $_.Exception.Message }} }}",
        port_name.replace("'", "''"),
        ip_address.replace("'", "''")
    );
    let port_add_result = super::ps::run_powershell(&port_add_script);
    
    match port_add_result {
        Ok(port_result) => {
            let port_stdout = decode_windows_string(&port_result.stdout);
            let port_stderr = decode_windows_string(&port_result.stderr);
            
            // 检查是否成功或端口已存在
            let port_created = port_result.status.success() && port_stdout.contains("PortSuccess");
            let port_exists = port_stdout.contains("PortExists")
                || port_stderr.contains("already exists")
                || port_stderr.contains("已存在");
            
            if !port_created && !port_exists {
                // 端口添加失败
                return Err(InstallError::PortAddFailedModern {
                    stdout: port_stdout,
                    stderr: port_stderr,
                });
            }
            
            // 确定端口是新建还是已存在
            let outcome = if port_created {
                eprintln!("[DEBUG] 端口创建成功: {}", port_stdout);
                PortAddOutcome::Created
            } else {
                eprintln!("[DEBUG] 端口已存在: {}", port_stdout);
                PortAddOutcome::AlreadyExists
            };
            
            // 验证端口确实存在（重试几次，因为端口创建可能需要时间）
            match verify_printer_port(port_name) {
                Ok(verified) => {
                    if !verified {
                        eprintln!("[DEBUG] 警告：端口验证失败，但继续尝试添加打印机");
                    }
                    Ok(outcome)
                }
                Err(e) => {
                    // 验证失败，但如果是端口已存在的情况，可能只是验证命令失败，继续执行
                    if matches!(outcome, PortAddOutcome::AlreadyExists) {
                        eprintln!("[DEBUG] 警告：端口验证失败，但端口可能已存在，继续尝试添加打印机");
                        Ok(outcome)
                    } else {
                        // 端口刚创建但验证失败，返回验证错误
                        Err(e)
                    }
                }
            }
        }
        Err(e) => {
            Err(InstallError::PowerShellFailed {
                step: "add_printer_port_modern",
                stderr: e,
            })
        }
    }
}

/// 使用现代方式添加打印机（使用指定的驱动）
fn add_printer_with_driver_modern(name: &str, port_name: &str, ip_address: &str, driver_name: &str) -> InstallResult {
    eprintln!("[DEBUG] 使用驱动 '{}' 安装打印机 '{}' 到端口 '{}'", driver_name, name, port_name);
    
    // 使用指定的驱动添加打印机
    let printer_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; try {{ Add-Printer -Name '{}' -DriverName '{}' -PortName '{}' -ErrorAction Stop; Write-Output 'Success' }} catch {{ Write-Error $_.Exception.Message }}",
        name.replace("'", "''"),
        driver_name.replace("'", "''"),
        port_name.replace("'", "''")
    );
    let printer_output = super::ps::run_powershell(&printer_script);
    
    match printer_output {
        Ok(printer_result) => {
            let printer_stdout = decode_windows_string(&printer_result.stdout);
            let printer_stderr = decode_windows_string(&printer_result.stderr);
            
            if printer_result.status.success() || printer_stdout.contains("Success") {
                InstallResult {
                    success: true,
                    message: format!("打印机 {} ({}) 安装成功", name, ip_address),
                    method: Some("Add-Printer".to_string()),
                    stdout: Some(printer_stdout),
                    stderr: Some(printer_stderr),
                }
            } else {
                // 失败时包含诊断信息：驱动名、端口名、PowerShell stderr
                let mut stderr_parts = Vec::new();
                let error = InstallError::PrinterInstallFailedModern {
                    stderr: printer_stderr.clone(),
                };
                let base_stderr = error.format_stderr_with_code(Some(printer_stderr.clone())).unwrap_or_default();
                stderr_parts.push(base_stderr);
                stderr_parts.push(format!("Driver used: {}", driver_name));
                stderr_parts.push(format!("Port: {}", port_name));
                
                InstallResult {
                    success: false,
                    message: error.to_user_message(),
                    method: Some("Add-Printer".to_string()),
                    stdout: Some(printer_stdout),
                    stderr: Some(stderr_parts.join(" | ")),
                }
            }
        }
        Err(e) => {
            // PowerShell 执行失败，包含诊断信息
            let mut stderr_parts = Vec::new();
            let error = InstallError::PowerShellFailed {
                step: "add_printer_with_driver_modern",
                stderr: e.clone(),
            };
            let base_stderr = error.format_stderr_with_code(Some(e)).unwrap_or_default();
            stderr_parts.push(base_stderr);
            stderr_parts.push(format!("Driver used: {}", driver_name));
            stderr_parts.push(format!("Port: {}", port_name));
            
            InstallResult {
                success: false,
                message: error.to_user_message(),
                method: Some("Add-Printer".to_string()),
                stdout: None,
                stderr: Some(stderr_parts.join(" | ")),
            }
        }
    }
}

/// 将 VBS 脚本写入临时文件
fn write_vbs_script_to_temp() -> Result<std::path::PathBuf, InstallError> {
    // 将嵌入的 VBS 脚本写入临时文件
    // 重要：直接写入原始字节，不要进行编码转换，因为 VBScript 需要 ANSI/GBK 编码
    let temp_dir = std::env::temp_dir();
    let script_path = temp_dir.join("prnport.vbs");
    
    // 直接写入原始字节（保持原始编码，ANSI/GBK）
    let mut file = fs::File::create(&script_path)
        .map_err(|e| InstallError::FileOperationFailed {
            step: "write_vbs_script_to_temp",
            operation: "创建临时脚本文件",
            error: e.to_string(),
        })?;
    file.write_all(PRNPORT_VBS_BYTES)
        .map_err(|e| InstallError::FileOperationFailed {
            step: "write_vbs_script_to_temp",
            operation: "写入脚本内容",
            error: e.to_string(),
        })?;
    file.sync_all()
        .map_err(|e| InstallError::FileOperationFailed {
            step: "write_vbs_script_to_temp",
            operation: "同步脚本文件",
            error: e.to_string(),
        })?;
    drop(file); // 确保文件已关闭
    
    Ok(script_path)
}

/// 使用 VBS 脚本方式添加打印机端口
fn add_printer_port_vbs(script_path: &std::path::Path, port_name: &str, ip_address: &str) -> Result<InstallResult, InstallResult> {
    // 使用 cscript 运行 prnport.vbs 脚本添加端口（隐藏窗口）
    // 参数参考：cscript prnport.vbs -a -r IP_192.168.x.x -h 192.168.x.x -o raw
    // 注意：移除 //B 参数以便捕获错误信息
    let script_path_str = script_path.to_str().unwrap();
    let output = super::cmd::run_command("cscript", &[
        "//NoLogo",  // 不显示脚本横幅
        script_path_str,
        "-a",        // 添加端口
        "-r",        // 端口名
        port_name,   // 端口名称
        "-h",        // IP地址
        ip_address,  // IP地址值
        "-o",        // 输出类型
        "raw"        // raw 类型
    ]);
    
    match output {
        Ok(result) => {
            // 执行完毕后删除临时文件
            let _ = std::fs::remove_file(script_path);
            
            let stdout = decode_windows_string(&result.stdout);
            let stderr = decode_windows_string(&result.stderr);
            
            if result.status.success() {
                Ok(InstallResult {
                    success: true,
                    message: String::new(),
                    method: None,
                    stdout: Some(stdout),
                    stderr: Some(stderr),
                })
            } else {
                // 组合详细的错误信息
                let error_details = if stderr.trim().is_empty() && stdout.trim().is_empty() {
                    format!("cscript 退出代码 {}", result.status.code().unwrap_or(-1))
                } else {
                    format!("错误输出: {} | 标准输出: {}", 
                        if stderr.trim().is_empty() { "无" } else { &stderr },
                        if stdout.trim().is_empty() { "无" } else { &stdout }
                    )
                };
                
                let exit_code = result.status.code().unwrap_or(-1);
                let error = InstallError::PortAddFailedVbs {
                    error_details: error_details.clone(),
                    exit_code,
                    stdout: stdout.clone(),
                    stderr: stderr.clone(),
                };
                
                Err(InstallResult {
                    success: false,
                    message: error.to_user_message(),
                    method: Some("VBS".to_string()),
                    stdout: Some(stdout),
                    stderr: error.format_stderr_with_code(Some(stderr)),
                })
            }
        }
        Err(e) => {
            // 执行失败时也删除临时文件
            let _ = std::fs::remove_file(script_path);
            
            // 检查脚本文件是否存在
            let script_exists = script_path.exists();
            let script_info = if script_exists {
                format!("脚本文件存在，大小 {} 字节", 
                    std::fs::metadata(script_path)
                        .map(|m| m.len())
                        .unwrap_or(0)
                )
            } else {
                "脚本文件不存在".to_string()
            };
            
            let error = InstallError::VbsScriptFailed {
                error: e,
                script_info: script_info.clone(),
            };
            
            Err(InstallResult {
                success: false,
                message: error.to_user_message(),
                method: Some("VBS".to_string()),
                stdout: None,
                stderr: error.format_stderr_with_code(None),
            })
        }
    }
}

/// 使用 VBS 方式添加打印机（使用指定的驱动）
fn add_printer_with_driver_vbs(name: &str, port_name: &str, ip_address: &str, driver_name: &str) -> InstallResult {
    eprintln!("[DEBUG] 使用驱动 '{}' 安装打印机 '{}' 到端口 '{}' (VBS方式)", driver_name, name, port_name);
    
    // 端口添加成功，现在使用 PowerShell Add-Printer 安装打印机
    let ps_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; try {{ Add-Printer -Name '{}' -DriverName '{}' -PortName '{}' -ErrorAction Stop; Write-Output 'Success' }} catch {{ Write-Error $_.Exception.Message }}",
        name.replace("'", "''"),
        driver_name.replace("'", "''"),
        port_name.replace("'", "''")
    );
    let ps_output = super::ps::run_powershell(&ps_script);
    
    match ps_output {
        Ok(ps_result) => {
            let ps_stdout = decode_windows_string(&ps_result.stdout);
            let ps_stderr = decode_windows_string(&ps_result.stderr);
            
            if ps_result.status.success() || ps_stdout.contains("Success") {
                InstallResult {
                    success: true,
                    message: format!("打印机 {} ({}) 安装成功", name, ip_address),
                    method: Some("VBS".to_string()),
                    stdout: Some(ps_stdout),
                    stderr: Some(ps_stderr),
                }
            } else {
                // 失败时包含诊断信息：驱动名、端口名、PowerShell stderr
                let mut stderr_parts = Vec::new();
                let error = InstallError::PrinterInstallFailedVbs {
                    stderr: ps_stderr.clone(),
                };
                let base_stderr = error.format_stderr_with_code(Some(ps_stderr.clone())).unwrap_or_default();
                stderr_parts.push(base_stderr);
                stderr_parts.push(format!("Driver used: {}", driver_name));
                stderr_parts.push(format!("Port: {}", port_name));
                
                InstallResult {
                    success: false,
                    message: error.to_user_message(),
                    method: Some("VBS".to_string()),
                    stdout: Some(ps_stdout),
                    stderr: Some(stderr_parts.join(" | ")),
                }
            }
        }
        Err(e) => {
            // PowerShell 执行失败，包含诊断信息
            let mut stderr_parts = Vec::new();
            let error = InstallError::PowerShellFailed {
                step: "add_printer_with_driver_vbs",
                stderr: e.clone(),
            };
            let base_stderr = error.format_stderr_with_code(Some(e)).unwrap_or_default();
            stderr_parts.push(base_stderr);
            stderr_parts.push(format!("Driver used: {}", driver_name));
            stderr_parts.push(format!("Port: {}", port_name));
            
            InstallResult {
                success: false,
                message: error.to_user_message(),
                method: Some("VBS".to_string()),
                stdout: None,
                stderr: Some(stderr_parts.join(" | ")),
            }
        }
    }
}

// ============================================================================
// 主函数
// ============================================================================

/// 驱动安装策略枚举
#[derive(Debug, Clone, Copy)]
enum DriverInstallPolicy {
    /// 总是安装/更新 INF 驱动（稳定）
    Always,
    /// 若系统已存在驱动则跳过 INF（更快，可能版本不一致）
    ReuseIfInstalled,
}

impl DriverInstallPolicy {
    fn from_str(s: Option<&str>) -> Self {
        match s {
            Some("reuse_if_installed") => DriverInstallPolicy::ReuseIfInstalled,
            _ => DriverInstallPolicy::Always,  // 默认值
        }
    }
}

/// Windows 平台打印机安装入口
/// 
/// 根据 Windows 版本自动选择安装方式：
/// - Windows 10+ (构建号 >= 10240): 使用 Add-PrinterPort + Add-Printer
/// - Windows 7/8 (构建号 < 10240): 使用 VBS 脚本 + Add-Printer
#[allow(non_snake_case)]
pub async fn install_printer_windows(
    name: String,
    path: String,
    driverPath: Option<String>,
    #[allow(unused_variables)] model: Option<String>,
    driverInstallPolicy: Option<String>,  // 驱动安装策略："always" | "reuse_if_installed"
) -> Result<InstallResult, String> {
    
    // 解析驱动安装策略
    let policy = DriverInstallPolicy::from_str(driverInstallPolicy.as_deref());
    eprintln!("[INFO] 驱动安装策略: {:?}", policy);
    
    // 步骤0：从配置文件中查找匹配的 printer item，提取 driver_names（避免生命周期问题）
    let driver_names_option = match crate::load_local_config() {
        Ok((config, _)) => {
            // 在所有 areas 中查找匹配的 printer item（通过 name 或 path 匹配）
            let mut matched_driver_names: Option<Vec<String>> = None;
            for area in &config.areas {
                for printer in &area.printers {
                    if printer.name == name || printer.path == path {
                        // 提取 driver_names 的副本，避免生命周期问题
                        matched_driver_names = printer.driver_names.clone();
                        break;
                    }
                }
                if matched_driver_names.is_some() {
                    break;
                }
            }
            matched_driver_names
        }
        Err(e) => {
            // 配置文件加载失败，不进行校验（保持向后兼容，允许通过其他方式安装）
            eprintln!("[WARN] 无法加载配置文件: {}，跳过配置校验", e);
            None
        }
    };
    
    // 步骤0.5：检查是否使用 PrintUIEntry /if 路径
    // 当 driver_path 和 model 都存在时，使用 PrintUIEntry /if 安装（同时导入驱动并创建打印机）
    if let Some(driver_path_str) = &driverPath {
        if !driver_path_str.trim().is_empty() {
            if let Some(model_str) = &model {
                if !model_str.trim().is_empty() {
                    // 使用 PrintUIEntry /if 路径
                    eprintln!("[INFO] 检测到 driver_path 和 model，使用 PrintUIEntry /if 安装路径");
                    
                    // 解析 driver_path（相对于应用目录）
                    let inf_path = match resolve_driver_path(driver_path_str) {
                        Ok(path) => path,
                        Err(e) => {
                            return Ok(InstallResult {
                                success: false,
                                message: e.to_user_message(),
                                method: Some("PrintUIEntry".to_string()),
                                stdout: None,
                                stderr: e.format_stderr_with_code(None),
                            });
                        }
                    };
                    
                    // 从路径中提取 IP 地址（格式：\\192.168.x.x）
                    let ip_address = path.trim_start_matches("\\\\").to_string();
                    
                    // 端口名格式：IP_IP地址（用下划线替换点）
                    let port_name = format!("IP_{}", ip_address.replace(".", "_"));
                    
                    // 检测 Windows 构建号来判断是否支持 Add-PrinterPort
                    let windows_build = get_windows_build_number().unwrap_or(0);
                    let use_modern_method = if windows_build == 0 {
                        eprintln!("[DEBUG] 构建号检测失败，默认使用现代方法（Add-PrinterPort）");
                        true
                    } else {
                        windows_build >= 10240
                    };
                    
                    eprintln!("[DEBUG] Windows 构建号: {}, 使用现代方法: {}", windows_build, use_modern_method);
                    
                    // 删除旧打印机（如果存在）
                    delete_existing_printer(&name);
                    
                    // 创建端口
                    if use_modern_method {
                        // Windows 10+ 使用 Add-PrinterPort
                        match add_printer_port_modern(&port_name, &ip_address) {
                            Err(e) => {
                                let (stdout, stderr) = e.get_output();
                                return Ok(InstallResult {
                                    success: false,
                                    message: e.to_user_message(),
                                    method: Some("PrintUIEntry".to_string()),
                                    stdout,
                                    stderr: e.format_stderr_with_code(stderr),
                                });
                            }
                            Ok(_) => {
                                eprintln!("[DEBUG] 端口创建成功，继续使用 PrintUIEntry 安装打印机");
                            }
                        }
                    } else {
                        // Windows 7/8 使用 VBS 脚本
                        let script_path = write_vbs_script_to_temp()
                            .map_err(|e| e.to_user_message())?;
                        
                        match add_printer_port_vbs(&script_path, &port_name, &ip_address) {
                            Err(result) => return Ok(result),
                            Ok(_) => {
                                eprintln!("[DEBUG] 端口创建成功（VBS），继续使用 PrintUIEntry 安装打印机");
                            }
                        }
                    }
                    
                    // 使用 PrintUIEntry /if 安装打印机（同时导入驱动）
                    match install_printer_with_printui(&name, &inf_path, &port_name, model_str) {
                        Ok(result) => {
                            return Ok(result);
                        }
                        Err(e) => {
                            let (stdout, stderr) = e.get_output();
                            return Ok(InstallResult {
                                success: false,
                                message: e.to_user_message(),
                                method: Some("PrintUIEntry".to_string()),
                                stdout,
                                stderr: e.format_stderr_with_code(stderr),
                            });
                        }
                    }
                }
            }
            
            // driver_path 存在但 model 缺失
            let error = InstallError::InvalidConfig {
                reason: "driver_path 需要同时提供 model 才能使用 PrintUIEntry /if 安装。请更新 printer_config.json 添加 model 字段。".to_string(),
            };
            return Ok(InstallResult {
                success: false,
                message: error.to_user_message(),
                method: Some("PrintUIEntry".to_string()),
                stdout: None,
                stderr: error.format_stderr_with_code(None),
            });
        }
    }
    
    // 步骤1：根据策略决定是否先安装 INF 驱动（无 driver_path 或 driver_path 存在但 model 缺失的场景）
    let mut inf_installed = false;
    
    if let Some(driver_path_str) = &driverPath {
        if !driver_path_str.trim().is_empty() {
            match policy {
                DriverInstallPolicy::Always => {
                    // 策略：总是安装 INF 驱动
                    eprintln!("[DEBUG] 策略: Always - 检测到 driver_path: {}，开始安装 INF 驱动", driver_path_str);
                    
                    // 解析 driver_path（相对于应用目录）
                    let inf_path = match resolve_driver_path(driver_path_str) {
                        Ok(path) => path,
                        Err(e) => {
                            return Ok(InstallResult {
                                success: false,
                                message: e.to_user_message(),
                                method: Some("install_inf_driver".to_string()),
                                stdout: None,
                                stderr: e.format_stderr_with_code(None),
                            });
                        }
                    };
                    
                    // 安装 INF 驱动
                    // 需要 driver_names 用于验证安装是否成功
                    let driver_names_for_install = driver_names_option.as_ref()
                        .map(|names| names.as_slice())
                        .unwrap_or(&[]);
                    
                    match install_inf_driver(&inf_path, driver_names_for_install) {
                        Ok(()) => {
                            eprintln!("[DEBUG] INF 驱动安装成功");
                            inf_installed = true;
                        }
                        Err(e) => {
                            // INF 安装失败，直接终止安装流程
                            let (stdout, stderr) = e.get_output();
                            return Ok(InstallResult {
                                success: false,
                                message: e.to_user_message(),
                                method: Some("install_inf_driver".to_string()),
                                stdout,
                                stderr: e.format_stderr_with_code(stderr),
                            });
                        }
                    }
                }
                DriverInstallPolicy::ReuseIfInstalled => {
                    // 策略：先尝试选择已安装的驱动，如果找不到再安装 INF
                    eprintln!("[DEBUG] 策略: ReuseIfInstalled - 先尝试选择已安装的驱动");
                    // 这一步将在步骤2中执行
                }
            }
        }
    }
    
    // 步骤2：校验 driver_names 字段并选择已安装的驱动
    let selected_driver_name = match driver_names_option {
        Some(names) => {
            // 检查 driver_names 是否存在且非空
            // 检查数组是否为空
            if names.is_empty() {
                let error = InstallError::InvalidConfig {
                    reason: "配置缺少 driver_names，请更新 printer_config.json".to_string(),
                };
                return Ok(InstallResult {
                    success: false,
                    message: error.to_user_message(),
                    method: None,
                    stdout: None,
                    stderr: error.format_stderr_with_code(None),
                });
            }
            // 检查数组中的元素是否全部为空白（trim 后为空）
            let all_empty = names.iter().all(|n| n.trim().is_empty());
            if all_empty {
                let error = InstallError::InvalidConfig {
                    reason: "driver_names 不能为空".to_string(),
                };
                return Ok(InstallResult {
                    success: false,
                    message: error.to_user_message(),
                    method: None,
                    stdout: None,
                    stderr: error.format_stderr_with_code(None),
                });
            }
            
            // 使用 driver_names 选择已安装的驱动
            let selected_driver_name_result = match select_installed_driver_name(&names) {
                Ok(driver_name) => {
                    // 如果策略是 ReuseIfInstalled 且找到了驱动，跳过 INF 安装
                    if matches!(policy, DriverInstallPolicy::ReuseIfInstalled) && !inf_installed {
                        eprintln!("[INFO] 策略: ReuseIfInstalled - 找到已安装的驱动: {}，跳过 INF 安装", driver_name);
                    }
                    Ok(driver_name)
                }
                Err((e, ps_stderr)) => {
                    // 如果策略是 ReuseIfInstalled 且未找到驱动，尝试安装 INF 后再选择
                    if matches!(policy, DriverInstallPolicy::ReuseIfInstalled) && !inf_installed {
                        if let Some(driver_path_str) = &driverPath {
                            if !driver_path_str.trim().is_empty() {
                                eprintln!("[INFO] 策略: ReuseIfInstalled - 未找到已安装的驱动，开始安装 INF 驱动");
                                
                                // 解析 driver_path（相对于应用目录）
                                let inf_path = match resolve_driver_path(driver_path_str) {
                                    Ok(path) => path,
                                    Err(e) => {
                                        // INF 路径解析失败，返回错误
                                        let mut stderr_parts = Vec::new();
                                        let error_code_msg = e.format_stderr_with_code(None).unwrap_or_default();
                                        if !error_code_msg.is_empty() {
                                            stderr_parts.push(error_code_msg);
                                        }
                                        let candidates_str = names.join(",");
                                        stderr_parts.push(format!("Candidates: {}", candidates_str));
                                        let stderr_msg = stderr_parts.join(" | ");
                                        
                                        return Ok(InstallResult {
                                            success: false,
                                            message: e.to_user_message(),
                                            method: None,
                                            stdout: None,
                                            stderr: Some(stderr_msg),
                                        });
                                    }
                                };
                                
                                // 安装 INF 驱动
                                // install_inf_driver 内部已经验证了 driver_names，如果成功则说明驱动已注册
                                match install_inf_driver(&inf_path, &names) {
                                    Ok(()) => {
                                        eprintln!("[DEBUG] INF 驱动安装成功");
                                        inf_installed = true;
                                        
                                        // install_inf_driver 内部已经验证了 driver_names，再次选择确认
                                        match select_installed_driver_name(&names) {
                                            Ok(driver_name) => {
                                                eprintln!("[INFO] INF 安装后找到驱动: {}", driver_name);
                                                // 继续流程，使用找到的驱动
                                                Ok(driver_name)
                                            }
                                            Err((e2, ps_stderr2)) => {
                                                // 理论上不应该到这里，因为 install_inf_driver 已经验证过了
                                                // 但为了安全起见，仍然返回错误
                                                Err((e2, ps_stderr2))
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        // INF 安装失败，返回错误
                                        let (stdout, stderr) = e.get_output();
                                        let mut stderr_parts = Vec::new();
                                        
                                        // 添加错误码前缀的 stderr
                                        let error_stderr = e.format_stderr_with_code(stderr).unwrap_or_default();
                                        if !error_stderr.is_empty() {
                                            stderr_parts.push(error_stderr);
                                        }
                                        
                                        let candidates_str = names.join(",");
                                        stderr_parts.push(format!("Candidates: {}", candidates_str));
                                        if let Some(ps_err) = ps_stderr {
                                            if !ps_err.trim().is_empty() {
                                                stderr_parts.push(format!("PowerShell stderr: {}", ps_err));
                                            }
                                        }
                                        let stderr_msg = stderr_parts.join(" | ");
                                        
                                        return Ok(InstallResult {
                                            success: false,
                                            message: e.to_user_message(),
                                            method: Some("install_inf_driver".to_string()),
                                            stdout,
                                            stderr: Some(stderr_msg),
                                        });
                                    }
                                }
                            } else {
                                Err((e, ps_stderr))
                            }
                        } else {
                            Err((e, ps_stderr))
                        }
                    } else {
                        Err((e, ps_stderr))
                    }
                }
            };
            
            // 处理驱动选择结果
            match selected_driver_name_result {
                Ok(driver_name) => Some(driver_name),
                Err((e, ps_stderr)) => {
                    // 如果 INF 已安装但找不到驱动，返回明确错误
                    let mut stderr_parts = Vec::new();
                    
                    // 添加错误码前缀
                    let error_code_msg = e.format_stderr_with_code(None).unwrap_or_default();
                    if !error_code_msg.is_empty() {
                        stderr_parts.push(error_code_msg);
                    }
                    
                    // 如果 driver_path 存在且 INF 已安装，添加特殊提示
                    if inf_installed {
                        stderr_parts.push("INF 安装完成但驱动未注册成功，请检查 INF 文件是否正确".to_string());
                    }
                    
                    // 添加候选列表
                    let candidates_str = names.join(",");
                    stderr_parts.push(format!("Candidates: {}", candidates_str));
                    
                    // 添加 PowerShell stderr（如果存在）
                    if let Some(ps_err) = ps_stderr {
                        if !ps_err.trim().is_empty() {
                            stderr_parts.push(format!("PowerShell stderr: {}", ps_err));
                        }
                    }
                    
                    let stderr_msg = stderr_parts.join(" | ");
                    
                    return Ok(InstallResult {
                        success: false,
                        message: e.to_user_message(),
                        method: None,
                        stdout: None,
                        stderr: Some(stderr_msg),
                    });
                }
            }
        }
        None => {
            // 如果没有找到匹配的 printer item，不进行校验（可能是通过其他方式安装，保持向后兼容）
            None
        }
    };
    
    // 如果没有选中的驱动（配置文件加载失败或未找到匹配项），直接返回错误
    let selected_driver = match selected_driver_name {
        Some(driver) => driver,
        None => {
            return Err("无法从配置中获取 driver_names，请确保配置文件存在且包含该打印机的配置".to_string());
        }
    };
    
    // 从路径中提取 IP 地址（格式：\\192.168.x.x）
    let ip_address = path.trim_start_matches("\\\\").to_string();
    
    // 端口名格式：IP_IP地址（用下划线替换点）
    let port_name = format!("IP_{}", ip_address.replace(".", "_"));
    
    // 检测 Windows 构建号来判断是否支持 Add-PrinterPort
    // Windows 10 (10240+) 和 Windows 11 (22000+) 都支持 Add-PrinterPort
    let windows_build = get_windows_build_number().unwrap_or(0);
    
    // 如果构建号为 0（检测失败），默认使用现代方法（因为可能是 Windows 10+）
    // 只有在明确检测到旧版本 Windows（构建号 < 10240）时才使用 VBS
    let use_modern_method = if windows_build == 0 {
        eprintln!("[DEBUG] 构建号检测失败，默认使用现代方法（Add-PrinterPort）");
        true // 默认使用现代方法
    } else {
        windows_build >= 10240 // Windows 10+ 使用新方法（包括 Windows 11）
    };
    
    // 调试日志：输出检测到的构建号和选择的安装方式
    eprintln!("[DEBUG] Windows 构建号: {}, 使用现代方法: {}", windows_build, use_modern_method);
    eprintln!("[INFO] 本次安装是否执行了 INF 安装: {}", inf_installed);
    
    // 步骤1：删除旧打印机（如果存在）静默模式，忽略错误，隐藏窗口
    delete_existing_printer(&name);
    
    // 根据 Windows 版本选择安装方式
    eprintln!("[DEBUG] 准备选择安装方式，use_modern_method = {}", use_modern_method);
    if use_modern_method {
        eprintln!("[DEBUG] 使用 Add-PrinterPort 方式安装");
        // Windows 10+ 使用 Add-PrinterPort + Add-Printer（现代方式）
        // 步骤1：添加打印机端口（如果不存在则创建，如果已存在则忽略错误）
        match add_printer_port_modern(&port_name, &ip_address) {
            Err(e) => {
                // 端口添加失败，构造 InstallResult 并返回
                let (stdout, stderr) = e.get_output();
                return Ok(InstallResult {
                    success: false,
                    message: e.to_user_message(),
                    method: Some("Add-Printer".to_string()),
                    stdout,
                    stderr: e.format_stderr_with_code(stderr),
                });
            }
            Ok(outcome) => {
                match outcome {
                    PortAddOutcome::Created => {
                        eprintln!("[DEBUG] 端口创建成功，继续安装打印机");
                    }
                    PortAddOutcome::AlreadyExists => {
                        eprintln!("[DEBUG] 端口已存在，继续安装打印机");
                    }
                }
            }
        }
        
        // 步骤2：使用选中的驱动添加打印机
        Ok(add_printer_with_driver_modern(&name, &port_name, &ip_address, &selected_driver))
    } else {
        eprintln!("[DEBUG] 使用 VBS 脚本方式安装");
        // Windows 7/8 使用 VBS 脚本方式（传统方式）
        // 步骤1：将嵌入的 VBS 脚本写入临时文件
        let script_path = write_vbs_script_to_temp()
            .map_err(|e| e.to_user_message())?;
        
        // 步骤2：使用 cscript 运行 prnport.vbs 脚本添加端口
        match add_printer_port_vbs(&script_path, &port_name, &ip_address) {
            Err(result) => Ok(result),
            Ok(_) => {
                // 步骤3：端口添加成功，现在使用 PowerShell Add-Printer 安装打印机
                Ok(add_printer_with_driver_vbs(&name, &port_name, &ip_address, &selected_driver))
            }
        }
    }
}

