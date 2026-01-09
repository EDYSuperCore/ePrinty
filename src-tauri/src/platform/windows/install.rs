// Windows 平台打印机安装模块
// 该文件是 Windows 安装入口实现，分为 Add-Printer 与 VBS 分支

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};


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
// Package 安装分支
// ============================================================================

/// Package 安装分支（install_mode=package）
/// 
/// 阶段 A：先 stub 返回，验证分流生效
/// 阶段 B：执行真实的 pnputil stage
/// 注册打印机驱动（Add-PrinterDriver + Get-PrinterDriver 验证）
/// 
/// # 参数
/// - `driver_name`: 驱动名称（来自 driver_names 配置）
/// - `published_inf_path`: 已发布的 INF 文件路径（C:\Windows\INF\oemXXX.inf）
/// - `dry_run`: 是否为 dryRun 模式
/// 
/// # 返回
/// - `Ok(())`: 注册成功
/// - `Err(String)`: 注册失败，包含错误信息
fn register_printer_driver(
    driver_name: &str,
    published_inf_path: &str,
    dry_run: bool,
) -> Result<(), String> {
    use crate::platform::windows::encoding::decode_windows_string;
    
    eprintln!("[RegisterPrinterDriver] start name=\"{}\" inf=\"{}\" dryRun={}", 
        driver_name, published_inf_path, dry_run);
    
    // 如果是 dryRun 模式，直接返回成功
    if dry_run {
        eprintln!("[RegisterPrinterDriver] success (dryRun mode)");
        return Ok(());
    }
    
    // 先验证驱动是否已存在（幂等检查）
    let verify_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-PrinterDriver -Name '{}' -ErrorAction SilentlyContinue | Select-Object -First 1 -ExpandProperty Name",
        driver_name.replace("'", "''")
    );
    
    match super::ps::run_powershell(&verify_script) {
        Ok(output) => {
            let stdout = decode_windows_string(&output.stdout);
            let trimmed_stdout = stdout.trim();
            
            // 如果驱动已存在，视为成功（幂等）
            if !trimmed_stdout.is_empty() && trimmed_stdout == driver_name {
                eprintln!("[RegisterPrinterDriver] success (driver already exists)");
                return Ok(());
            }
        }
        Err(_) => {
            // 验证失败，继续尝试注册
        }
    }
    
    // 执行 Add-PrinterDriver
    let add_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Add-PrinterDriver -Name '{}' -InfPath '{}' -ErrorAction Stop",
        driver_name.replace("'", "''"),
        published_inf_path.replace("'", "''")
    );
    
    match super::ps::run_powershell(&add_script) {
        Ok(output) => {
            let stdout = decode_windows_string(&output.stdout);
            let stderr = decode_windows_string(&output.stderr);
            let exit_code = output.status.code();
            
            // 再次验证驱动是否已注册
            match super::ps::run_powershell(&verify_script) {
                Ok(verify_output) => {
                    let verify_stdout = decode_windows_string(&verify_output.stdout);
                    let trimmed_verify = verify_stdout.trim();
                    
                    if !trimmed_verify.is_empty() && trimmed_verify == driver_name {
                        eprintln!("[RegisterPrinterDriver] success");
                        return Ok(());
                    }
                }
                Err(e) => {
                    eprintln!("[RegisterPrinterDriver] failed (verification failed): {}", e);
                }
            }
            
            // 构建 combined_output（用于后续检查）
            let combined_output = if !stderr.is_empty() {
                format!("{}\n{}", stdout, stderr)
            } else {
                stdout.clone()
            };
            
            // 如果验证失败，但 Add-PrinterDriver 没有报错，可能是驱动已存在但名称不匹配
            if output.status.success() {
                // 检查是否是驱动已存在的情况（幂等）
                let output_lower = combined_output.to_lowercase();
                
                if output_lower.contains("already exists") || output_lower.contains("已存在") {
                    eprintln!("[RegisterPrinterDriver] success (driver already exists, name may not match exactly)");
                    return Ok(());
                }
            }
            
            // 验证失败，使用已构建的 combined_output
            
            Err(format!(
                "驱动包已导入，但未注册为可用打印驱动；请检查 driver_names 是否匹配该驱动包\n\nAdd-PrinterDriver 输出:\n{}\n\n退出码: {:?}",
                combined_output, exit_code
            ))
        }
        Err(e) => {
            // Add-PrinterDriver 执行失败
            let error_lower = e.to_lowercase();
            
            // 检查是否是权限错误
            if error_lower.contains("access denied") 
                || error_lower.contains("0x80070005") 
                || error_lower.contains("拒绝访问")
                || error_lower.contains("需要提升")
            {
                eprintln!("[RegisterPrinterDriver] failed error=\"{}\" (permission error)", e);
                return Err(format!("Add-PrinterDriver 失败（权限错误）。请以管理员身份运行\n\n错误详情:\n{}", e));
            }
            
            // 检查是否是参数错误
            if error_lower.contains("0x80070057") || error_lower.contains("invalid parameter") {
                eprintln!("[RegisterPrinterDriver] failed error=\"{}\" (parameter error)", e);
                return Err(format!(
                    "Add-PrinterDriver 失败（参数错误）。请检查 InfPath 和 DriverName 是否匹配\n\nInfPath: {}\nDriverName: {}\n\n错误详情:\n{}",
                    published_inf_path, driver_name, e
                ));
            }
            
            // 其他错误
            eprintln!("[RegisterPrinterDriver] failed error=\"{}\"", e);
            Err(format!("Add-PrinterDriver 失败\n\n错误详情:\n{}", e))
        }
    }
}

async fn install_printer_package_branch(
    name: String,
    path: String,
    driverPath: Option<String>,
    _model: Option<String>,
    dry_run: bool,
) -> Result<InstallResult, String> {
    eprintln!("[PackageBranch] start printer=\"{}\" dryRun={}", name, dry_run);
    
    // 从配置中获取 driver_names
    let driver_names = match crate::load_local_config() {
        Ok((config, _)) => {
            let mut matched_driver_names: Option<Vec<String>> = None;
            for area in &config.areas {
                for printer in &area.printers {
                    if printer.name == name || printer.path == path {
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
            eprintln!("[WARN] 无法加载配置文件: {}，将无法注册驱动", e);
            None
        }
    };
    
    let target_driver_name = match &driver_names {
        Some(names) if !names.is_empty() => {
            // 使用第一个 driver_name
            names[0].clone()
        }
        _ => {
            return Ok(InstallResult {
                success: false,
                message: "Package 安装模式需要提供 driver_names（驱动名称列表）".to_string(),
                method: Some("Package".to_string()),
                stdout: None,
                stderr: Some("driver_names 缺失或为空".to_string()),
            });
        }
    };
    
    // 检查 driverPath
    let driver_path_str = match &driverPath {
        Some(path) if !path.trim().is_empty() => path.as_str(),
        _ => {
            return Ok(InstallResult {
                success: false,
                message: "Package 安装模式需要提供 driver_path（INF 文件路径）".to_string(),
                method: Some("Package".to_string()),
                stdout: None,
                stderr: Some("driver_path 缺失或为空".to_string()),
            });
        }
    };
    
    // 解析 driver_path（相对于应用目录）
    let inf_path = match resolve_driver_path(driver_path_str) {
        Ok(path) => path,
        Err(e) => {
            let stderr = e.format_stderr_with_code(None).unwrap_or_default();
            return Ok(InstallResult {
                success: false,
                message: format!("缺少 driver_path 或 INF 文件不存在: {}", e.to_user_message()),
                method: Some("Package".to_string()),
                stdout: None,
                stderr: Some(stderr),
            });
        }
    };
    
    // 检查 INF 文件是否存在
    if !inf_path.exists() {
        return Ok(InstallResult {
            success: false,
            message: format!("INF 文件不存在: {}", inf_path.display()),
            method: Some("Package".to_string()),
            stdout: None,
            stderr: Some(format!("文件不存在: {}", inf_path.display())),
        });
    }
    
    // 如果是 dryRun 模式，返回 stub 结果
    if dry_run {
        eprintln!("[PackageBranch] dryRun=true，返回 stub 结果");
        return Ok(InstallResult {
            success: true,
            message: "已命中 Package 安装分支（dryRun 模式，未执行真实安装）".to_string(),
            method: Some("Package".to_string()),
            stdout: None,
            stderr: None,
        });
    }
    
    // 阶段 B：真实执行 pnputil stage
    eprintln!("[PackageBranch] dryRun=false，执行真实 pnputil stage");
    eprintln!("[PackageBranch] inf_path=\"{}\"", inf_path.display());
    
    match stage_driver_package_windows(&inf_path) {
        Ok(stage_result) => {
            eprintln!("[PackageBranch] pnputil stage 成功 exit={:?}", stage_result.exit_code);
            
            // 从 pnputil 输出中提取 published name（oemXXX.inf）
            let published_name = extract_published_name(&stage_result.output_text);
            let published_name = match published_name {
                Some(name) => name,
                None => {
                    return Ok(InstallResult {
                        success: false,
                        message: "pnputil stage 成功，但无法从输出中提取 published name（oemXXX.inf）".to_string(),
                        method: Some("Package".to_string()),
                        stdout: Some(stage_result.output_text),
                        stderr: Some("无法解析 published name".to_string()),
                    });
                }
            };
            
            eprintln!("[PackageBranch] extracted published_name=\"{}\"", published_name);
            
            // 构建 published_inf_path
            let published_inf_path = format!(r"C:\Windows\INF\{}", published_name);
            
            // 步骤：注册打印机驱动
            match register_printer_driver(&target_driver_name, &published_inf_path, dry_run) {
                Ok(()) => {
                    eprintln!("[PackageBranch] RegisterPrinterDriver 成功");
                    
                    // 步骤：确保端口和队列存在
                    // 检测目标类型
                    let target_type = match detect_target_type(&path) {
                        Ok(t) => t,
                        Err(e) => {
                            return Ok(InstallResult {
                                success: false,
                                message: format!("无法识别目标路径格式: {}", e),
                                method: Some("Package".to_string()),
                                stdout: Some(stage_result.output_text),
                                stderr: Some(e),
                            });
                        }
                    };
                    
                    match target_type {
                        TargetType::TcpIpHost { host } => {
                            eprintln!("[PackageBranch] EnsurePrinterPort step=start inputs=host=\"{}\"", host);
                            
                            // 检测 Windows 版本以决定是否使用 VBS
                            let windows_build = get_windows_build_number().unwrap_or(0);
                            let is_legacy = windows_build > 0 && windows_build < 10240;
                            
                            // 确保端口存在
                            let port_name = match ensure_printer_port(&host, 9100, is_legacy) {
                                Ok(port) => {
                                    eprintln!("[PackageBranch] EnsurePrinterPort step=success port_name=\"{}\"", port);
                                    port
                                }
                                Err(e) => {
                                    eprintln!("[PackageBranch] EnsurePrinterPort step=failed error=\"{}\"", e);
                                    return Ok(InstallResult {
                                        success: false,
                                        message: format!("端口创建失败: {}", e),
                                        method: Some("Package".to_string()),
                                        stdout: Some(stage_result.output_text),
                                        stderr: Some(e),
                                    });
                                }
                            };
                            
                            // 确保队列存在
                            eprintln!("[PackageBranch] EnsurePrinterQueue step=start inputs=queue_name=\"{}\" driver_name=\"{}\" port_name=\"{}\"", 
                                name, target_driver_name, port_name);
                            
                            match ensure_printer_queue(&name, &target_driver_name, &port_name) {
                                Ok(()) => {
                                    eprintln!("[PackageBranch] EnsurePrinterQueue step=success");
                                    Ok(InstallResult {
                                        success: true,
                                        message: format!(
                                            "Package 安装完成\n\nPublished name: {}\nDriver name: {}\nPort name: {}\nQueue name: {}\n\npnputil 输出:\n{}",
                                            published_name, target_driver_name, port_name, name, stage_result.output_text
                                        ),
                                        method: Some("Package".to_string()),
                                        stdout: Some(stage_result.output_text),
                                        stderr: None,
                                    })
                                }
                                Err(e) => {
                                    eprintln!("[PackageBranch] EnsurePrinterQueue step=failed error=\"{}\"", e);
                                    Ok(InstallResult {
                                        success: false,
                                        message: format!("队列创建失败: {}", e),
                                        method: Some("Package".to_string()),
                                        stdout: Some(stage_result.output_text),
                                        stderr: Some(e),
                                    })
                                }
                            }
                        }
                        TargetType::SharedConnection { path: conn_path } => {
                            // 共享连接：直接使用 Add-Printer -ConnectionName
                            // 防御性检查：确保 connection_name 格式正确（至少包含两段）
                            let parts: Vec<&str> = conn_path.split('\\').filter(|s| !s.is_empty()).collect();
                            if parts.len() < 2 {
                                let evidence = format!("InvalidSharedConnectionName connection_name=\"{}\" parts_count={}", conn_path, parts.len());
                                eprintln!("[PackageBranch] EnsurePrinterQueue step=failed evidence=\"{}\"", evidence);
                                return Ok(InstallResult {
                                    success: false,
                                    message: format!(
                                        "无效的共享连接名称: \"{}\"。共享打印机路径必须为 \"\\\\server\\\\share\" 格式。\n\nEvidence: {}",
                                        conn_path, evidence
                                    ),
                                    method: Some("Package".to_string()),
                                    stdout: Some(stage_result.output_text),
                                    stderr: Some(evidence),
                                });
                            }
                            
                            eprintln!("[PackageBranch] EnsurePrinterQueue step=start inputs=connection_name=\"{}\" driver_name=\"{}\"", 
                                conn_path, target_driver_name);
                            
                            // 修复：使用 Where-Object 精确过滤，避免 Get-Printer -Name 的通配符匹配导致误判
                            let escaped_conn_path = conn_path.replace("'", "''");
                            let check_shared_script = format!(
                                "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $q = '{}'; $printer = Get-Printer -Name $q -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -eq $q }} | Select-Object -ExpandProperty Name",
                                escaped_conn_path
                            );
                            
                            let queue_exists = match super::ps::run_powershell(&check_shared_script) {
                                Ok(output) => {
                                    let stdout = decode_windows_string(&output.stdout);
                                    // 二次确认：验证返回的名称是否完全等于 conn_path
                                    !stdout.trim().is_empty() && stdout.trim() == conn_path
                                }
                                Err(e) => {
                                    let evidence = format!("check_shared_failed error=\"{}\"", e);
                                    eprintln!("[PackageBranch] EnsurePrinterQueue step=check_shared result=error evidence=\"{}\"", evidence);
                                    // 检查失败，继续尝试创建
                                    false
                                }
                            };
                            
                            if queue_exists {
                                eprintln!("[PackageBranch] EnsurePrinterQueue step=skipped action=reuse connection=\"{}\"", conn_path);
                                Ok(InstallResult {
                                    success: true,
                                    message: format!(
                                        "Package 安装完成（共享连接已存在）\n\nPublished name: {}\nDriver name: {}\nConnection: {}\n\npnputil 输出:\n{}",
                                        published_name, target_driver_name, conn_path, stage_result.output_text
                                    ),
                                    method: Some("Package".to_string()),
                                    stdout: Some(stage_result.output_text),
                                    stderr: None,
                                })
                            } else {
                                eprintln!("[PackageBranch] EnsurePrinterQueue step=create_shared inputs=connection_name=\"{}\"", conn_path);
                                let add_shared_script = format!(
                                    "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Add-Printer -ConnectionName '{}' -ErrorAction Stop",
                                    conn_path.replace("'", "''")
                                );
                                
                                match super::ps::run_powershell(&add_shared_script) {
                                    Ok(output) => {
                                        let stdout = decode_windows_string(&output.stdout);
                                        let stderr = decode_windows_string(&output.stderr);
                                        let exit_code = output.status.code();
                                        
                                        if exit_code == Some(0) {
                                            eprintln!("[PackageBranch] EnsurePrinterQueue step=success action=create connection=\"{}\"", conn_path);
                                            Ok(InstallResult {
                                                success: true,
                                                message: format!(
                                                    "Package 安装完成\n\nPublished name: {}\nDriver name: {}\nConnection: {}\n\npnputil 输出:\n{}",
                                                    published_name, target_driver_name, conn_path, stage_result.output_text
                                                ),
                                                method: Some("Package".to_string()),
                                                stdout: Some(stage_result.output_text),
                                                stderr: None,
                                            })
                                        } else {
                                            let evidence = format!("add_shared_failed stdout=\"{}\" stderr=\"{}\" exit_code={:?} connection_name=\"{}\"", 
                                                stdout, stderr, exit_code, conn_path);
                                            eprintln!("[PackageBranch] EnsurePrinterQueue step=failed evidence=\"{}\"", evidence);
                                            Ok(InstallResult {
                                                success: false,
                                                message: format!("共享连接创建失败: {}\n\n连接名称: {}\n\nEvidence: {}", stderr, conn_path, evidence),
                                                method: Some("Package".to_string()),
                                                stdout: Some(stage_result.output_text),
                                                stderr: Some(evidence),
                                            })
                                        }
                                    }
                                    Err(e) => {
                                        let evidence = format!("add_shared_command_failed error=\"{}\" connection_name=\"{}\"", e, conn_path);
                                        eprintln!("[PackageBranch] EnsurePrinterQueue step=failed evidence=\"{}\"", evidence);
                                        Ok(InstallResult {
                                            success: false,
                                            message: format!("共享连接创建命令失败: {}\n\n连接名称: {}\n\nEvidence: {}", e, conn_path, evidence),
                                            method: Some("Package".to_string()),
                                            stdout: Some(stage_result.output_text),
                                            stderr: Some(evidence),
                                        })
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[PackageBranch] RegisterPrinterDriver 失败: {}", e);
                    Ok(InstallResult {
                        success: false,
                        message: format!(
                            "pnputil stage 成功，但驱动注册失败\n\nPublished name: {}\nDriver name: {}\n\n错误:\n{}",
                            published_name, target_driver_name, e
                        ),
                        method: Some("Package".to_string()),
                        stdout: Some(stage_result.output_text),
                        stderr: Some(e),
                    })
                }
            }
        }
        Err(e) => {
            eprintln!("[PackageBranch] pnputil stage 失败: {}", e);
            // 检查是否是权限错误
            let mut error_msg = format!("pnputil stage 失败: {}", e);
            let error_lower = e.to_lowercase();
            if error_lower.contains("access is denied") 
                || error_lower.contains("0x5") 
                || error_lower.contains("requires elevation")
                || error_lower.contains("需要提升")
                || error_lower.contains("权限")
            {
                error_msg.push_str("\n\n请以管理员身份运行");
            }
            
            Ok(InstallResult {
                success: false,
                message: error_msg,
                method: Some("Package".to_string()),
                stdout: None,
                stderr: Some(e),
            })
        }
    }
}

// ============================================================================
// dryRun 模式：模拟安装流程
// ============================================================================

/// dryRun 模式下的模拟安装流程
async fn install_printer_windows_dry_run(
    name: String,
    _path: String,
    driver_path: Option<String>,
    _model: Option<String>,
    install_mode: Option<String>,
) -> Result<InstallResult, String> {
    use tokio::time::{sleep, Duration};
    
    let normalized_mode = install_mode.as_deref().unwrap_or("auto");
    eprintln!("[InstallStep] DRY_RUN_START step=检查打印机驱动 start");
    
    // 步骤1: 检查打印机驱动
    eprintln!("[InstallStep] DRY_RUN step=检查打印机驱动 start");
    sleep(Duration::from_millis(250)).await; // 固定 250ms 延迟
    eprintln!("[InstallStep] DRY_RUN step=检查打印机驱动 success");
    
    // 步骤2: 添加打印机端口
    eprintln!("[InstallStep] DRY_RUN step=添加打印机端口 start");
    sleep(Duration::from_millis(250)).await; // 固定 250ms 延迟
    eprintln!("[InstallStep] DRY_RUN step=添加打印机端口 success");
    
    // 步骤3: 查找品牌驱动（如果有 driver_path）
    if driver_path.is_some() {
        eprintln!("[InstallStep] DRY_RUN step=查找品牌驱动 start");
        sleep(Duration::from_millis(250)).await; // 固定 250ms 延迟
        eprintln!("[InstallStep] DRY_RUN step=查找品牌驱动 success");
        
        // 步骤4: 从配置文件安装 INF 驱动
        eprintln!("[InstallStep] DRY_RUN step=从配置文件安装_INF_驱动 start");
        sleep(Duration::from_millis(250)).await; // 固定 250ms 延迟
        eprintln!("[InstallStep] DRY_RUN step=从配置文件安装_INF_驱动 success");
    }
    
    // 步骤5: 安装打印机驱动
    eprintln!("[InstallStep] DRY_RUN step=安装打印机驱动 start");
    sleep(Duration::from_millis(250)).await; // 固定 250ms 延迟
    eprintln!("[InstallStep] DRY_RUN step=安装打印机驱动 success");
    
    // 步骤6: 配置打印机
    eprintln!("[InstallStep] DRY_RUN step=配置打印机 start");
    sleep(Duration::from_millis(250)).await; // 固定 250ms 延迟
    eprintln!("[InstallStep] DRY_RUN step=配置打印机 success");
    
    // 步骤7: 验证安装
    eprintln!("[InstallStep] DRY_RUN step=验证安装 start");
    sleep(Duration::from_millis(200)).await; // 固定 200ms 延迟
    eprintln!("[InstallStep] DRY_RUN step=验证安装 success");
    
    eprintln!("[InstallStep] DRY_RUN_STOP step=验证安装 success");
    
    Ok(InstallResult {
        success: true,
        message: format!("安装请求已接收（模式: {}），当前为测试模式，未执行真实安装", normalized_mode),
        method: Some("dryRun".to_string()),
        stdout: None,
        stderr: None,
    })
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

/// 检查已存在的打印机（如果存在）
/// 注意：不再执行删除操作，仅用于检查存在性
/// 如果打印机已存在，安装过程可能会失败，需要用户手动删除
fn check_existing_printer(name: &str) -> bool {
    // 使用 list_printers_windows 检查打印机是否存在
    match super::list::list_printers_windows() {
        Ok(printers) => printers.iter().any(|p| p == name),
        Err(_) => false, // 检查失败，假设不存在
    }
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

/// 判定 pnputil stage 是否成功（基于输出内容）
/// 
/// # 成功条件（满足任一即 true）
/// - 输出包含"已成功添加驱动程序包"（中文）
/// - 输出包含"发布名称:" AND "oem" AND ".inf"（中文）
/// - 输出包含"Driver package added successfully"（英文）
/// - 输出包含"Published name" AND "oem" AND ".inf"（英文）
/// 
/// # 注意
/// 此函数不检查失败条件，失败条件在主逻辑中优先判定
fn pnputil_stage_succeeded(output: &str) -> bool {
    let output_lower = output.to_lowercase();
    
    // 中文成功关键词
    if output.contains("已成功添加驱动程序包") {
        return true;
    }
    if output.contains("发布名称:") && output_lower.contains("oem") && output_lower.contains(".inf") {
        return true;
    }
    
    // 英文成功关键词
    if output_lower.contains("driver package added successfully") {
        return true;
    }
    if output_lower.contains("published name") && output_lower.contains("oem") && output_lower.contains(".inf") {
        return true;
    }
    
    false
}

/// 检查是否包含权限失败关键词
fn has_permission_error(output: &str) -> bool {
    let output_lower = output.to_lowercase();
    output_lower.contains("access is denied")
        || output_lower.contains("拒绝访问")
        || output_lower.contains("需要提升")
        || output_lower.contains("requires elevation")
        || output_lower.contains("0x5")
}

/// 检查是否包含明显失败关键词
fn has_failure_keywords(output: &str) -> bool {
    let output_lower = output.to_lowercase();
    output_lower.contains("失败")
        || output_lower.contains("failed")
        || output_lower.contains("error")
        || output_lower.contains("the system cannot find the file specified")
        || output_lower.contains("找不到")
        || output_lower.contains("no such file")
}

/// 从输出中提取发布名称（oemXXX.inf）
fn extract_published_name(output: &str) -> Option<String> {
    // 查找 "发布名称:" 或 "Published name:" 后的 oemXXX.inf
    let patterns = [
        ("发布名称:", "oem"),
        ("Published name:", "oem"),
    ];
    
    for (prefix, start_pattern) in patterns.iter() {
        if let Some(prefix_pos) = output.find(prefix) {
            let after_prefix = &output[prefix_pos + prefix.len()..];
            // 查找 oem 开头的 .inf 文件名（不区分大小写）
            let after_prefix_lower = after_prefix.to_lowercase();
            if let Some(oem_pos) = after_prefix_lower.find(start_pattern) {
                // 使用原始字符串的位置（因为大小写可能不同）
                let oem_part = &after_prefix[oem_pos..];
                // 提取到 .inf 为止（不区分大小写）
                let oem_part_lower = oem_part.to_lowercase();
                if let Some(inf_pos) = oem_part_lower.find(".inf") {
                    let name = &oem_part[..inf_pos + 4];
                    // 提取完整的文件名（可能包含空格，需要 trim）
                    let name = name.trim();
                    if !name.is_empty() {
                        return Some(name.to_string());
                    }
                }
            }
        }
    }
    
    None
}

/// Stage 驱动包（仅执行 pnputil stage，不创建队列）
/// 
/// # 参数
/// - `inf_path`: INF 文件的完整路径
/// 
/// # 返回
/// - `Ok(StageResult)`: stage 成功，包含 exit_code 和输出
/// - `Err(String)`: stage 失败，包含错误信息
/// 
/// # 实现说明
/// 使用 pnputil.exe /add-driver <inf_abs_path> /install /subdirs
/// - inf_path 必须 canonicalize 成绝对路径
/// - current_dir 必须设置为 inf_path 的父目录
/// - 不手动加引号；Command::args 传裸字符串
/// - 捕获 stdout/stderr 合并输出
/// 
/// # 成功判定逻辑
/// 1. 如果命中权限失败关键词 => 返回 Err
/// 2. 如果输出内容表示成功 => 返回 Ok（即使 exit_code 非 0）
/// 3. 如果 exit_code == 0 => 返回 Ok（兜底）
/// 4. 否则 => 返回 Err
fn stage_driver_package_windows(inf_path: &std::path::Path) -> Result<StageResult, String> {
    use std::process::{Command, Stdio};
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    
    // 检查 INF 文件是否存在
    if !inf_path.exists() {
        return Err(format!("缺少 driver_path 或 INF 文件不存在: {}", inf_path.display()));
    }
    
    // 将路径转换为绝对路径（canonicalize）
    let inf_path_abs = match inf_path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            return Err(format!("无法解析 INF 文件路径: {}", e));
        }
    };
    
    // 获取 inf_path 的父目录作为 current_dir
    let inf_dir = match inf_path_abs.parent() {
        Some(dir) => dir.to_path_buf(),
        None => {
            return Err(format!("无法获取 INF 文件所在目录: {}", inf_path_abs.display()));
        }
    };
    
    // 将路径转换为字符串（不手动加引号）
    let inf_path_str = inf_path_abs.to_string_lossy().to_string();
    
    // 构建 pnputil 命令
    let mut cmd = Command::new("pnputil.exe");
    cmd.args(&["/add-driver", &inf_path_str, "/install", "/subdirs"])
       .current_dir(&inf_dir)
       .stdin(Stdio::null())
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());
    
    #[cfg(windows)]
    {
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    
    // 执行命令
    let output = match cmd.output() {
        Ok(output) => output,
        Err(e) => {
            let err_msg = format!("pnputil 命令执行失败: {}", e);
            eprintln!("[StageDriverPackage] failed error=\"{}\"", err_msg);
            return Err(err_msg);
        }
    };
    
    let exit_code = output.status.code();
    let stdout = crate::platform::windows::encoding::decode_windows_string(&output.stdout);
    let stderr = crate::platform::windows::encoding::decode_windows_string(&output.stderr);
    
    // 合并 stdout 和 stderr（2>&1 等效）
    let output_text = if !stderr.is_empty() {
        format!("{}\n{}", stdout, stderr)
    } else {
        stdout
    };
    
    // 按优先级判定成功/失败
    // 1. 先检查权限失败关键词（优先级最高）
    if has_permission_error(&output_text) {
        let output_preview = if output_text.len() > 300 {
            format!("{}... (truncated, total {} chars)", &output_text[..300], output_text.len())
        } else {
            output_text.clone()
        };
        eprintln!("[StageDriverPackage] failed exit={:?} output=\"{}\"", exit_code, output_preview);
        return Err(format!("pnputil stage 失败（权限错误）。请以管理员身份运行\n\n完整输出:\n{}", output_text));
    }
    
    // 2. 检查输出内容是否表示成功（即使 exit_code 非 0）
    if pnputil_stage_succeeded(&output_text) {
        let published_name = extract_published_name(&output_text);
        let published_str = published_name.as_ref()
            .map(|n| format!(" published={}", n))
            .unwrap_or_default();
        eprintln!("[StageDriverPackage] success exit={:?}{}", exit_code, published_str);
        return Ok(StageResult {
            exit_code,
            output_text,
            inf_path: inf_path_str,
        });
    }
    
    // 3. 如果 exit_code == 0，视为成功（兜底）
    if exit_code == Some(0) {
        let published_name = extract_published_name(&output_text);
        let published_str = published_name.as_ref()
            .map(|n| format!(" published={}", n))
            .unwrap_or_default();
        eprintln!("[StageDriverPackage] success exit={:?}{}", exit_code, published_str);
        return Ok(StageResult {
            exit_code,
            output_text,
            inf_path: inf_path_str,
        });
    }
    
    // 4. 检查是否包含明显失败关键词
    if has_failure_keywords(&output_text) {
        let output_preview = if output_text.len() > 300 {
            format!("{}... (truncated, total {} chars)", &output_text[..300], output_text.len())
        } else {
            output_text.clone()
        };
        eprintln!("[StageDriverPackage] failed exit={:?} output=\"{}\"", exit_code, output_preview);
        return Err(format!("pnputil stage 失败 (exit code: {:?})\n\n完整输出:\n{}", exit_code, output_text));
    }
    
    // 5. 其他情况：如果 exit_code 非 0 且输出不明确，也判定为失败
    let output_preview = if output_text.len() > 300 {
        format!("{}... (truncated, total {} chars)", &output_text[..300], output_text.len())
    } else {
        output_text.clone()
    };
    eprintln!("[StageDriverPackage] failed exit={:?} output=\"{}\"", exit_code, output_preview);
    Err(format!("pnputil stage 失败 (exit code: {:?})\n\n完整输出:\n{}", exit_code, output_text))
}

/// Stage 驱动包的结果
#[derive(Debug)]
struct StageResult {
    exit_code: Option<i32>,
    output_text: String,
    inf_path: String,
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
/// 检测目标类型（IP/hostname 或共享连接）
/// 
/// # 返回
/// - `Ok(TargetType)`: 目标类型
/// - `Err(String)`: 无法识别的路径格式
/// 
/// # 判定优先级（按顺序）
/// 1. SharedConnection：必须匹配 `^\\[^\\]+\\[^\\]+` 且路径段数 >= 2（host + share）
/// 2. HostOnlyUNC：匹配 `^\\[^\\]+$`（只有 host，没有第二段）
/// 3. Plain TcpIpHost：IPv4 或 hostname（不含反斜杠）
/// 4. 其他：返回错误
#[derive(Debug, Clone)]
enum TargetType {
    /// TCP/IP 主机（IP 或 hostname）
    TcpIpHost { host: String },
    /// 共享连接（\\server\share）
    SharedConnection { path: String },
}

fn detect_target_type(target_path: &str) -> Result<TargetType, String> {
    let trimmed = target_path.trim();
    
    // 优先级 1：检查是否是合法的共享路径（\\server\share）
    // 必须匹配：^\\[^\\]+\\[^\\]+ 且 split 后路径段数 >= 2
    if trimmed.starts_with("\\\\") {
        // 按反斜杠分割路径
        let parts: Vec<&str> = trimmed.split('\\').filter(|s| !s.is_empty()).collect();
        
        if parts.len() >= 2 {
            // 合法的共享连接：\\server\share
            return Ok(TargetType::SharedConnection {
                path: trimmed.to_string(),
            });
        } else if parts.len() == 1 {
            // 只有 host 的 UNC 写法：\\192.168.20.5 或 \\server
            // 提取 host = 去掉开头两个反斜杠后的内容
            let host = parts[0].to_string();
            eprintln!("[DetectTargetType] HostOnlyUNC detected: host=\"{}\"", host);
            return Ok(TargetType::TcpIpHost { host });
        } else {
            // 空路径或只有反斜杠
            return Err(format!(
                "无效的 UNC 路径格式: \"{}\"。\n\nTCP/IP 打印机：填写 \"192.168.20.5\" 或 \"printer.company.local\"\n共享打印机：必须填写 \"\\\\server\\\\share\"",
                trimmed
            ));
        }
    }
    
    // 优先级 2：Plain TcpIpHost（不含反斜杠）
    // 检查是否是 IP 地址或 hostname
    if trimmed.contains('.') || trimmed.contains(':') {
        // 可能是 IP 地址（IPv4 或 IPv6）
        Ok(TargetType::TcpIpHost {
            host: trimmed.to_string(),
        })
    } else if trimmed.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        // 可能是 hostname
        Ok(TargetType::TcpIpHost {
            host: trimmed.to_string(),
        })
    } else {
        // 其他格式：不支持
        Err(format!(
            "无法识别的目标路径格式: \"{}\"。\n\nTCP/IP 打印机：填写 \"192.168.20.5\" 或 \"printer.company.local\"\n共享打印机：必须填写 \"\\\\server\\\\share\"",
            trimmed
        ))
    }
}

/// 生成端口名（沿用旧规则）
/// 
/// # 规则
/// - IP 地址：`IP_{ip.replace(".", "_")}`
/// - hostname：`IP_{hostname.replace(非法字符, "_")}`
fn generate_port_name(host: &str) -> String {
    // 如果是 IP 地址，用下划线替换点
    if host.contains('.') {
        format!("IP_{}", host.replace(".", "_"))
    } else {
        // hostname：将非法字符替换为下划线
        let sanitized = host
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect::<String>();
        format!("IP_{}", sanitized)
    }
}

/// 确保打印机端口存在（严格幂等 + 参数校验）
/// 
/// # 参数
/// - `ip_or_host`: IP 地址或 hostname
/// - `port_number`: 端口号（默认 9100）
/// - `is_legacy`: 是否使用 VBS 方式（Windows 7/8）
/// 
/// # 返回
/// - `Ok(port_name)`: 端口名
/// - `Err(String)`: 错误信息（包含 evidence）
fn ensure_printer_port(ip_or_host: &str, port_number: u16, is_legacy: bool) -> Result<String, String> {
    use crate::platform::windows::encoding::decode_windows_string;
    
    let port_name = generate_port_name(ip_or_host);
    eprintln!("[EnsurePrinterPort] step=start inputs=host=\"{}\" port={} port_name=\"{}\" is_legacy={}", 
        ip_or_host, port_number, port_name, is_legacy);
    
    // 步骤 1：检查端口是否存在
    // 确保 exit_code=0，通过 JSON exists 字段表达是否存在
    let check_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $port = Get-PrinterPort -Name '{}' -ErrorAction SilentlyContinue; if ($null -eq $port) {{ @{{ exists=$false }} | ConvertTo-Json -Compress | Write-Output }} else {{ @{{ exists=$true; Name=$port.Name; PrinterHostAddress=$port.PrinterHostAddress; PortNumber=$port.PortNumber; Protocol=$port.Protocol }} | ConvertTo-Json -Compress | Write-Output }}",
        port_name.replace("'", "''")
    );
    
    let check_result = match super::ps::run_powershell(&check_script) {
        Ok(output) => {
            let stdout = decode_windows_string(&output.stdout);
            let stderr = decode_windows_string(&output.stderr);
            let exit_code = output.status.code();
            
            // 统一成功判据：exit_code==0 才能进入 success 分支
            if exit_code != Some(0) {
                let evidence = format!("check_port_failed exit_code={:?} stdout=\"{}\" stderr=\"{}\"", exit_code, stdout, stderr);
                eprintln!("[EnsurePrinterPort] step=check_port result=error evidence=\"{}\"", evidence);
                return Err(format!("检查端口状态失败: exit_code={:?}, stderr={}\n\nEvidence: {}", exit_code, stderr, evidence));
            }
            
            // 解析 JSON 判断是否存在
            let extract_json_bool = |json: &str, key: &str| -> Option<bool> {
                let key_pattern = format!("\"{}\"", key);
                if let Some(key_pos) = json.find(&key_pattern) {
                    let after_key = &json[key_pos + key_pattern.len()..];
                    if let Some(colon_pos) = after_key.find(':') {
                        let after_colon = &after_key[colon_pos + 1..].trim();
                        if after_colon.starts_with("true") {
                            return Some(true);
                        } else if after_colon.starts_with("false") {
                            return Some(false);
                        }
                    }
                }
                None
            };
            
            let exists = extract_json_bool(&stdout, "exists").unwrap_or(false);
            if exists {
                // 端口存在，解析 JSON
                Some(stdout)
            } else {
                // 端口不存在
                None
            }
        }
        Err(e) => {
            eprintln!("[EnsurePrinterPort] step=check_port result=error evidence=check_failed error=\"{}\"", e);
            return Err(format!("检查端口状态失败: {}", e));
        }
    };
    
    if let Some(check_stdout) = check_result {
        // 端口已存在，需要校验参数
        eprintln!("[EnsurePrinterPort] step=validate_existing inputs=port_name=\"{}\" expected_host=\"{}\" expected_port={}", 
            port_name, ip_or_host, port_number);
        
        // 解析 JSON（简化处理，提取关键字段）
        let extract_json_value = |json: &str, key: &str| -> Option<String> {
            let key_pattern = format!("\"{}\"", key);
            if let Some(key_pos) = json.find(&key_pattern) {
                let after_key = &json[key_pos + key_pattern.len()..];
                if let Some(colon_pos) = after_key.find(':') {
                    let after_colon = &after_key[colon_pos + 1..].trim();
                    if let Some(quote_start) = after_colon.find('"') {
                        let value_start = quote_start + 1;
                        if let Some(quote_end) = after_colon[value_start..].find('"') {
                            return Some(after_colon[value_start..value_start + quote_end].to_string());
                        }
                    }
                }
            }
            None
        };
        
        let actual_host = extract_json_value(&check_stdout, "PrinterHostAddress");
        
        // 校验参数
        let host_matches = actual_host.as_ref().map(|h| h == ip_or_host).unwrap_or(false);
        
        // 如果 host 不匹配，fail-fast
        if !host_matches {
            let evidence = format!(
                "端口已存在但参数不匹配 | expected_host={} expected_port={} | actual_host={:?} actual_port=unknown | port_name={}",
                ip_or_host, port_number, actual_host, port_name
            );
            eprintln!("[EnsurePrinterPort] step=validate_existing result=fail-fast evidence=\"{}\"", evidence);
            return Err(format!(
                "端口名 \"{}\" 已被占用，但 Host 地址不匹配。期望: {}，实际: {:?}。请手动清理该端口或更改端口名策略。\n\nEvidence: {}",
                port_name, ip_or_host, actual_host, evidence
            ));
        }
        
        // 参数匹配，复用现有端口
        eprintln!("[EnsurePrinterPort] step=validate_existing result=skipped action=reuse evidence=host_matches port_name=\"{}\"", port_name);
        return Ok(port_name);
    }
    
    // 端口不存在，需要创建
    eprintln!("[EnsurePrinterPort] step=create_port inputs=port_name=\"{}\" host=\"{}\" port={}", 
        port_name, ip_or_host, port_number);
    
    let create_result = if is_legacy {
        // 使用 VBS 方式（Windows 7/8）
        let script_path = match write_vbs_script_to_temp() {
            Ok(path) => path,
            Err(e) => {
                let evidence = format!("vbs_script_creation_failed error=\"{}\"", e.to_user_message());
                eprintln!("[EnsurePrinterPort] step=create_port result=error evidence=\"{}\"", evidence);
                return Err(format!("创建 VBS 脚本失败: {}", e.to_user_message()));
            }
        };
        
        match add_printer_port_vbs(&script_path, &port_name, ip_or_host) {
            Ok(_) => {
                eprintln!("[EnsurePrinterPort] step=create_port result=success action=create method=vbs port_name=\"{}\"", port_name);
                Ok(())
            }
            Err(result) => {
                let evidence = format!("vbs_create_failed stdout=\"{}\" stderr=\"{}\"", 
                    result.stdout.as_ref().map(|s| s.as_str()).unwrap_or(""), 
                    result.stderr.as_ref().map(|s| s.as_str()).unwrap_or(""));
                eprintln!("[EnsurePrinterPort] step=create_port result=error evidence=\"{}\"", evidence);
                Err(format!("VBS 方式创建端口失败: {}", result.message))
            }
        }
    } else {
        // 使用现代方式（Windows 10+）
        match add_printer_port_modern(&port_name, ip_or_host) {
            Ok(outcome) => {
                match outcome {
                    PortAddOutcome::Created => {
                        eprintln!("[EnsurePrinterPort] step=create_port result=success action=create method=modern port_name=\"{}\"", port_name);
                    }
                    PortAddOutcome::AlreadyExists => {
                        eprintln!("[EnsurePrinterPort] step=create_port result=success action=reuse method=modern port_name=\"{}\"", port_name);
                    }
                }
                Ok(())
            }
            Err(e) => {
                let (stdout, stderr) = e.get_output();
                let evidence = format!("modern_create_failed stdout=\"{}\" stderr=\"{}\"", 
                    stdout.as_ref().map(|s| s.as_str()).unwrap_or(""), 
                    stderr.as_ref().map(|s| s.as_str()).unwrap_or(""));
                eprintln!("[EnsurePrinterPort] step=create_port result=error evidence=\"{}\"", evidence);
                Err(format!("现代方式创建端口失败: {}", e.to_user_message()))
            }
        }
    };
    
    // 创建后必须验证
    match create_result {
        Ok(_) => {
            match verify_printer_port(&port_name) {
                Ok(verified) => {
                    if verified {
                        eprintln!("[EnsurePrinterPort] step=verify_port result=success port_name=\"{}\"", port_name);
                        Ok(port_name)
                    } else {
                        let evidence = format!("port_created_but_verify_failed port_name=\"{}\"", port_name);
                        eprintln!("[EnsurePrinterPort] step=verify_port result=error evidence=\"{}\"", evidence);
                        Err(format!("端口创建成功但验证失败。端口名: {}\n\nEvidence: {}", port_name, evidence))
                    }
                }
                Err(e) => {
                    let evidence = format!("verify_command_failed error=\"{}\"", e.to_user_message());
                    eprintln!("[EnsurePrinterPort] step=verify_port result=error evidence=\"{}\"", evidence);
                    Err(format!("端口创建后验证失败: {}\n\nEvidence: {}", e.to_user_message(), evidence))
                }
            }
        }
        Err(e) => Err(e),
    }
}

/// 确保打印机队列存在（严格幂等 + 绑定修正）
/// 
/// # 参数
/// - `queue_name`: 队列名称
/// - `driver_name`: 驱动名称
/// - `port_name`: 端口名称
/// 
/// # 返回
/// - `Ok(())`: 成功
/// - `Err(String)`: 错误信息（包含 evidence）
fn ensure_printer_queue(queue_name: &str, driver_name: &str, port_name: &str) -> Result<(), String> {
    use crate::platform::windows::encoding::decode_windows_string;
    
    eprintln!("[EnsurePrinterQueue] step=start inputs=queue_name=\"{}\" driver_name=\"{}\" port_name=\"{}\"", 
        queue_name, driver_name, port_name);
    
    // 步骤 1：检查队列是否存在
    // 确保 exit_code=0，通过 JSON exists 字段表达是否存在
    // 修复：使用 Where-Object 精确过滤，避免 Get-Printer -Name 的通配符匹配导致误判
    let escaped_queue_name = queue_name.replace("'", "''");
    let check_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $q = '{}'; $printer = Get-Printer -Name $q -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -eq $q }}; if ($null -eq $printer) {{ @{{ exists=$false }} | ConvertTo-Json -Compress | Write-Output }} else {{ @{{ exists=$true; Name=$printer.Name; DriverName=$printer.DriverName; PortName=$printer.PortName }} | ConvertTo-Json -Compress | Write-Output }}",
        escaped_queue_name
    );
    
    let check_result = match super::ps::run_powershell(&check_script) {
        Ok(output) => {
            let stdout = decode_windows_string(&output.stdout);
            let stderr = decode_windows_string(&output.stderr);
            let exit_code = output.status.code();
            
            // 统一成功判据：exit_code==0 才能进入 success 分支
            if exit_code != Some(0) {
                let evidence = format!("check_queue_failed exit_code={:?} stdout=\"{}\" stderr=\"{}\"", exit_code, stdout, stderr);
                eprintln!("[EnsurePrinterQueue] step=check_queue result=error evidence=\"{}\"", evidence);
                return Err(format!("检查队列状态失败: exit_code={:?}, stderr={}\n\nEvidence: {}", exit_code, stderr, evidence));
            }
            
            // 解析 JSON 判断是否存在
            let extract_json_bool = |json: &str, key: &str| -> Option<bool> {
                let key_pattern = format!("\"{}\"", key);
                if let Some(key_pos) = json.find(&key_pattern) {
                    let after_key = &json[key_pos + key_pattern.len()..];
                    if let Some(colon_pos) = after_key.find(':') {
                        let after_colon = &after_key[colon_pos + 1..].trim();
                        if after_colon.starts_with("true") {
                            return Some(true);
                        } else if after_colon.starts_with("false") {
                            return Some(false);
                        }
                    }
                }
                None
            };
            
            let extract_json_value = |json: &str, key: &str| -> Option<String> {
                let key_pattern = format!("\"{}\"", key);
                if let Some(key_pos) = json.find(&key_pattern) {
                    let after_key = &json[key_pos + key_pattern.len()..];
                    if let Some(colon_pos) = after_key.find(':') {
                        let after_colon = &after_key[colon_pos + 1..].trim();
                        if let Some(quote_start) = after_colon.find('"') {
                            let value_start = quote_start + 1;
                            if let Some(quote_end) = after_colon[value_start..].find('"') {
                                return Some(after_colon[value_start..value_start + quote_end].to_string());
                            }
                        }
                    }
                }
                None
            };
            
            let exists = extract_json_bool(&stdout, "exists").unwrap_or(false);
            
            // 二次确认：如果 exists=true，必须验证 Name 字段完全等于 queue_name
            if exists {
                let actual_name = extract_json_value(&stdout, "Name");
                if actual_name.as_ref().map(|n| n != queue_name).unwrap_or(true) {
                    // Name 不匹配或不存在，强制视为不存在（防御性检查）
                    let evidence = format!("exists_check_mismatch expected=\"{}\" actual={:?} stdout=\"{}\"", 
                        queue_name, actual_name, stdout);
                    eprintln!("[EnsurePrinterQueue] step=check_queue result=error evidence=\"{}\"", evidence);
                    // 视为不存在，继续创建流程
                    None
                } else {
                    // 精确匹配，队列存在
                    Some(stdout)
                }
            } else {
                // 队列不存在
                None
            }
        }
        Err(e) => {
            eprintln!("[EnsurePrinterQueue] step=check_queue result=error evidence=check_failed error=\"{}\"", e);
            return Err(format!("检查队列状态失败: {}", e));
        }
    };
    
    if let Some(check_stdout) = check_result {
        // 队列已存在，需要校验和修正绑定
        eprintln!("[EnsurePrinterQueue] step=validate_existing inputs=queue_name=\"{}\" expected_driver=\"{}\" expected_port=\"{}\"", 
            queue_name, driver_name, port_name);
        
        // 解析 JSON（简化处理，提取关键字段）
        // 使用简单的字符串匹配提取值
        let extract_json_value = |json: &str, key: &str| -> Option<String> {
            let key_pattern = format!("\"{}\"", key);
            if let Some(key_pos) = json.find(&key_pattern) {
                let after_key = &json[key_pos + key_pattern.len()..];
                if let Some(colon_pos) = after_key.find(':') {
                    let after_colon = &after_key[colon_pos + 1..].trim();
                    if let Some(quote_start) = after_colon.find('"') {
                        let value_start = quote_start + 1;
                        if let Some(quote_end) = after_colon[value_start..].find('"') {
                            return Some(after_colon[value_start..value_start + quote_end].to_string());
                        }
                    }
                }
            }
            None
        };
        
        let actual_driver = extract_json_value(&check_stdout, "DriverName");
        let actual_port = extract_json_value(&check_stdout, "PortName");
        
        // 检查是否需要修正
        let driver_needs_fix = actual_driver.as_ref().map(|d| d != driver_name).unwrap_or(true);
        let port_needs_fix = actual_port.as_ref().map(|p| p != port_name).unwrap_or(true);
        
        if driver_needs_fix || port_needs_fix {
            eprintln!("[EnsurePrinterQueue] step=fix_binding inputs=queue_name=\"{}\" driver_needs_fix={} port_needs_fix={}", 
                queue_name, driver_needs_fix, port_needs_fix);
            
            // 修正驱动绑定
            if driver_needs_fix {
                let fix_driver_script = format!(
                    "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; try {{ Set-Printer -Name '{}' -DriverName '{}' -ErrorAction Stop; Write-Output 'FixDriverSuccess' }} catch {{ Write-Error $_.Exception.Message; exit 1 }}",
                    queue_name.replace("'", "''"),
                    driver_name.replace("'", "''")
                );
                
                match super::ps::run_powershell(&fix_driver_script) {
                    Ok(output) => {
                        let stdout = decode_windows_string(&output.stdout);
                        let stderr = decode_windows_string(&output.stderr);
                        let exit_code = output.status.code();
                        
                        // 统一成功判据：exit_code==0 才能进入 success 分支
                        if exit_code != Some(0) {
                            let evidence = format!("fix_driver_failed exit_code={:?} stdout=\"{}\" stderr=\"{}\"", 
                                exit_code, stdout, stderr);
                            eprintln!("[EnsurePrinterQueue] step=fix_driver result=error evidence=\"{}\"", evidence);
                            return Err(format!("修正驱动绑定失败: {}\n\nEvidence: {}", stderr, evidence));
                        }
                        
                        eprintln!("[EnsurePrinterQueue] step=fix_driver result=success driver_name=\"{}\" stdout=\"{}\"", driver_name, stdout);
                    }
                    Err(e) => {
                        let evidence = format!("fix_driver_command_failed error=\"{}\"", e);
                        eprintln!("[EnsurePrinterQueue] step=fix_driver result=error evidence=\"{}\"", evidence);
                        return Err(format!("修正驱动绑定命令失败: {}\n\nEvidence: {}", e, evidence));
                    }
                }
            }
            
            // 修正端口绑定
            if port_needs_fix {
                let fix_port_script = format!(
                    "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; try {{ Set-Printer -Name '{}' -PortName '{}' -ErrorAction Stop; Write-Output 'FixPortSuccess' }} catch {{ Write-Error $_.Exception.Message; exit 1 }}",
                    queue_name.replace("'", "''"),
                    port_name.replace("'", "''")
                );
                
                match super::ps::run_powershell(&fix_port_script) {
                    Ok(output) => {
                        let stdout = decode_windows_string(&output.stdout);
                        let stderr = decode_windows_string(&output.stderr);
                        let exit_code = output.status.code();
                        
                        // 统一成功判据：exit_code==0 才能进入 success 分支
                        if exit_code != Some(0) {
                            let evidence = format!("fix_port_failed exit_code={:?} stdout=\"{}\" stderr=\"{}\"", 
                                exit_code, stdout, stderr);
                            eprintln!("[EnsurePrinterQueue] step=fix_port result=error evidence=\"{}\"", evidence);
                            return Err(format!("修正端口绑定失败: {}\n\nEvidence: {}", stderr, evidence));
                        }
                        
                        eprintln!("[EnsurePrinterQueue] step=fix_port result=success port_name=\"{}\" stdout=\"{}\"", port_name, stdout);
                    }
                    Err(e) => {
                        let evidence = format!("fix_port_command_failed error=\"{}\"", e);
                        eprintln!("[EnsurePrinterQueue] step=fix_port result=error evidence=\"{}\"", evidence);
                        return Err(format!("修正端口绑定命令失败: {}\n\nEvidence: {}", e, evidence));
                    }
                }
            }
        } else {
            eprintln!("[EnsurePrinterQueue] step=validate_existing result=skipped action=reuse evidence=bindings_match");
            // 绑定已匹配，无需修正
        }
    } else {
        // 队列不存在，需要创建
        eprintln!("[EnsurePrinterQueue] step=create_queue inputs=queue_name=\"{}\" driver_name=\"{}\" port_name=\"{}\"", 
            queue_name, driver_name, port_name);
        
        // 改造脚本：所有幂等逻辑必须 swallow 异常并保持 exit 0
        let create_script = format!(
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; try {{ Add-Printer -Name '{}' -DriverName '{}' -PortName '{}' -ErrorAction Stop; Write-Output 'QueueSuccess' }} catch {{ if ($_.Exception.Message -like '*already exists*' -or $_.Exception.Message -like '*已存在*') {{ Write-Output 'QueueExists' }} else {{ Write-Error $_.Exception.Message; exit 1 }} }}",
            queue_name.replace("'", "''"),
            driver_name.replace("'", "''"),
            port_name.replace("'", "''")
        );
        
        match super::ps::run_powershell(&create_script) {
            Ok(output) => {
                let stdout = decode_windows_string(&output.stdout);
                let stderr = decode_windows_string(&output.stderr);
                let exit_code = output.status.code();
                
                // 统一成功判据：exit_code==0 才能进入 success 分支
                if exit_code != Some(0) {
                    let evidence = format!("create_queue_failed exit_code={:?} stdout=\"{}\" stderr=\"{}\"", 
                        exit_code, stdout, stderr);
                    eprintln!("[EnsurePrinterQueue] step=create_queue result=error evidence=\"{}\"", evidence);
                    return Err(format!("创建队列失败: {}\n\nEvidence: {}", stderr, evidence));
                }
                
                // exit_code==0，检查 stdout 中的标记
                let queue_created = stdout.contains("QueueSuccess");
                let queue_exists = stdout.contains("QueueExists");
                
                if !queue_created && !queue_exists {
                    // exit_code=0 但没有明确的成功标记，视为失败
                    let evidence = format!("create_queue_unclear_result exit_code={:?} stdout=\"{}\" stderr=\"{}\"", 
                        exit_code, stdout, stderr);
                    eprintln!("[EnsurePrinterQueue] step=create_queue result=error evidence=\"{}\"", evidence);
                    return Err(format!("创建队列结果不明确: {}\n\nEvidence: {}", stderr, evidence));
                }
                
                if queue_created {
                    eprintln!("[EnsurePrinterQueue] step=create_queue result=success action=create stdout=\"{}\"", stdout);
                } else {
                    eprintln!("[EnsurePrinterQueue] step=create_queue result=success action=reuse stdout=\"{}\"", stdout);
                }
            }
            Err(e) => {
                let evidence = format!("create_queue_command_failed error=\"{}\"", e);
                eprintln!("[EnsurePrinterQueue] step=create_queue result=error evidence=\"{}\"", evidence);
                return Err(format!("创建队列命令失败: {}\n\nEvidence: {}", e, evidence));
            }
        }
    }
    
    // 最终强校验：确保绑定正确
    eprintln!("[EnsurePrinterQueue] step=final_verify inputs=queue_name=\"{}\" expected_driver=\"{}\" expected_port=\"{}\"", 
        queue_name, driver_name, port_name);
    
    // final_verify 必须输出 JSON（即使不存在）
    // 修复：使用 Where-Object 精确过滤，避免 Get-Printer -Name 的通配符匹配导致误判
    let escaped_queue_name = queue_name.replace("'", "''");
    let verify_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $q = '{}'; $printer = Get-Printer -Name $q -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -eq $q }}; if ($null -eq $printer) {{ @{{ exists=$false }} | ConvertTo-Json -Compress | Write-Output }} else {{ @{{ exists=$true; Name=$printer.Name; DriverName=$printer.DriverName; PortName=$printer.PortName }} | ConvertTo-Json -Compress | Write-Output }}",
        escaped_queue_name
    );
    
    match super::ps::run_powershell(&verify_script) {
        Ok(output) => {
            let stdout = decode_windows_string(&output.stdout);
            let stderr = decode_windows_string(&output.stderr);
            let exit_code = output.status.code();
            
            // 统一成功判据：exit_code==0 才能进入 success 分支
            if exit_code != Some(0) {
                let evidence = format!("final_verify_failed exit_code={:?} stdout=\"{}\" stderr=\"{}\"", 
                    exit_code, stdout, stderr);
                eprintln!("[EnsurePrinterQueue] step=final_verify result=error evidence=\"{}\"", evidence);
                return Err(format!("最终验证失败: exit_code={:?}, stderr={}\n\nEvidence: {}", exit_code, stderr, evidence));
            }
            
            // exit_code==0，必须输出 JSON
            if stdout.trim().is_empty() {
                let evidence = format!("final_verify_empty_output exit_code={:?} stderr=\"{}\"", exit_code, stderr);
                eprintln!("[EnsurePrinterQueue] step=final_verify result=error evidence=\"{}\"", evidence);
                return Err(format!("最终验证失败: stdout 为空\n\nEvidence: {}", evidence));
            }
            
            // 解析 JSON 判断是否存在
            let extract_json_bool = |json: &str, key: &str| -> Option<bool> {
                let key_pattern = format!("\"{}\"", key);
                if let Some(key_pos) = json.find(&key_pattern) {
                    let after_key = &json[key_pos + key_pattern.len()..];
                    if let Some(colon_pos) = after_key.find(':') {
                        let after_colon = &after_key[colon_pos + 1..].trim();
                        if after_colon.starts_with("true") {
                            return Some(true);
                        } else if after_colon.starts_with("false") {
                            return Some(false);
                        }
                    }
                }
                None
            };
            
            let extract_json_value = |json: &str, key: &str| -> Option<String> {
                let key_pattern = format!("\"{}\"", key);
                if let Some(key_pos) = json.find(&key_pattern) {
                    let after_key = &json[key_pos + key_pattern.len()..];
                    if let Some(colon_pos) = after_key.find(':') {
                        let after_colon = &after_key[colon_pos + 1..].trim();
                        if let Some(quote_start) = after_colon.find('"') {
                            let value_start = quote_start + 1;
                            if let Some(quote_end) = after_colon[value_start..].find('"') {
                                return Some(after_colon[value_start..value_start + quote_end].to_string());
                            }
                        }
                    }
                }
                None
            };
            
            let exists = extract_json_bool(&stdout, "exists").unwrap_or(false);
            
            if !exists {
                // exists=false -> final_verify_failed（队列没创建成功或名称不一致）
                let evidence = format!("final_verify_queue_not_exists stdout=\"{}\"", stdout);
                eprintln!("[EnsurePrinterQueue] step=final_verify result=error evidence=\"{}\"", evidence);
                return Err(format!("最终验证失败: 队列不存在（exists=false）\n\nEvidence: {}", evidence));
            }
            
            // 二次确认：exists=true 时，必须验证 Name 字段完全等于 queue_name
            let actual_name = extract_json_value(&stdout, "Name");
            if actual_name.as_ref().map(|n| n != queue_name).unwrap_or(true) {
                // Name 不匹配或不存在，视为验证失败
                let evidence = format!(
                    "final_verify_name_mismatch | expected=\"{}\" actual={:?} | stdout=\"{}\"",
                    queue_name, actual_name, stdout
                );
                eprintln!("[EnsurePrinterQueue] step=final_verify result=error evidence=\"{}\"", evidence);
                return Err(format!(
                    "最终验证失败: 名称不匹配。期望: \"{}\"，实际: {:?}\n\nEvidence: {}",
                    queue_name, actual_name, evidence
                ));
            }
            
            // 解析并校验绑定（复用提取函数）
            let extract_json_value = |json: &str, key: &str| -> Option<String> {
                let key_pattern = format!("\"{}\"", key);
                if let Some(key_pos) = json.find(&key_pattern) {
                    let after_key = &json[key_pos + key_pattern.len()..];
                    if let Some(colon_pos) = after_key.find(':') {
                        let after_colon = &after_key[colon_pos + 1..].trim();
                        if let Some(quote_start) = after_colon.find('"') {
                            let value_start = quote_start + 1;
                            if let Some(quote_end) = after_colon[value_start..].find('"') {
                                return Some(after_colon[value_start..value_start + quote_end].to_string());
                            }
                        }
                    }
                }
                None
            };
            
            let actual_driver = extract_json_value(&stdout, "DriverName");
            let actual_port = extract_json_value(&stdout, "PortName");
            
            let driver_matches = actual_driver.as_ref().map(|d| d == driver_name).unwrap_or(false);
            let port_matches = actual_port.as_ref().map(|p| p == port_name).unwrap_or(false);
            
            if !driver_matches || !port_matches {
                let evidence = format!(
                    "bindings_mismatch | expected_driver={} expected_port={} | actual_driver={:?} actual_port={:?}",
                    driver_name, port_name, actual_driver, actual_port
                );
                eprintln!("[EnsurePrinterQueue] step=final_verify result=error evidence=\"{}\"", evidence);
                return Err(format!(
                    "最终验证失败: 绑定不匹配。期望: driver={}, port={}，实际: driver={:?}, port={:?}\n\nEvidence: {}",
                    driver_name, port_name, actual_driver, actual_port, evidence
                ));
            }
            
            eprintln!("[EnsurePrinterQueue] step=final_verify result=success evidence=bindings_match");
            Ok(())
        }
        Err(e) => {
            let evidence = format!("final_verify_command_failed error=\"{}\"", e);
            eprintln!("[EnsurePrinterQueue] step=final_verify result=error evidence=\"{}\"", evidence);
            Err(format!("最终验证命令失败: {}\n\nEvidence: {}", e, evidence))
        }
    }
}

/// 重试几次，因为端口创建可能需要时间
fn verify_printer_port(port_name: &str) -> Result<bool, InstallError> {
    let mut port_verified = false;
    for attempt in 1..=3 {
        eprintln!("[DEBUG] 验证端口存在（尝试 {}/3）", attempt);
        
        // Get-PrinterPort 查询脚本必须保证 exit_code=0（通过 -ErrorAction SilentlyContinue 且不 throw）
        // 通过 JSON exists 字段表达是否存在
        let verify_port_script = format!(
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $port = Get-PrinterPort -Name '{}' -ErrorAction SilentlyContinue; if ($null -eq $port) {{ @{{ exists=$false }} | ConvertTo-Json -Compress | Write-Output }} else {{ @{{ exists=$true }} | ConvertTo-Json -Compress | Write-Output }}",
            port_name.replace("'", "''")
        );
        let verify_port = super::ps::run_powershell(&verify_port_script);
        
        match verify_port {
            Ok(verify_result) => {
                let verify_stdout = decode_windows_string(&verify_result.stdout);
                let verify_stderr = decode_windows_string(&verify_result.stderr);
                let exit_code = verify_result.status.code();
                
                // 统一成功判据：exit_code==0 才能进入 success 分支
                if exit_code != Some(0) {
                    eprintln!("[VerifyPrinterPort] attempt={} exit_code={:?} stderr=\"{}\"", attempt, exit_code, verify_stderr);
                    if attempt < 3 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    } else {
                        return Err(InstallError::PowerShellFailed {
                            step: "verify_printer_port",
                            stderr: format!("端口验证失败: exit_code={:?}, stderr={}", exit_code, verify_stderr),
                        });
                    }
                    continue;
                }
                
                // 解析 JSON 判断是否存在
                let extract_json_bool = |json: &str, key: &str| -> Option<bool> {
                    let key_pattern = format!("\"{}\"", key);
                    if let Some(key_pos) = json.find(&key_pattern) {
                        let after_key = &json[key_pos + key_pattern.len()..];
                        if let Some(colon_pos) = after_key.find(':') {
                            let after_colon = &after_key[colon_pos + 1..].trim();
                            if after_colon.starts_with("true") {
                                return Some(true);
                            } else if after_colon.starts_with("false") {
                                return Some(false);
                            }
                        }
                    }
                    None
                };
                
                let exists = extract_json_bool(&verify_stdout, "exists").unwrap_or(false);
                if exists {
                    eprintln!("[VerifyPrinterPort] attempt={} result=success port_name=\"{}\"", attempt, port_name);
                    port_verified = true;
                    break;
                } else {
                    eprintln!("[VerifyPrinterPort] attempt={} result=not_found port_name=\"{}\"", attempt, port_name);
                    if attempt < 3 {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                    }
                }
            }
            Err(e) => {
                eprintln!("[VerifyPrinterPort] attempt={} result=error error=\"{}\"", attempt, e);
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
/// 
/// # 幂等逻辑
/// - 如果端口已存在：输出 "PortExists"，不 throw，exit_code=0
/// - 如果端口创建成功：输出 "PortSuccess"，exit_code=0
/// - 其他错误：throw，exit_code!=0
fn add_printer_port_modern(port_name: &str, ip_address: &str) -> Result<PortAddOutcome, InstallError> {
    eprintln!("[DEBUG] 添加打印机端口 {}", port_name);
    // 改造脚本：所有幂等逻辑必须 swallow 异常并保持 exit 0
    let port_add_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; try {{ Add-PrinterPort -Name '{}' -PrinterHostAddress '{}' -ErrorAction Stop; Write-Output 'PortSuccess' }} catch {{ if ($_.Exception.Message -like '*already exists*' -or $_.Exception.Message -like '*已存在*') {{ Write-Output 'PortExists' }} else {{ Write-Error $_.Exception.Message; exit 1 }} }}",
        port_name.replace("'", "''"),
        ip_address.replace("'", "''")
    );
    let port_add_result = super::ps::run_powershell(&port_add_script);
    
    match port_add_result {
        Ok(port_result) => {
            let port_stdout = decode_windows_string(&port_result.stdout);
            let port_stderr = decode_windows_string(&port_result.stderr);
            let exit_code = port_result.status.code();
            
            // 统一成功判据：exit_code==0 才能进入 success 分支
            if exit_code != Some(0) {
                let evidence = format!("add_port_failed exit_code={:?} stdout=\"{}\" stderr=\"{}\"", exit_code, port_stdout, port_stderr);
                eprintln!("[AddPrinterPortModern] step=add_port result=error evidence=\"{}\"", evidence);
                return Err(InstallError::PortAddFailedModern {
                    stdout: port_stdout,
                    stderr: port_stderr,
                });
            }
            
            // exit_code==0，检查 stdout 中的标记
            let port_created = port_stdout.contains("PortSuccess");
            let port_exists = port_stdout.contains("PortExists");
            
            if !port_created && !port_exists {
                // exit_code=0 但没有明确的成功标记，视为失败
                let evidence = format!("add_port_unclear_result exit_code={:?} stdout=\"{}\" stderr=\"{}\"", exit_code, port_stdout, port_stderr);
                eprintln!("[AddPrinterPortModern] step=add_port result=error evidence=\"{}\"", evidence);
                return Err(InstallError::PortAddFailedModern {
                    stdout: port_stdout,
                    stderr: port_stderr,
                });
            }
            
            // 确定端口是新建还是已存在
            let outcome = if port_created {
                eprintln!("[AddPrinterPortModern] step=add_port result=success action=create stdout=\"{}\"", port_stdout);
                PortAddOutcome::Created
            } else {
                eprintln!("[AddPrinterPortModern] step=add_port result=success action=reuse stdout=\"{}\"", port_stdout);
                PortAddOutcome::AlreadyExists
            };
            
            Ok(outcome)
        }
        Err(e) => {
            let evidence = format!("add_port_command_failed error=\"{}\"", e);
            eprintln!("[AddPrinterPortModern] step=add_port result=error evidence=\"{}\"", evidence);
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
// 辅助函数：安装后写入 ePrinty tag
// ============================================================================

/// 生成 stable_id
/// 
/// 规则：
/// 1. 如果配置中有 printer.id，使用它
/// 2. 如果没有，使用 area_id + printer.name + ip 生成稳定 hash
fn generate_stable_id(printer_name: &str, printer_path: &str) -> String {
    // 从配置中查找匹配的 printer，获取 area.name
    match crate::load_local_config() {
        Ok((config, _)) => {
            // 在所有 areas 中查找匹配的 printer
            for area in &config.areas {
                for printer in &area.printers {
                    if printer.name == printer_name || printer.path == printer_path {
                        // 检查是否有 id 字段（虽然当前结构体没有，但为将来扩展预留）
                        // 如果没有 id，生成 hash
                        let ip = printer_path.trim_start_matches("\\\\").trim_start_matches("\\").to_string();
                        let hash_input = format!("{}|{}|{}", area.name, printer.name, ip);
                        
                        // 使用 DefaultHasher 生成 hash（简单且跨机器一致）
                        let mut hasher = DefaultHasher::new();
                        hash_input.hash(&mut hasher);
                        let hash = hasher.finish();
                        
                        // 转换为 base64 编码的字符串（更短且可读）
                        // 使用简单的 hex 编码
                        return format!("{:x}", hash);
                    }
                }
            }
        }
        Err(_) => {
            // 配置加载失败，使用默认方式生成
        }
    }
    
    // 如果找不到配置，使用 name + path 生成
    let ip = printer_path.trim_start_matches("\\\\").trim_start_matches("\\").to_string();
    let hash_input = format!("{}|{}", printer_name, ip);
    let mut hasher = DefaultHasher::new();
    hash_input.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:x}", hash)
}

/// 安装成功后写入 ePrinty tag（调用 remove 模块的函数）
fn write_eprinty_tag_after_install(printer_name: &str, stable_id: &str, printer_path: &str) -> Result<(), String> {
    // 调用 remove 模块的 write_eprinty_tag 函数
    super::remove::write_eprinty_tag(printer_name, stable_id, printer_path)
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
    installMode: Option<String>,  // 安装方式："auto" | "package" | "installer" | "ipp" | "legacy_inf"（使用 camelCase 匹配前端）
    dry_run: bool,  // 测试模式：true 表示仅模拟，不执行真实安装
) -> Result<InstallResult, String> {
    
    // 打印接收到的参数
    eprintln!("[InstallPrinterWindows] received installMode={:?} dry_run={}", installMode, dry_run);
    
    // 如果是 dryRun 模式，执行模拟安装流程
    if dry_run {
        eprintln!("[InstallPrinterWindows] entering dryRun mode");
        return install_printer_windows_dry_run(name, path, driverPath, model, installMode).await;
    }
    
    eprintln!("[InstallPrinterWindows] dryRun=false, entering real installation path");
    
    // ============================================================================
    // 路由策略枚举
    // ============================================================================
    #[derive(Debug, Clone, Copy)]
    enum RoutingPolicy {
        /// 优先使用 modern 链路，失败时允许 fallback 到 legacy
        ModernPreferred,
        /// 仅使用 modern 链路，禁止 legacy
        ModernOnly,
        /// 允许 legacy fallback（显式允许）
        LegacyFallback,
    }
    
    // 解析 installMode（用于判断安装策略分流）
    let valid_modes = ["auto", "package", "installer", "ipp", "legacy_inf"];
    let resolved_mode = match &installMode {
        Some(mode) if valid_modes.contains(&mode.as_str()) => mode.as_str(),
        Some(invalid_mode) => {
            eprintln!("[InstallRequest] invalid installMode=\"{}\", fallback to auto", invalid_mode);
            "auto"
        },
        None => {
            eprintln!("[InstallRequest] missing install_mode, fallback to auto");
            "auto"
        }
    };
    
    // 确定路由策略
    let routing_policy = if resolved_mode == "package" {
        // 当 installMode="package"：强制 modern_only（禁止 PrintUIEntry）
        RoutingPolicy::ModernOnly
    } else {
        // 默认：modern_preferred
        RoutingPolicy::ModernPreferred
    };
    
    eprintln!("[InstallRequest] printer=\"{}\" path=\"{}\" mode={} resolved={} routing_policy={:?} dryRun={}", 
        name, path, installMode.as_deref().unwrap_or("auto"), resolved_mode, routing_policy, dry_run);
    
    // ============================================================================
    // 路由决策：收集输入信息
    // ============================================================================
    let has_driver_package = resolved_mode == "package";
    let has_driver_path = driverPath.as_ref().map(|p| !p.trim().is_empty()).unwrap_or(false);
    let has_model = model.as_ref().map(|m| !m.trim().is_empty()).unwrap_or(false);
    
    // 从配置中获取 driver_names
    let driver_names_option = match crate::load_local_config() {
        Ok((config, _)) => {
            let mut matched_driver_names: Option<Vec<String>> = None;
            for area in &config.areas {
                for printer in &area.printers {
                    if printer.name == name || printer.path == path {
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
            eprintln!("[WARN] 无法加载配置文件: {}，跳过配置校验", e);
            None
        }
    };
    
    let has_driver_names = driver_names_option.as_ref()
        .map(|names| !names.is_empty() && names.iter().any(|n| !n.trim().is_empty()))
        .unwrap_or(false);
    
    // 强制打印 RoutingDecision 日志
    eprintln!("[RoutingDecision] policy={:?} inputs=installMode={:?} driverPackage={} driverPath={} driver_name={} model={} target_path=\"{}\"", 
        routing_policy, resolved_mode, has_driver_package, has_driver_path, has_driver_names, has_model, path);
    
    // ============================================================================
    // 路由决策：选择安装路径
    // ============================================================================
    
    // 优先级 1：如果有 driver package（或已选择 package 模式）
    if has_driver_package {
        eprintln!("[RoutingDecision] selected_path=package reason=installMode_is_package");
        return install_printer_package_branch(name, path, driverPath, model, dry_run).await;
    }
    
    // 优先级 2：如果没有 package，但有 INF（driverPath）
    if has_driver_path {
        // 需要确定 driver_name
        let target_driver_name = if has_driver_names {
            // a) 配置显式 driver_name（使用第一个）
            Some(driver_names_option.as_ref().unwrap()[0].clone())
        } else if has_model {
            // b) 若配置有 model：从 INF 中匹配得到 driver_name
            // 简化实现：如果无法匹配则失败并提示需要配置 driver_name
            eprintln!("[RoutingDecision] selected_path=modern_inf reason=has_driverPath_but_no_driver_name_need_extract_from_inf");
            // TODO: 从 INF 中提取 driver_name（简化实现：先要求配置 driver_name）
            None
        } else {
            // c) 无法确定 driver_name：直接失败
            None
        };
        
        if let Some(driver_name) = target_driver_name {
            // 有 driver_name，尝试 modern_inf 路径
            eprintln!("[RoutingDecision] selected_path=modern_inf reason=has_driverPath_and_driver_name");
            
            // 解析 driver_path
            let inf_path = match resolve_driver_path(driverPath.as_ref().unwrap()) {
                Ok(path) => {
                    // 检查 INF 文件是否存在
                    if !path.exists() {
                        let evidence = format!("inf_file_not_exists path=\"{}\"", path.display());
                        eprintln!("[RoutingDecision] modern_inf_failed step=check_inf_exists evidence=\"{}\"", evidence);
                        
                        if matches!(routing_policy, RoutingPolicy::LegacyFallback) {
                            eprintln!("[RoutingDecision] fallback_to_legacy reason=inf_file_not_exists path=\"{}\"", path.display());
                            // 继续到 legacy 路径（设置标志，跳出 match）
                            return Ok(InstallResult {
                                success: false,
                                message: format!("INF 文件不存在，fallback 到 legacy 路径: {}\n\nEvidence: {}", path.display(), evidence),
                                method: Some("ModernInf".to_string()),
                                stdout: None,
                                stderr: Some(evidence),
                            });
                        } else {
                            return Ok(InstallResult {
                                success: false,
                                message: format!("INF 文件不存在: {}\n\nEvidence: {}", path.display(), evidence),
                                method: Some("ModernInf".to_string()),
                                stdout: None,
                                stderr: Some(evidence),
                            });
                        }
                    }
                    path
                }
                Err(e) => {
                    let evidence = format!("resolve_driver_path_failed error=\"{}\"", e.to_user_message());
                    eprintln!("[RoutingDecision] modern_inf_failed step=resolve_driver_path evidence=\"{}\"", evidence);
                    
                    // 根据 routing_policy 决定是否 fallback
                    if matches!(routing_policy, RoutingPolicy::LegacyFallback) {
                        eprintln!("[RoutingDecision] fallback_to_legacy reason=resolve_driver_path_failed error=\"{}\"", e.to_user_message());
                        // 继续到 legacy 路径（返回错误，但标记为 fallback）
                        return Ok(InstallResult {
                            success: false,
                            message: format!("无法解析 driver_path，fallback 到 legacy 路径: {}\n\nEvidence: {}", e.to_user_message(), evidence),
                            method: Some("ModernInf".to_string()),
                            stdout: None,
                            stderr: Some(evidence),
                        });
                    } else {
                        return Ok(InstallResult {
                            success: false,
                            message: format!("无法解析 driver_path: {}\n\nEvidence: {}", e.to_user_message(), evidence),
                            method: Some("ModernInf".to_string()),
                            stdout: None,
                            stderr: Some(evidence),
                        });
                    }
                }
            };
            
            // 执行 modern_inf 路径：stage + Add-PrinterDriver + ensure port+queue
            eprintln!("[ModernInf] step=start inputs=inf_path=\"{}\" driver_name=\"{}\"", inf_path.display(), driver_name);
            
            // 步骤 1：pnputil stage
            let stage_result = match stage_driver_package_windows(&inf_path) {
                Ok(result) => {
                    eprintln!("[ModernInf] step=pnputil_stage result=success exit_code={:?}", result.exit_code);
                    result
                }
                Err(e) => {
                    let evidence = format!("pnputil_stage_failed error=\"{}\"", e);
                    eprintln!("[ModernInf] step=pnputil_stage result=error evidence=\"{}\"", evidence);
                    
                    if matches!(routing_policy, RoutingPolicy::LegacyFallback) {
                        eprintln!("[RoutingDecision] fallback_to_legacy reason=pnputil_stage_failed error=\"{}\"", e);
                        // 继续到 legacy 路径
                        return Ok(InstallResult {
                            success: false,
                            message: format!("pnputil stage 失败: {}\n\nEvidence: {}", e, evidence),
                            method: Some("ModernInf".to_string()),
                            stdout: None,
                            stderr: Some(evidence),
                        });
                    } else {
                        return Ok(InstallResult {
                            success: false,
                            message: format!("pnputil stage 失败: {}\n\nEvidence: {}", e, evidence),
                            method: Some("ModernInf".to_string()),
                            stdout: None,
                            stderr: Some(evidence),
                        });
                    }
                }
            };
            
            // 步骤 2：提取 published_name 并注册驱动
            let published_name = match extract_published_name(&stage_result.output_text) {
                Some(name) => name,
                None => {
                    let evidence = format!("extract_published_name_failed output=\"{}\"", 
                        if stage_result.output_text.len() > 200 { &stage_result.output_text[..200] } else { &stage_result.output_text });
                    eprintln!("[ModernInf] step=extract_published_name result=error evidence=\"{}\"", evidence);
                    
                    if matches!(routing_policy, RoutingPolicy::LegacyFallback) {
                        eprintln!("[RoutingDecision] fallback_to_legacy reason=extract_published_name_failed");
                        return Ok(InstallResult {
                            success: false,
                            message: format!("无法从 pnputil 输出中提取 published name\n\nEvidence: {}", evidence),
                            method: Some("ModernInf".to_string()),
                            stdout: Some(stage_result.output_text),
                            stderr: Some(evidence),
                        });
                    } else {
                        return Ok(InstallResult {
                            success: false,
                            message: format!("无法从 pnputil 输出中提取 published name\n\nEvidence: {}", evidence),
                            method: Some("ModernInf".to_string()),
                            stdout: Some(stage_result.output_text),
                            stderr: Some(evidence),
                        });
                    }
                }
            };
            
            let published_inf_path = format!(r"C:\Windows\INF\{}", published_name);
            eprintln!("[ModernInf] step=register_driver inputs=driver_name=\"{}\" published_inf_path=\"{}\"", 
                driver_name, published_inf_path);
            
            match register_printer_driver(&driver_name, &published_inf_path, dry_run) {
                Ok(()) => {
                    eprintln!("[ModernInf] step=register_driver result=success");
                }
                Err(e) => {
                    let evidence = format!("register_driver_failed error=\"{}\"", e);
                    eprintln!("[ModernInf] step=register_driver result=error evidence=\"{}\"", evidence);
                    
                    if matches!(routing_policy, RoutingPolicy::LegacyFallback) {
                        eprintln!("[RoutingDecision] fallback_to_legacy reason=register_driver_failed error=\"{}\"", e);
                        return Ok(InstallResult {
                            success: false,
                            message: format!("注册驱动失败: {}\n\nEvidence: {}", e, evidence),
                            method: Some("ModernInf".to_string()),
                            stdout: Some(stage_result.output_text),
                            stderr: Some(evidence),
                        });
                    } else {
                        return Ok(InstallResult {
                            success: false,
                            message: format!("注册驱动失败: {}\n\nEvidence: {}", e, evidence),
                            method: Some("ModernInf".to_string()),
                            stdout: Some(stage_result.output_text),
                            stderr: Some(evidence),
                        });
                    }
                }
            }
            
            // 步骤 3：确保端口和队列存在（复用 package 分支的逻辑）
            let target_type = match detect_target_type(&path) {
                Ok(t) => t,
                Err(e) => {
                    let evidence = format!("detect_target_type_failed error=\"{}\"", e);
                    eprintln!("[ModernInf] step=detect_target_type result=error evidence=\"{}\"", evidence);
                    return Ok(InstallResult {
                        success: false,
                        message: format!("无法识别目标路径格式: {}\n\nEvidence: {}", e, evidence),
                        method: Some("ModernInf".to_string()),
                        stdout: Some(stage_result.output_text),
                        stderr: Some(evidence),
                    });
                }
            };
            
            match target_type {
                TargetType::TcpIpHost { host } => {
                    eprintln!("[ModernInf] step=ensure_port inputs=host=\"{}\"", host);
                    
                    let windows_build = get_windows_build_number().unwrap_or(0);
                    let is_legacy = windows_build > 0 && windows_build < 10240;
                    
                    let port_name = match ensure_printer_port(&host, 9100, is_legacy) {
                        Ok(port) => {
                            eprintln!("[ModernInf] step=ensure_port result=success port_name=\"{}\"", port);
                            port
                        }
                        Err(e) => {
                            let evidence = format!("ensure_port_failed error=\"{}\"", e);
                            eprintln!("[ModernInf] step=ensure_port result=error evidence=\"{}\"", evidence);
                            
                            if matches!(routing_policy, RoutingPolicy::LegacyFallback) {
                                eprintln!("[RoutingDecision] fallback_to_legacy reason=ensure_port_failed error=\"{}\"", e);
                                return Ok(InstallResult {
                                    success: false,
                                    message: format!("端口创建失败: {}\n\nEvidence: {}", e, evidence),
                                    method: Some("ModernInf".to_string()),
                                    stdout: Some(stage_result.output_text),
                                    stderr: Some(evidence),
                                });
                            } else {
                                return Ok(InstallResult {
                                    success: false,
                                    message: format!("端口创建失败: {}\n\nEvidence: {}", e, evidence),
                                    method: Some("ModernInf".to_string()),
                                    stdout: Some(stage_result.output_text),
                                    stderr: Some(evidence),
                                });
                            }
                        }
                    };
                    
                    eprintln!("[ModernInf] step=ensure_queue inputs=queue_name=\"{}\" driver_name=\"{}\" port_name=\"{}\"", 
                        name, driver_name, port_name);
                    
                    match ensure_printer_queue(&name, &driver_name, &port_name) {
                        Ok(()) => {
                            eprintln!("[ModernInf] step=ensure_queue result=success");
                            return Ok(InstallResult {
                                success: true,
                                message: format!(
                                    "Modern INF 安装完成\n\nPublished name: {}\nDriver name: {}\nPort name: {}\nQueue name: {}\n\npnputil 输出:\n{}",
                                    published_name, driver_name, port_name, name, stage_result.output_text
                                ),
                                method: Some("ModernInf".to_string()),
                                stdout: Some(stage_result.output_text),
                                stderr: None,
                            });
                        }
                        Err(e) => {
                            let evidence = format!("ensure_queue_failed error=\"{}\"", e);
                            eprintln!("[ModernInf] step=ensure_queue result=error evidence=\"{}\"", evidence);
                            
                            if matches!(routing_policy, RoutingPolicy::LegacyFallback) {
                                eprintln!("[RoutingDecision] fallback_to_legacy reason=ensure_queue_failed error=\"{}\"", e);
                                return Ok(InstallResult {
                                    success: false,
                                    message: format!("队列创建失败: {}\n\nEvidence: {}", e, evidence),
                                    method: Some("ModernInf".to_string()),
                                    stdout: Some(stage_result.output_text),
                                    stderr: Some(evidence),
                                });
                            } else {
                                return Ok(InstallResult {
                                    success: false,
                                    message: format!("队列创建失败: {}\n\nEvidence: {}", e, evidence),
                                    method: Some("ModernInf".to_string()),
                                    stdout: Some(stage_result.output_text),
                                    stderr: Some(evidence),
                                });
                            }
                        }
                    }
                }
                TargetType::SharedConnection { path: conn_path } => {
                    // 共享连接处理（复用 package 分支的逻辑）
                    eprintln!("[ModernInf] step=ensure_queue_shared inputs=connection_name=\"{}\" driver_name=\"{}\"", 
                        conn_path, driver_name);
                    
                    let parts: Vec<&str> = conn_path.split('\\').filter(|s| !s.is_empty()).collect();
                    if parts.len() < 2 {
                        let evidence = format!("InvalidSharedConnectionName connection_name=\"{}\" parts_count={}", conn_path, parts.len());
                        eprintln!("[ModernInf] step=ensure_queue_shared result=error evidence=\"{}\"", evidence);
                        return Ok(InstallResult {
                            success: false,
                            message: format!("无效的共享连接名称: \"{}\"\n\nEvidence: {}", conn_path, evidence),
                            method: Some("ModernInf".to_string()),
                            stdout: Some(stage_result.output_text),
                            stderr: Some(evidence),
                        });
                    }
                    
                    // 修复：使用 Where-Object 精确过滤，避免 Get-Printer -Name 的通配符匹配导致误判
                    let escaped_conn_path = conn_path.replace("'", "''");
                    let check_shared_script = format!(
                        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $q = '{}'; $printer = Get-Printer -Name $q -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -eq $q }} | Select-Object -ExpandProperty Name",
                        escaped_conn_path
                    );
                    
                    let queue_exists = match super::ps::run_powershell(&check_shared_script) {
                        Ok(output) => {
                            let stdout = decode_windows_string(&output.stdout);
                            // 二次确认：验证返回的名称是否完全等于 conn_path
                            !stdout.trim().is_empty() && stdout.trim() == conn_path
                        }
                        Err(_) => false,
                    };
                    
                    if queue_exists {
                        eprintln!("[ModernInf] step=ensure_queue_shared result=success action=reuse connection=\"{}\"", conn_path);
                        return Ok(InstallResult {
                            success: true,
                            message: format!(
                                "Modern INF 安装完成（共享连接已存在）\n\nPublished name: {}\nDriver name: {}\nConnection: {}\n\npnputil 输出:\n{}",
                                published_name, driver_name, conn_path, stage_result.output_text
                            ),
                            method: Some("ModernInf".to_string()),
                            stdout: Some(stage_result.output_text),
                            stderr: None,
                        });
                    } else {
                        let add_shared_script = format!(
                            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Add-Printer -ConnectionName '{}' -ErrorAction Stop",
                            conn_path.replace("'", "''")
                        );
                        
                        match super::ps::run_powershell(&add_shared_script) {
                            Ok(output) => {
                                let stdout = decode_windows_string(&output.stdout);
                                let stderr = decode_windows_string(&output.stderr);
                                let exit_code = output.status.code();
                                
                                if exit_code == Some(0) {
                                    eprintln!("[ModernInf] step=ensure_queue_shared result=success action=create connection=\"{}\"", conn_path);
                                    return Ok(InstallResult {
                                        success: true,
                                        message: format!(
                                            "Modern INF 安装完成\n\nPublished name: {}\nDriver name: {}\nConnection: {}\n\npnputil 输出:\n{}",
                                            published_name, driver_name, conn_path, stage_result.output_text
                                        ),
                                        method: Some("ModernInf".to_string()),
                                        stdout: Some(stage_result.output_text),
                                        stderr: None,
                                    });
                                } else {
                                    let evidence = format!("add_shared_failed exit_code={:?} stdout=\"{}\" stderr=\"{}\"", 
                                        exit_code, stdout, stderr);
                                    eprintln!("[ModernInf] step=ensure_queue_shared result=error evidence=\"{}\"", evidence);
                                    
                                    if matches!(routing_policy, RoutingPolicy::LegacyFallback) {
                                        eprintln!("[RoutingDecision] fallback_to_legacy reason=add_shared_failed exit_code={:?}", exit_code);
                                        return Ok(InstallResult {
                                            success: false,
                                            message: format!("共享连接创建失败: {}\n\nEvidence: {}", stderr, evidence),
                                            method: Some("ModernInf".to_string()),
                                            stdout: Some(stage_result.output_text),
                                            stderr: Some(evidence),
                                        });
                                    } else {
                                        return Ok(InstallResult {
                                            success: false,
                                            message: format!("共享连接创建失败: {}\n\nEvidence: {}", stderr, evidence),
                                            method: Some("ModernInf".to_string()),
                                            stdout: Some(stage_result.output_text),
                                            stderr: Some(evidence),
                                        });
                                    }
                                }
                            }
                            Err(e) => {
                                let evidence = format!("add_shared_command_failed error=\"{}\"", e);
                                eprintln!("[ModernInf] step=ensure_queue_shared result=error evidence=\"{}\"", evidence);
                                
                                if matches!(routing_policy, RoutingPolicy::LegacyFallback) {
                                    eprintln!("[RoutingDecision] fallback_to_legacy reason=add_shared_command_failed error=\"{}\"", e);
                                    return Ok(InstallResult {
                                        success: false,
                                        message: format!("共享连接创建命令失败: {}\n\nEvidence: {}", e, evidence),
                                        method: Some("ModernInf".to_string()),
                                        stdout: Some(stage_result.output_text),
                                        stderr: Some(evidence),
                                    });
                                } else {
                                    return Ok(InstallResult {
                                        success: false,
                                        message: format!("共享连接创建命令失败: {}\n\nEvidence: {}", e, evidence),
                                        method: Some("ModernInf".to_string()),
                                        stdout: Some(stage_result.output_text),
                                        stderr: Some(evidence),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        } else {
            // 缺失 driver_name：禁止"无 driver_name 就 printui"这种隐性兜底
            let evidence = format!("MissingDriverNameMapping driverPath={:?} has_driver_names={} has_model={}", 
                driverPath.as_ref().map(|p| p.as_str()), has_driver_names, has_model);
            eprintln!("[RoutingDecision] selected_path=error reason=MissingDriverNameMapping evidence=\"{}\"", evidence);
            
            return Ok(InstallResult {
                success: false,
                message: format!(
                    "缺失 driver_name 映射。Modern 链路需要 driver_name 才能执行。\n\n请在 printer_config.json 中补齐以下字段之一：\n- driver_names: [\"驱动名称\"]\n- model: \"型号\"（用于从 INF 中匹配 driver_name）\n\n当前配置：driverPath={:?}, driver_names={}, model={}\n\nEvidence: {}",
                    driverPath.as_ref().map(|p| p.as_str()), has_driver_names, has_model, evidence
                ),
                method: Some("ModernInf".to_string()),
                stdout: None,
                stderr: Some(evidence),
            });
        }
    }
    
    // 优先级 3：仅当 routing_policy=legacy_fallback 且 modern 链路明确失败时，才允许调用 PrintUIEntry
    if matches!(routing_policy, RoutingPolicy::LegacyFallback) {
        eprintln!("[RoutingDecision] selected_path=legacy_printui reason=routing_policy_is_legacy_fallback");
    } else {
        // modern_preferred 或 modern_only：不允许 fallback 到 PrintUIEntry
        let evidence = format!("NoModernPathAvailable routing_policy={:?} has_driver_package={} has_driver_path={}", 
            routing_policy, has_driver_package, has_driver_path);
        eprintln!("[RoutingDecision] selected_path=error reason=NoModernPathAvailable evidence=\"{}\"", evidence);
        
        return Ok(InstallResult {
            success: false,
            message: format!(
                "无法使用 Modern 链路安装，且 routing_policy={:?} 不允许 fallback 到 Legacy。\n\n请检查配置：\n- 是否提供了 driverPath？\n- 是否提供了 driver_names 或 model？\n\nEvidence: {}",
                routing_policy, evidence
            ),
            method: Some("Routing".to_string()),
            stdout: None,
            stderr: Some(evidence),
        });
    }
    
    // Legacy PrintUIEntry 路径（仅在 legacy_fallback 模式下）
    eprintln!("[RoutingDecision] selected_path=legacy_printui reason=fallback_to_legacy");
    
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
                    
                    // 检查旧打印机是否存在（不再删除）
                    if check_existing_printer(&name) {
                        eprintln!("[DEBUG] 检测到同名打印机已存在: {}", name);
                        // 不删除，继续尝试安装（系统可能会提示已存在）
                    }
                    
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
                            // 安装成功后写入 ePrinty tag
                            if result.success {
                                let stable_id = generate_stable_id(&name, &path);
                                match write_eprinty_tag_after_install(&name, &stable_id, &path) {
                                    Ok(_) => {
                                        eprintln!("[INFO] ePrinty tag 写入成功: name={}, stable_id={}", name, stable_id);
                                    }
                                    Err(err) => {
                                        eprintln!("[WARN] ePrinty tag 写入失败（不影响安装成功）: {}", err);
                                        super::log::write_log(&format!("[Install] TAG_WRITE_FAIL name={} error={}", name, err));
                                    }
                                }
                            }
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
    
    // 注意：当 resolved_mode == "package" 时，已经在上面分流到 Package 分支，不会执行到这里
    // 这里保留原有的逻辑用于其他模式（legacy）
    
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
    
    // 步骤1：检查旧打印机是否存在（不再删除）
    if check_existing_printer(&name) {
        eprintln!("[DEBUG] 检测到同名打印机已存在: {}", name);
        // 不删除，继续尝试安装（系统可能会提示已存在）
    }
    
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
        let result = add_printer_with_driver_modern(&name, &port_name, &ip_address, &selected_driver);
        
        // 如果安装成功，写入 ePrinty tag
        if result.success {
            let stable_id = generate_stable_id(&name, &path);
            match write_eprinty_tag_after_install(&name, &stable_id, &path) {
                Ok(_) => {
                    eprintln!("[INFO] ePrinty tag 写入成功: name={}, stable_id={}", name, stable_id);
                }
                Err(e) => {
                    eprintln!("[WARN] ePrinty tag 写入失败（不影响安装成功）: {}", e);
                    super::log::write_log(&format!("[Install] TAG_WRITE_FAIL name={} error={}", name, e));
                }
            }
        }
        
        Ok(result)
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
                let result = add_printer_with_driver_vbs(&name, &port_name, &ip_address, &selected_driver);
                
                // 如果安装成功，写入 ePrinty tag
                if result.success {
                    let stable_id = generate_stable_id(&name, &path);
                    match write_eprinty_tag_after_install(&name, &stable_id, &path) {
                        Ok(_) => {
                            eprintln!("[INFO] ePrinty tag 写入成功: name={}, stable_id={}", name, stable_id);
                        }
                        Err(e) => {
                            eprintln!("[WARN] ePrinty tag 写入失败（不影响安装成功）: {}", e);
                            super::log::write_log(&format!("[Install] TAG_WRITE_FAIL name={} error={}", name, e));
                        }
                    }
                }
                
                Ok(result)
            }
        }
    }
}

