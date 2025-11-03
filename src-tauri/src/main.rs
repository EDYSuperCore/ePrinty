// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde::{Deserialize, Serialize};
use std::fs;

#[cfg(windows)]
use encoding_rs::GBK;

// Windows 编码转换辅助函数（解决中文乱码问题）
#[cfg(windows)]
fn decode_windows_string(bytes: &[u8]) -> String {
    // 尝试 UTF-8 解码
    if let Ok(utf8_str) = String::from_utf8(bytes.to_vec()) {
        return utf8_str;
    }
    
    // 如果 UTF-8 失败，尝试 GBK 解码（中文 Windows 默认编码）
    // 对于非 GBK 编码，使用 lossy 解码以避免崩溃
    let (decoded, _, had_errors) = GBK.decode(bytes);
    let result = decoded.to_string();
    
    // 如果 GBK 解码也有错误，使用 UTF-8 lossy 作为后备
    if had_errors {
        String::from_utf8_lossy(bytes).to_string()
    } else {
        result
    }
}

#[cfg(not(windows))]
fn decode_windows_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrinterConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>, // 配置文件版本号（可选，兼容旧版本）
    areas: Vec<Area>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Area {
    name: String,
    printers: Vec<Printer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Printer {
    name: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>, // 打印机型号（可选）
}

#[derive(Debug, Serialize, Deserialize)]
struct InstallResult {
    success: bool,
    message: String,
    method: Option<String>, // 安装方式："VBS" 或 "Add-Printer"
}

#[derive(Debug, Serialize, Deserialize)]
struct LoadConfigResult {
    config: PrinterConfig,
    source: String, // "local" 或 "remote"
    remote_error: Option<String>,
    has_remote_update: bool, // 是否有远程更新可用
    remote_config: Option<PrinterConfig>, // 远程配置（如果有更新）
    local_version: Option<String>, // 本地版本号
    remote_version: Option<String>, // 远程版本号
}

// 加载本地配置文件，返回配置和文件路径
fn load_local_config() -> Result<(PrinterConfig, std::path::PathBuf), String> {
    use std::path::PathBuf;
    
    // 尝试从多个可能的位置加载本地配置
    // 优先级：1. 可执行文件所在目录（应用目录）- 最高优先级
    //         2. 当前工作目录
    //         3. 项目根目录（开发模式）
    
    let mut search_paths: Vec<PathBuf> = vec![];
    
    // 优先：可执行文件所在目录（打包后的应用会在这里查找）
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            search_paths.push(exe_dir.join("printer_config.json"));
        }
    }
    
    // 其次：当前工作目录
    if let Ok(current_dir) = std::env::current_dir() {
        search_paths.push(current_dir.join("printer_config.json"));
        // 也尝试上级目录（开发模式下可能在 src-tauri 目录运行）
        if let Some(parent) = current_dir.parent() {
            search_paths.push(parent.join("printer_config.json"));
        }
    }
    
    // 尝试所有可能的路径
    for config_path in &search_paths {
        if config_path.exists() {
            let content = fs::read_to_string(config_path)
                .map_err(|e| format!("读取本地配置文件失败 ({}): {}", config_path.display(), e))?;
            
            let config: PrinterConfig = serde_json::from_str(&content)
                .map_err(|e| format!("解析本地配置文件失败: {}", e))?;
            
            return Ok((config, config_path.clone()));
        }
    }
    
    // 如果没有找到，返回详细错误信息
    let mut error_msg = "未找到本地配置文件 printer_config.json。已搜索以下位置：\n".to_string();
    for path in search_paths {
        error_msg.push_str(&format!("  - {}\n", path.display()));
    }
    
    Err(error_msg)
}

// 保存配置到本地文件
fn save_config_to_local(config: &PrinterConfig, config_path: &std::path::Path) -> Result<(), String> {
    use std::io::Write;
    
    let json_content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("序列化配置失败: {}", e))?;
    
    // 确保目录存在
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }
    
    // 写入文件
    let mut file = fs::File::create(config_path)
        .map_err(|e| format!("创建配置文件失败: {}", e))?;
    
    file.write_all(json_content.as_bytes())
        .map_err(|e| format!("写入配置文件失败: {}", e))?;
    
    file.sync_all()
        .map_err(|e| format!("同步文件失败: {}", e))?;
    
    Ok(())
}

// 比较两个配置是否不同（比较版本号和配置内容）
fn config_different(local: &PrinterConfig, remote: &PrinterConfig) -> bool {
    // 先比较版本号
    if let (Some(local_v), Some(remote_v)) = (&local.version, &remote.version) {
        if local_v != remote_v {
            return true;
        }
    } else if local.version.is_some() != remote.version.is_some() {
        // 一个有版本号，一个没有
        return true;
    }
    
    // 比较配置内容（areas）
    // 如果 areas 数量不同，肯定不同
    if local.areas.len() != remote.areas.len() {
        return true;
    }
    
    // 比较每个 area 和 printer
    for (local_area, remote_area) in local.areas.iter().zip(remote.areas.iter()) {
        if local_area.name != remote_area.name || local_area.printers.len() != remote_area.printers.len() {
            return true;
        }
        
        for (local_printer, remote_printer) in local_area.printers.iter().zip(remote_area.printers.iter()) {
            if local_printer.name != remote_printer.name 
                || local_printer.path != remote_printer.path 
                || local_printer.model != remote_printer.model {
                return true;
            }
        }
    }
    
    false
}

// 加载打印机配置（默认加载本地，后台检查远程更新）
#[tauri::command]
async fn load_config() -> Result<LoadConfigResult, String> {
    // 优先加载本地配置
    match load_local_config() {
        Ok((local_config, _config_path)) => {
            let local_version = local_config.version.clone();
            
            // 本地配置加载成功，后台尝试加载远程配置进行对比
            // 使用 tokio::time::timeout 确保不会无限等待
            let remote_result = tokio::time::timeout(
                std::time::Duration::from_secs(6), // 6秒总超时
                load_remote_config()
            ).await;
            
            let (has_update, remote_config, remote_version, remote_error) = match remote_result {
                Ok(Ok(remote_config)) => {
                    let remote_version = remote_config.version.clone();
                    
                    // 比较配置是否不同
                    if config_different(&local_config, &remote_config) {
                        eprintln!("[INFO] 检测到远程配置更新 (本地: {:?}, 远程: {:?})", 
                            local_version.as_ref().unwrap_or(&"未知".to_string()),
                            remote_version.as_ref().unwrap_or(&"未知".to_string())
                        );
                        (true, Some(remote_config), remote_version, None)
                    } else {
                        (false, None, remote_version, None)
                    }
                }
                Ok(Err(e)) => {
                    // 远程加载失败，只记录错误，不影响使用
                    (false, None, None, Some(format!("远程配置加载失败: {}（已使用本地配置）", e)))
                }
                Err(_) => {
                    // 超时，使用本地配置
                    (false, None, None, Some("远程配置加载超时（已使用本地配置）".to_string()))
                }
            };
            
            Ok(LoadConfigResult {
                config: local_config,
                source: "local".to_string(),
                remote_error,
                has_remote_update: has_update,
                remote_config,
                local_version,
                remote_version,
            })
        }
        Err(local_err) => {
            // 本地配置不存在，尝试加载远程配置
            let remote_result = tokio::time::timeout(
                std::time::Duration::from_secs(6),
                load_remote_config()
            ).await;
            
            match remote_result {
                Ok(Ok(remote_config)) => {
                    let remote_version = remote_config.version.clone();
                    
                    // 远程配置加载成功，尝试保存到本地（使用可执行文件所在目录）
                    let save_path = if let Ok(exe_path) = std::env::current_exe() {
                        if let Some(exe_dir) = exe_path.parent() {
                            exe_dir.join("printer_config.json")
                        } else {
                            std::path::PathBuf::from("printer_config.json")
                        }
                    } else {
                        std::path::PathBuf::from("printer_config.json")
                    };
                    
                    // 尝试保存远程配置到本地（可选，失败不影响使用）
                    let remote_error = match save_config_to_local(&remote_config, &save_path) {
                        Ok(_) => {
                            eprintln!("[INFO] 本地配置不存在，已将远程配置保存到本地");
                            None
                        }
                        Err(save_err) => {
                            eprintln!("[WARN] 本地配置不存在，远程配置保存失败: {}", save_err);
                            Some(format!("远程配置加载成功，但保存到本地失败: {}", save_err))
                        }
                    };
                    
                    Ok(LoadConfigResult {
                        config: remote_config.clone(),
                        source: "remote".to_string(),
                        remote_error,
                        has_remote_update: false,
                        remote_config: None,
                        local_version: None,
                        remote_version,
                    })
                }
                Ok(Err(remote_err)) => {
                    // 两者都失败，返回详细错误信息
                    Err(format!("本地和远程配置都加载失败:\n本地错误: {}\n远程错误: {}", local_err, remote_err))
                }
                Err(_) => {
                    // 超时
                    Err(format!("本地配置加载失败:\n本地错误: {}\n远程配置加载超时", local_err))
                }
            }
        }
    }
}

// 确认更新配置（保存远程配置到本地）
#[tauri::command]
async fn confirm_update_config() -> Result<LoadConfigResult, String> {
    // 重新加载本地配置和远程配置
    match load_local_config() {
        Ok((local_config, config_path)) => {
            let local_version = local_config.version.clone();
            
            // 加载远程配置
            let remote_result = tokio::time::timeout(
                std::time::Duration::from_secs(6),
                load_remote_config()
            ).await;
            
            match remote_result {
                Ok(Ok(remote_config)) => {
                    let remote_version = remote_config.version.clone();
                    
                    // 保存远程配置到本地
                    match save_config_to_local(&remote_config, &config_path) {
                        Ok(_) => {
                            eprintln!("[INFO] 已确认更新，远程配置已保存到本地");
                            
                            Ok(LoadConfigResult {
                                config: remote_config,
                                source: "remote_updated".to_string(),
                                remote_error: None,
                                has_remote_update: false,
                                remote_config: None,
                                local_version,
                                remote_version,
                            })
                        }
                        Err(save_err) => {
                            Err(format!("保存配置文件失败: {}", save_err))
                        }
                    }
                }
                Ok(Err(e)) => {
                    Err(format!("加载远程配置失败: {}", e))
                }
                Err(_) => {
                    Err("加载远程配置超时".to_string())
                }
            }
        }
        Err(e) => {
            Err(format!("加载本地配置失败: {}", e))
        }
    }
}

// 加载远程配置
async fn load_remote_config() -> Result<PrinterConfig, String> {
    // 创建带超时的 HTTP 客户端（5秒超时）
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
    
    let url = "https://p.edianyun.icu/printer_config.json";
    
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("网络请求失败: {}", e);
            // 如果是超时错误，提供更友好的提示
            if e.is_timeout() {
                format!("{} (请求超时)", error_msg)
            } else {
                error_msg
            }
        })?;
    
    if !response.status().is_success() {
        return Err(format!("服务器返回错误: {}", response.status()));
    }
    
    let config: PrinterConfig = response
        .json()
        .await
        .map_err(|e| format!("解析JSON失败: {}", e))?;
    
    Ok(config)
}

// 获取本地已安装的打印机列表
#[tauri::command]
fn list_printers() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        
        use std::process::Stdio;
        
        // 设置 PowerShell 输出编码为 UTF-8，避免中文乱码，并隐藏窗口
        let output = Command::new("powershell")
            .arg("-NoProfile")
            .arg("-WindowStyle")
            .arg("Hidden")
            .args([
                "-Command",
                "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-Printer | Select-Object -ExpandProperty Name | ConvertTo-Json -Compress"
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("PYTHONIOENCODING", "utf-8")
            .output()
            .map_err(|e| format!("执行 PowerShell 命令失败: {}", e))?;
        
        if !output.status.success() {
            let error = decode_windows_string(&output.stderr);
            return Err(format!("获取打印机列表失败: {}", error));
        }
        
        let stdout = decode_windows_string(&output.stdout);
        let trimmed = stdout.trim();
        
        // 处理空输出
        if trimmed.is_empty() {
            return Ok(vec![]);
        }
        
        // PowerShell 返回的可能是单个字符串或数组
        if trimmed.starts_with('[') {
            // 是数组格式
            let printers: Vec<String> = serde_json::from_str(trimmed)
                .map_err(|e| format!("解析打印机列表失败: {}", e))?;
            Ok(printers)
        } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
            // 是单个字符串
            let printer = trimmed.trim_matches('"').to_string();
            Ok(vec![printer])
        } else {
            // 可能是空数组或格式不正确
            Ok(vec![])
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // 使用 macOS 的 lpstat 命令获取打印机列表
        // lpstat -p 列出所有打印机及其状态
        let output = Command::new("lpstat")
            .arg("-p")
            .output()
            .map_err(|e| format!("执行 lpstat 命令失败: {}", e))?;
        
        if !output.status.success() {
            // lpstat -p 在没有任何打印机时也会返回错误，所以先尝试 lpstat -d 或直接解析输出
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // 如果没有输出，可能只是没有打印机
            if stdout.trim().is_empty() && stderr.trim().is_empty() {
                return Ok(vec![]);
            }
            
            // 尝试使用 lpstat -a 获取打印机列表（列出所有可用的打印机）
            let output2 = Command::new("lpstat")
                .arg("-a")
                .output();
            
            match output2 {
                Ok(result) => {
                    let stdout2 = String::from_utf8_lossy(&result.stdout);
                    if !stdout2.trim().is_empty() {
                        // lpstat -a 输出格式: printer_name accepting requests since ...
                        let printers: Vec<String> = stdout2
                            .lines()
                            .filter_map(|line| {
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if !parts.is_empty() {
                                    Some(parts[0].to_string())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        return Ok(printers);
                    }
                }
                Err(_) => {}
            }
            
            return Err(format!("获取打印机列表失败: {}", stderr));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // lpstat -p 输出格式示例:
        // printer Ricoh_Printer is idle.  enabled since ...
        // printer HP_Printer is idle.  enabled since ...
        let printers: Vec<String> = stdout
            .lines()
            .filter_map(|line| {
                if line.starts_with("printer ") {
                    // 格式: "printer Ricoh_Printer is idle..."
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[0] == "printer" {
                        Some(parts[1].to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        
        // 如果没有找到，尝试使用 lpstat -d 获取默认打印机
        if printers.is_empty() {
            let default_output = Command::new("lpstat")
                .arg("-d")
                .output();
            
            if let Ok(result) = default_output {
                let default_stdout = String::from_utf8_lossy(&result.stdout);
                // lpstat -d 输出格式: system default destination: printer_name
                if let Some(colon_pos) = default_stdout.find(':') {
                    let printer_name = default_stdout[colon_pos + 1..].trim().to_string();
                    if !printer_name.is_empty() {
                        return Ok(vec![printer_name]);
                    }
                }
            }
        }
        
        Ok(printers)
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        // 其他平台的实现
        Err("当前仅支持 Windows 和 macOS 平台".to_string())
    }
}

// 嵌入的 prnport.vbs 脚本内容（在编译时打包进 exe）
// 注意：VBS 文件可能是 GBK/ANSI 编码，使用 include_bytes! 直接嵌入原始字节
// 写入文件时保持原始编码，因为 VBScript 需要 ANSI/GBK 编码才能正确解析
#[cfg(windows)]
const PRNPORT_VBS_BYTES: &[u8] = include_bytes!("../scripts/prnport.vbs");

// 注意：直接使用原始字节写入文件，不进行编码转换
// VBScript 需要 ANSI/GBK 编码的文件，所以保持原始字节不变

// 检测 Windows 版本（返回构建号，用于判断是否支持 Add-PrinterPort）
// 注意：GetVersionExW API 在 Windows 10+ 可能返回兼容版本信息（如 9200），不准确
// 因此优先使用 PowerShell 获取真实版本信息
#[cfg(windows)]
fn get_windows_build_number() -> Result<u32, String> {
    use std::process::Command;
    use std::process::Stdio;
    
    // 优先使用 PowerShell 检测真实构建号（更可靠）
    // 使用 Get-CimInstance 获取真实的操作系统版本信息
    match Command::new("powershell")
        .arg("-NoProfile")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-Command")
        .arg("(Get-CimInstance Win32_OperatingSystem).BuildNumber")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => {
            let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            eprintln!("[DEBUG] PowerShell Get-CimInstance 返回: '{}'", version_str);
            
            if !version_str.is_empty() {
                match version_str.parse::<u32>() {
                    Ok(build_number) => {
                        eprintln!("[DEBUG] 解析构建号成功: {}", build_number);
                        return Ok(build_number);
                    }
                    Err(e) => {
                        eprintln!("[DEBUG] 解析构建号失败: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("[DEBUG] PowerShell Get-CimInstance 执行失败: {}", e);
        }
    }
    
    // 备用方案：使用 Environment.OSVersion
    eprintln!("[DEBUG] 尝试备用方案：Environment.OSVersion");
    match Command::new("powershell")
        .arg("-NoProfile")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-Command")
        .arg("[System.Environment]::OSVersion.Version.Build")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
    {
        Ok(output) => {
            let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            eprintln!("[DEBUG] PowerShell Environment.OSVersion 返回: '{}'", version_str);
            
            if !version_str.is_empty() {
                match version_str.parse::<u32>() {
                    Ok(build_number) => {
                        eprintln!("[DEBUG] 解析构建号成功: {}", build_number);
                        return Ok(build_number);
                    }
                    Err(e) => {
                        eprintln!("[DEBUG] 解析构建号失败: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("[DEBUG] PowerShell Environment.OSVersion 执行失败: {}", e);
        }
    }
    
    // 最后的备用方案：使用 GetVersionExW（但可能不准确）
    use winapi::um::sysinfoapi::GetVersionExW;
    use winapi::um::winnt::OSVERSIONINFOW;
    
    unsafe {
        let mut os_info: OSVERSIONINFOW = std::mem::zeroed();
        os_info.dwOSVersionInfoSize = std::mem::size_of::<OSVERSIONINFOW>() as u32;
        
        if GetVersionExW(&mut os_info) != 0 {
            let build_number = os_info.dwBuildNumber;
            eprintln!("[DEBUG] GetVersionExW 返回（可能不准确）: {}", build_number);
            Ok(build_number)
        } else {
            eprintln!("[DEBUG] 所有检测方法都失败");
            Err("无法检测 Windows 构建号".to_string())
        }
    }
}

// 安装打印机（根据 Windows 版本选择安装方式）
#[tauri::command]
async fn install_printer(name: String, path: String) -> Result<InstallResult, String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        use std::io::Write;
        use std::process::Stdio;
        
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
        
        // 步骤1：删除旧打印机（如果存在）- 静默模式，忽略错误，隐藏窗口
        // rundll32 的 /q 参数会静默执行，不会显示窗口
        // 使用 CREATE_NO_WINDOW 标志确保在打包后也不显示窗口
        let _ = Command::new("rundll32")
            .args([
                "printui.dll,PrintUIEntry",
                "/dl",  // 删除本地打印机
                "/n",   // 打印机名称
                &format!("\"{}\"", name),
                "/q"    // 静默模式，不显示确认对话框
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
        
        // 根据 Windows 版本选择安装方式
        eprintln!("[DEBUG] 准备选择安装方式，use_modern_method = {}", use_modern_method);
        if use_modern_method {
            eprintln!("[DEBUG] 使用 Add-PrinterPort 方式安装");
            // Windows 10+ 使用 Add-PrinterPort + Add-Printer（现代方式）
            // 步骤1：添加打印机端口（如果不存在则创建，如果已存在则忽略错误）
            eprintln!("[DEBUG] 添加打印机端口: {}", port_name);
            let port_add_result = Command::new("powershell")
                .arg("-NoProfile")
                .arg("-WindowStyle")
                .arg("Hidden")
                .args([
                    "-Command",
                    &format!(
                        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; try {{ Add-PrinterPort -Name '{}' -PrinterHostAddress '{}' -ErrorAction Stop; Write-Output 'PortSuccess' }} catch {{ if ($_.Exception.Message -like '*already exists*' -or $_.Exception.Message -like '*已存在*') {{ Write-Output 'PortExists' }} else {{ Write-Error $_.Exception.Message }} }}",
                        port_name.replace("'", "''"),
                        ip_address.replace("'", "''")
                    )
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output();
            
            match port_add_result {
                Ok(port_result) => {
                    let port_stdout = decode_windows_string(&port_result.stdout);
                    let port_stderr = decode_windows_string(&port_result.stderr);
                    
                    // 检查是否成功或端口已存在
                    let port_success = port_result.status.success() 
                        || port_stdout.contains("PortSuccess")
                        || port_stdout.contains("PortExists")
                        || port_stderr.contains("already exists")
                        || port_stderr.contains("已存在");
                    
                    if !port_success {
                        // 端口添加失败
                        return Ok(InstallResult {
                            success: false,
                            message: format!("添加打印机端口失败: {}。错误信息: {}。请确保有管理员权限。", port_stdout, port_stderr),
                            method: Some("Add-Printer".to_string()),
                        });
                    } else {
                        eprintln!("[DEBUG] 端口添加成功或已存在: {}", port_stdout);
                    }
                    
                    // 验证端口确实存在（重试几次，因为端口创建可能需要时间）
                    let mut port_verified = false;
                    for attempt in 1..=3 {
                        eprintln!("[DEBUG] 验证端口存在（尝试 {}/3）", attempt);
                        
                        let verify_port = Command::new("powershell")
                            .arg("-NoProfile")
                            .arg("-WindowStyle")
                            .arg("Hidden")
                            .args([
                                "-Command",
                                &format!(
                                    "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $port = Get-PrinterPort -Name '{}' -ErrorAction SilentlyContinue; if ($port) {{ Write-Output 'PortVerified' }} else {{ Write-Error 'Port not found' }}",
                                    port_name.replace("'", "''")
                                )
                            ])
                            .stdin(Stdio::null())
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .creation_flags(0x08000000) // CREATE_NO_WINDOW
                            .output();
                        
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
                            Err(_) => {
                                eprintln!("[DEBUG] 无法验证端口，等待后重试...");
                                if attempt < 3 {
                                    std::thread::sleep(std::time::Duration::from_millis(500));
                                }
                            }
                        }
                    }
                    
                    if !port_verified {
                        eprintln!("[DEBUG] 警告：端口验证失败，但继续尝试添加打印机");
                    }
                }
                Err(e) => {
                    return Ok(InstallResult {
                        success: false,
                        message: format!("执行 Add-PrinterPort 命令失败: {}", e),
                        method: Some("Add-Printer".to_string()),
                    });
                }
            }
            
            // 查找通用驱动并添加打印机
            // 改进的错误处理：捕获更详细的错误信息
            let printer_output = Command::new("powershell")
                .arg("-NoProfile")
                .arg("-WindowStyle")
                .arg("Hidden")
                .args([
                    "-Command",
                    &format!(
                        r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;
$ErrorActionPreference = 'Stop';
$driverName = $null;
$errorMsg = '';

# 尝试查找通用驱动
try {{
    $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
        $_.Name -like '*Generic*' -or 
        $_.Name -like '*Text*' -or 
        $_.Name -like '*HP*' -or 
        $_.Name -like '*RICOH*' -or 
        $_.Name -like '*PCL*' -or 
        $_.Name -like '*PostScript*'
    }} | Select-Object -First 1;
    
    if ($drivers) {{
        $driverName = $drivers.Name;
        Write-Output ('找到驱动: ' + $driverName);
    }}
}} catch {{
    $errorMsg += ('查找驱动错误: ' + $_.Exception.Message + '; ');
}}

# 如果没有找到通用驱动，尝试使用第一个可用驱动
if (-not $driverName) {{
    try {{
        $allDrivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Select-Object -First 1;
        if ($allDrivers) {{
            $driverName = $allDrivers.Name;
            Write-Output ('使用默认驱动: ' + $driverName);
        }} else {{
            Write-Error '系统中没有可用的打印机驱动。请先安装打印机驱动。';
            exit 1;
        }}
    }} catch {{
        Write-Error ('获取驱动列表失败: ' + $_.Exception.Message);
        exit 1;
    }}
}}

# 尝试安装打印机
if ($driverName) {{
    try {{
        Add-Printer -Name '{}' -DriverName $driverName -PortName '{}' -ErrorAction Stop;
        Write-Output 'Success';
    }} catch {{
        $fullError = $_.Exception.GetType().FullName + ': ' + $_.Exception.Message;
        if ($_.Exception.InnerException) {{
            $fullError += ' | 内部错误: ' + $_.Exception.InnerException.Message;
        }}
        Write-Error $fullError;
        exit 1;
    }}
}} else {{
    Write-Error '无法找到可用的打印机驱动';
    exit 1;
}}
"#,
                        name.replace("'", "''"),
                        port_name.replace("'", "''")
                    )
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output();
            
            match printer_output {
                Ok(printer_result) => {
                    let printer_stdout = decode_windows_string(&printer_result.stdout);
                    let printer_stderr = decode_windows_string(&printer_result.stderr);
                    
                    eprintln!("[DEBUG] Add-Printer stdout: {}", printer_stdout);
                    eprintln!("[DEBUG] Add-Printer stderr: {}", printer_stderr);
                    eprintln!("[DEBUG] Add-Printer exit code: {:?}", printer_result.status.code());
                    
                    if printer_result.status.success() || printer_stdout.contains("Success") {
                        Ok(InstallResult {
                            success: true,
                            message: format!("打印机 {} ({}) 安装成功", name, ip_address),
                            method: Some("Add-Printer".to_string()),
                        })
                    } else {
                        // 组合详细的错误信息
                        let mut error_details = String::new();
                        if !printer_stdout.trim().is_empty() {
                            error_details.push_str(&format!("标准输出: {}; ", printer_stdout.trim()));
                        }
                        if !printer_stderr.trim().is_empty() {
                            error_details.push_str(&format!("错误输出: {}", printer_stderr.trim()));
                        }
                        if error_details.is_empty() {
                            error_details = format!("退出代码: {:?}", printer_result.status.code().unwrap_or(-1));
                        }
                        
                        Ok(InstallResult {
                            success: false,
                            message: format!("端口添加成功，但打印机安装失败。{}请确保系统中已安装打印机驱动，或联系管理员安装驱动。", error_details),
                            method: Some("Add-Printer".to_string()),
                        })
                    }
                }
                Err(e) => {
                    Ok(InstallResult {
                        success: false,
                        message: format!("端口添加成功，但执行 Add-Printer 命令失败: {}。请确保系统中已安装打印机驱动，或联系管理员安装驱动。", e),
                        method: Some("Add-Printer".to_string()),
                    })
                }
            }
        } else {
            eprintln!("[DEBUG] 使用 VBS 脚本方式安装");
            // Windows 7/8 使用 VBS 脚本方式（传统方式）
            // 将嵌入的 VBS 脚本写入临时文件
            // 重要：直接写入原始字节，不要进行编码转换，因为 VBScript 需要 ANSI/GBK 编码
            let temp_dir = std::env::temp_dir();
            let script_path = temp_dir.join("prnport.vbs");
            
            // 直接写入原始字节（保持原始编码，ANSI/GBK）
            let mut file = fs::File::create(&script_path)
                .map_err(|e| format!("创建临时脚本文件失败: {}", e))?;
            file.write_all(PRNPORT_VBS_BYTES)
                .map_err(|e| format!("写入脚本内容失败: {}", e))?;
            file.sync_all()
                .map_err(|e| format!("同步脚本文件失败: {}", e))?;
            drop(file); // 确保文件已关闭
            
            // 步骤2：使用 cscript 运行 prnport.vbs 脚本添加端口（隐藏窗口）
            // 参考 C# 代码：cscript prnport.vbs -a -r IP_192.168.x.x -h 192.168.x.x -o raw
            // 注意：移除 //B 参数以便捕获错误信息
            let output = Command::new("cscript")
                .args([
                    "//NoLogo",  // 不显示脚本横幅
                    script_path.to_str().unwrap(),
                    "-a",        // 添加端口
                    "-r",        // 端口名
                    &port_name,  // 端口名称
                    "-h",        // IP地址
                    &ip_address, // IP地址值
                    "-o",        // 输出类型
                    "raw"        // raw 类型（参考 C# 代码）
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output();
            
            match output {
                Ok(result) => {
                    // 执行完毕后删除临时文件
                    let _ = std::fs::remove_file(&script_path);
                    
                    let stdout = decode_windows_string(&result.stdout);
                    let stderr = decode_windows_string(&result.stderr);
                    
                    if result.status.success() {
                        // 端口添加成功，现在使用 PowerShell Add-Printer 安装打印机
                        // 使用改进的错误处理（与 Add-Printer 方式一致）
                        let ps_output = Command::new("powershell")
                            .arg("-NoProfile")
                            .arg("-WindowStyle")
                            .arg("Hidden")
                            .args([
                                "-Command",
                                &format!(
                                    r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;
$ErrorActionPreference = 'Stop';
$driverName = $null;
$port = '{}';

# 尝试查找通用驱动
try {{
    $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
        $_.Name -like '*Generic*' -or 
        $_.Name -like '*Text*' -or 
        $_.Name -like '*HP*' -or 
        $_.Name -like '*RICOH*' -or 
        $_.Name -like '*PCL*' -or 
        $_.Name -like '*PostScript*'
    }} | Select-Object -First 1;
    
    if ($drivers) {{
        $driverName = $drivers.Name;
        Write-Output ('找到驱动: ' + $driverName);
    }}
}} catch {{
    Write-Error ('查找驱动错误: ' + $_.Exception.Message);
}}

# 如果没有找到通用驱动，尝试使用第一个可用驱动
if (-not $driverName) {{
    try {{
        $allDrivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Select-Object -First 1;
        if ($allDrivers) {{
            $driverName = $allDrivers.Name;
            Write-Output ('使用默认驱动: ' + $driverName);
        }} else {{
            Write-Error '系统中没有可用的打印机驱动。请先安装打印机驱动。';
            exit 1;
        }}
    }} catch {{
        Write-Error ('获取驱动列表失败: ' + $_.Exception.Message);
        exit 1;
    }}
}}

# 尝试安装打印机
if ($driverName) {{
    try {{
        Add-Printer -Name '{}' -PortName $port -DriverName $driverName -ErrorAction Stop;
        Write-Output 'Success';
    }} catch {{
        $fullError = $_.Exception.GetType().FullName + ': ' + $_.Exception.Message;
        if ($_.Exception.InnerException) {{
            $fullError += ' | 内部错误: ' + $_.Exception.InnerException.Message;
        }}
        Write-Error $fullError;
        exit 1;
    }}
}} else {{
    Write-Error '无法找到可用的打印机驱动';
    exit 1;
}}
"#,
                                    port_name,
                                    name.replace("'", "''")
                                )
                            ])
                            .stdin(Stdio::null())
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .creation_flags(0x08000000) // CREATE_NO_WINDOW
                            .output();
                        
                        match ps_output {
                            Ok(ps_result) => {
                                let ps_stdout = decode_windows_string(&ps_result.stdout);
                                let ps_stderr = decode_windows_string(&ps_result.stderr);
                                
                                eprintln!("[DEBUG] VBS Add-Printer stdout: {}", ps_stdout);
                                eprintln!("[DEBUG] VBS Add-Printer stderr: {}", ps_stderr);
                                eprintln!("[DEBUG] VBS Add-Printer exit code: {:?}", ps_result.status.code());
                                
                                if ps_result.status.success() || ps_stdout.contains("Success") {
                                    Ok(InstallResult {
                                        success: true,
                                        message: format!("打印机 {} ({}) 安装成功", name, ip_address),
                                        method: Some("VBS".to_string()),
                                    })
                                } else {
                                    // 组合详细的错误信息
                                    let mut error_details = String::new();
                                    if !ps_stdout.trim().is_empty() {
                                        error_details.push_str(&format!("标准输出: {}; ", ps_stdout.trim()));
                                    }
                                    if !ps_stderr.trim().is_empty() {
                                        error_details.push_str(&format!("错误输出: {}", ps_stderr.trim()));
                                    }
                                    if error_details.is_empty() {
                                        error_details = format!("退出代码: {:?}", ps_result.status.code().unwrap_or(-1));
                                    }
                                    
                                    Ok(InstallResult {
                                        success: false,
                                        message: format!("端口添加成功，但打印机安装失败。{}请确保系统中已安装打印机驱动，或联系管理员安装驱动。", error_details),
                                        method: Some("VBS".to_string()),
                                    })
                                }
                            }
                            Err(e) => {
                                Ok(InstallResult {
                                    success: false,
                                    message: format!("端口添加成功，但执行 PowerShell 命令失败: {}。请确保系统中已安装打印机驱动，或联系管理员安装驱动。", e),
                                    method: Some("VBS".to_string()),
                                })
                            }
                        }
                    } else {
                        // 组合详细的错误信息
                        let error_details = if stderr.trim().is_empty() && stdout.trim().is_empty() {
                            format!("cscript 退出代码: {}", result.status.code().unwrap_or(-1))
                        } else {
                            format!("错误输出: {} | 标准输出: {}", 
                                if stderr.trim().is_empty() { "无" } else { &stderr },
                                if stdout.trim().is_empty() { "无" } else { &stdout }
                            )
                        };
                        
                        Ok(InstallResult {
                            success: false,
                            message: format!("添加打印机端口失败: {} | 退出代码: {}", 
                                error_details,
                                result.status.code().unwrap_or(-1)
                            ),
                            method: Some("VBS".to_string()),
                        })
                    }
                }
                Err(e) => {
                    // 执行失败时也删除临时文件
                    let _ = std::fs::remove_file(&script_path);
                    
                    // 检查脚本文件是否存在
                    let script_exists = script_path.exists();
                    let script_info = if script_exists {
                        format!("脚本文件存在，大小: {} 字节", 
                            std::fs::metadata(&script_path)
                                .map(|m| m.len())
                                .unwrap_or(0)
                        )
                    } else {
                        "脚本文件不存在".to_string()
                    };
                    
                    Ok(InstallResult {
                        success: false,
                        message: format!("执行 prnport.vbs 脚本失败: {} | {}", e, script_info),
                        method: Some("VBS".to_string()),
                    })
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // macOS 使用 lpadmin 命令安装打印机
        // 参考 Objective-C 代码的逻辑：
        // 1. 复制 PPD 文件（如果有）或使用系统自带的驱动
        // 2. 使用 lpadmin 安装打印机
        
        // 从路径中提取 IP 地址（格式：\\192.168.x.x 或 lpd://192.168.x.x）
        let ip_address = path
            .trim_start_matches("\\\\")
            .trim_start_matches("lpd://")
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .to_string();
        
        // 构建 lpd:// URL（macOS 通常使用 lpd 协议）
        let printer_url = format!("lpd://{}", ip_address);
        
        // 使用 lpadmin 命令安装打印机
        // lpadmin -p <printer_name> -E -v <printer_url> -P <ppd_file> -D <description>
        // -E: 启用打印机
        // -v: 指定打印机地址
        // -P: 指定 PPD 文件（可选，如果不指定会使用通用驱动）
        // -D: 打印机描述
        
        // 先尝试查找是否有 PPD 文件在资源目录中
        // 参考 Objective-C 代码：从 NSBundle mainBundle 查找 .txt 文件，复制到用户目录为 .ppd
        let mut ppd_path: Option<String> = None;
        
        // 获取用户主目录
        let home_dir = std::env::var("HOME")
            .map_err(|_| "无法获取用户主目录".to_string())?;
        
        if let Ok(exe_path) = std::env::current_exe() {
            // 查找资源目录
            // macOS app bundle 结构: App.app/Contents/Resources/
            // 开发模式下可能在 src-tauri/target/... 目录
            let mut resources_dir: Option<std::path::PathBuf> = None;
            
            if let Some(exe_dir) = exe_path.parent() {
                // 如果是打包的 app，exe 在 Contents/MacOS/，Resources 在 Contents/Resources/
                if exe_dir.ends_with("MacOS") {
                    resources_dir = exe_dir.parent().map(|p| p.join("Resources"));
                } else {
                    // 开发模式或其他情况
                    resources_dir = Some(exe_dir.join("Resources"));
                    if !resources_dir.as_ref().unwrap().exists() {
                        // 也尝试直接使用 exe_dir（开发模式）
                        resources_dir = Some(exe_dir.to_path_buf());
                    }
                }
            }
            
            // 查找可能的 PPD 文件（.txt 或 .ppd）
            let possible_ppd_names = ["ricoh320", "generic", "PostScript"];
            
            if let Some(res_dir) = resources_dir {
                for ppd_base in &possible_ppd_names {
                    // 先查找 .txt 文件（参考原代码：ricoh320.txt）
                    let txt_file = res_dir.join(format!("{}.txt", ppd_base));
                    if txt_file.exists() {
                        // 复制到用户目录并改为 .ppd 扩展名
                        let target_ppd = format!("{}/{}.ppd", home_dir, ppd_base);
                        
                        match fs::copy(&txt_file, &target_ppd) {
                            Ok(_) => {
                                ppd_path = Some(target_ppd);
                                break;
                            }
                            Err(_) => {
                                // 复制失败，尝试直接使用原文件
                                ppd_path = Some(txt_file.to_string_lossy().to_string());
                                break;
                            }
                        }
                    }
                    
                    // 查找 .ppd 文件
                    let ppd_file = res_dir.join(format!("{}.ppd", ppd_base));
                    if ppd_file.exists() {
                        ppd_path = Some(ppd_file.to_string_lossy().to_string());
                        break;
                    }
                }
            }
            
            // 也尝试在当前目录和 scripts 目录查找
            if ppd_path.is_none() {
                if let Some(exe_dir) = exe_path.parent() {
                    let possible_dirs = vec![
                        exe_dir.join("Resources"),
                        exe_dir.join("scripts"),
                        exe_dir.to_path_buf(),
                    ];
                    
                    for dir in possible_dirs {
                        for ppd_base in &possible_ppd_names {
                            let txt_file = dir.join(format!("{}.txt", ppd_base));
                            if txt_file.exists() {
                                let target_ppd = format!("{}/{}.ppd", home_dir, ppd_base);
                                if fs::copy(&txt_file, &target_ppd).is_ok() {
                                    ppd_path = Some(target_ppd);
                                    break;
                                }
                            }
                            
                            let ppd_file = dir.join(format!("{}.ppd", ppd_base));
                            if ppd_file.exists() {
                                ppd_path = Some(ppd_file.to_string_lossy().to_string());
                                break;
                            }
                        }
                        if ppd_path.is_some() {
                            break;
                        }
                    }
                }
            }
        }
        
        // 构建 lpadmin 命令
        let mut lpadmin_cmd = Command::new("lpadmin");
        
        // 基本参数
        lpadmin_cmd
            .arg("-p")
            .arg(&name)
            .arg("-E")  // 启用打印机
            .arg("-v")
            .arg(&printer_url)
            .arg("-D")
            .arg(&name);  // 打印机描述
        
        // 如果找到 PPD 文件，使用它；否则让系统自动选择驱动
        if let Some(ppd) = ppd_path {
            lpadmin_cmd.arg("-P").arg(ppd);
        } else {
            // 不指定 -P，系统会使用通用 PostScript 驱动
            // 或者可以使用 -m everywhere 让系统自动选择
            lpadmin_cmd.arg("-m").arg("everywhere");
        }
        
        // 执行 lpadmin 命令
        let output = lpadmin_cmd
            .output()
            .map_err(|e| format!("执行 lpadmin 命令失败: {}", e))?;
        
        if output.status.success() {
            Ok(InstallResult {
                success: true,
                message: format!("打印机 {} ({}) 安装成功", name, ip_address),
                method: Some("macOS".to_string()),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let error_msg = if !stderr.is_empty() {
                stderr.to_string()
            } else if !stdout.is_empty() {
                stdout.to_string()
            } else {
                "未知错误".to_string()
            };
            
            Ok(InstallResult {
                success: false,
                message: format!("安装打印机失败: {}。请确保已授予管理员权限，或联系管理员。", error_msg),
                method: Some("macOS".to_string()),
            })
        }
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("当前仅支持 Windows 和 macOS 平台".to_string())
    }
}

#[tauri::command]
fn open_url(url: String) -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::ptr;
        use winapi::um::shellapi::ShellExecuteW;
        use winapi::um::winuser::SW_SHOWNORMAL;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        
        // 将 URL 字符串转换为宽字符串
        let url_wide: Vec<u16> = OsStr::new(&url)
            .encode_wide()
            .chain(Some(0))
            .collect();
        
        let url_ptr = url_wide.as_ptr();
        let operation: Vec<u16> = OsStr::new("open")
            .encode_wide()
            .chain(Some(0))
            .collect();
        
        // 使用 ShellExecuteW 打开 URL，不显示命令行窗口
        let result = unsafe {
            ShellExecuteW(
                ptr::null_mut(),
                operation.as_ptr(),
                url_ptr,
                ptr::null(),
                ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        
        // ShellExecuteW 返回大于 32 表示成功
        if result as usize > 32 {
            Ok("已打开".to_string())
        } else {
            Err(format!("无法打开 URL，错误代码: {}", result as i32))
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // macOS 使用 open 命令打开 URL
        let output = Command::new("open")
            .arg(&url)
            .output()
            .map_err(|e| format!("执行命令失败: {}", e))?;
        
        if output.status.success() {
            Ok("已打开".to_string())
        } else {
            let error = String::from_utf8_lossy(&output.stderr);
            Err(format!("无法打开 URL: {}", error))
        }
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("当前平台不支持打开 URL".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_config,
            list_printers,
            install_printer,
            open_url,
            confirm_update_config
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

