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

// ============================================================================
// 常量定义
// ============================================================================

// 配置文件相关常量
const CONFIG_FILE_NAME: &str = "printer_config.json";
const CONFIG_REMOTE_URL: &str = "https://p.edianyun.icu/printer_config.json";
const VERSION_CONFIG_REMOTE_URL: &str = "https://p.edianyun.icu/version_config.json";

// Windows 相关常量
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;
#[cfg(windows)]
const ERROR_FILE_NOT_FOUND: &str = "0x80070002";

// WebView2 下载链接
const WEBVIEW2_DOWNLOAD_URL: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

// 网络请求超时时间（秒）
const HTTP_TIMEOUT_SECS: u64 = 5;
const HTTP_TIMEOUT_DOWNLOAD_SECS: u64 = 300; // 5分钟，用于下载更新文件

// PowerShell 命令执行超时时间（秒）
const POWERSHELL_TIMEOUT_SECS: u64 = 120; // 2分钟，用于打印机安装相关命令

// 打印机驱动相关常量
const DRIVER_GENERIC_TEXT_ONLY: &str = "Generic / Text Only";
const DRIVER_UNIVERSAL_PRINT_CLASS: &str = "Universal Print Class Driver";

// 打印机品牌匹配关键词（中文和英文）
const BRAND_KEYWORDS: &[(&str, &[&str])] = &[
    ("RICOH", &["RICOH", "理光"]),
    ("HP", &["HP", "HEWLETT", "惠普"]),
    ("CANON", &["CANON", "佳能"]),
    ("EPSON", &["EPSON", "爱普生"]),
    ("XEROX", &["XEROX", "施乐"]),
    ("KYOCERA", &["KYOCERA", "京瓷"]),
    ("BROTHER", &["BROTHER", "兄弟"]),
];

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
    #[serde(skip_serializing_if = "Option::is_none")]
    driver_path: Option<String>, // 驱动路径（可选，相对于应用目录）
}

#[derive(Debug, Serialize, Deserialize)]
struct InstallResult {
    success: bool,
    message: String,
    method: Option<String>, // 安装方式："VBS" 或 "Add-Printer"
    stdout: Option<String>,  // PowerShell 标准输出
    stderr: Option<String>,  // PowerShell 错误输出
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

// 版本检查配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionConfig {
    pub app_name: String,
    pub app_version: String,
    pub build_number: u32,
    pub release_date: String,
    pub update_url: Option<String>,
    pub update_type: String, // "manual" 或 "auto"
    pub update_description: Option<String>,
    pub changelog: Option<Vec<ChangelogEntry>>,
    pub force_update: bool,
    pub min_supported_version: Option<String>,
    pub download_size: Option<String>,
    pub checksum: Option<Checksum>,
    pub printer_config: Option<PrinterConfigInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogEntry {
    pub version: String,
    pub date: String,
    pub changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checksum {
    pub algorithm: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrinterConfigInfo {
    pub version: String,
    pub url: String,
}

// 版本检查结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCheckResult {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub update_url: Option<String>,
    pub update_type: String,
    pub update_description: Option<String>,
    pub force_update: bool,
    pub changelog: Option<Vec<ChangelogEntry>>,
    pub download_size: Option<String>,
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
            search_paths.push(exe_dir.join(CONFIG_FILE_NAME));
        }
    }
    
    // 其次：当前工作目录
    if let Ok(current_dir) = std::env::current_dir() {
        search_paths.push(current_dir.join(CONFIG_FILE_NAME));
        // 也尝试上级目录（开发模式下可能在 src-tauri 目录运行）
        if let Some(parent) = current_dir.parent() {
            search_paths.push(parent.join(CONFIG_FILE_NAME));
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
    let mut error_msg = format!("未找到本地配置文件 {}。已搜索以下位置：\n", CONFIG_FILE_NAME);
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
                            exe_dir.join(CONFIG_FILE_NAME)
                        } else {
                            std::path::PathBuf::from(CONFIG_FILE_NAME)
                        }
                    } else {
                        std::path::PathBuf::from(CONFIG_FILE_NAME)
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
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
    
    let url = CONFIG_REMOTE_URL;
    
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
            
            if !version_str.is_empty() {
                if let Ok(build_number) = version_str.parse::<u32>() {
                    return Ok(build_number);
                }
            }
        }
        Err(_) => {}
    }
    
    // 备用方案：使用 Environment.OSVersion
    match Command::new("powershell")
        .arg("-NoProfile")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-Command")
        .arg("[System.Environment]::OSVersion.Version.Build")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
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

// 检查系统中是否有可用的打印机驱动
#[cfg(windows)]
fn check_printer_driver_available() -> Result<bool, String> {
    use std::process::Command;
    use std::process::Stdio;
    
    // 使用 PowerShell 查询系统中是否有可用的打印机驱动
    let output = Command::new("powershell")
        .arg("-NoProfile")
        .arg("-WindowStyle")
        .arg("Hidden")
        .args([
            "-Command",
            r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;
$ErrorActionPreference = 'SilentlyContinue';
$drivers = Get-PrinterDriver -ErrorAction SilentlyContinue;
if ($drivers -and $drivers.Count -gt 0) {
    Write-Output 'DriverAvailable';
} else {
    Write-Output 'NoDriver';
}
"#
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    
    match output {
        Ok(result) => {
            let stdout = decode_windows_string(&result.stdout);
            let has_driver = stdout.contains("DriverAvailable");
            Ok(has_driver)
        }
        Err(_) => {
            // 如果检查失败，假设有驱动（不阻止安装尝试）
            Ok(true)
        }
    }
}

// 安装打印机（根据 Windows 版本选择安装方式）
// 注意：尝试使用 camelCase 参数名 driverPath，因为 Tauri 可能对带下划线的参数名 driver_path 有问题
#[tauri::command]
#[allow(non_snake_case)]  // 允许使用 camelCase，因为前端传递的是 driverPath
async fn install_printer(
    name: String, 
    path: String, 
    driverPath: Option<String>,  // 改为 camelCase，看看是否能解决问题
    model: Option<String>
) -> Result<InstallResult, String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        use std::io::Write;
        use std::process::Stdio;
        
        // 兼容两种参数名：driverPath (camelCase) 和 driver_path (snake_case)
        let driver_path = driverPath.clone();
        
        // 步骤0：在安装之前先检查是否有可用的打印机驱动
        match check_printer_driver_available() {
            Ok(has_driver) => {
                if !has_driver {
                    return Ok(InstallResult {
                        success: false,
                        message: "系统中没有可用的打印机驱动程序。请先安装打印机驱动程序，或联系IT管理员安装驱动后再试。".to_string(),
                        method: Some("预检查".to_string()),
                        stdout: None,
                        stderr: None,
                    });
                }
            }
            Err(_) => {
                // 检查失败时不阻止安装，让后续流程处理
            }
        }
        
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
            true // 默认使用现代方法
        } else {
            windows_build >= 10240 // Windows 10+ 使用新方法（包括 Windows 11）
        };
        
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
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        
        // 根据 Windows 版本选择安装方式
        if use_modern_method {
            // Windows 10+ 使用 Add-PrinterPort + Add-Printer（现代方式）
            // 步骤1：添加打印机端口（如果不存在则创建，如果已存在则忽略错误）
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
                .creation_flags(CREATE_NO_WINDOW)
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
                            stdout: Some(port_stdout.clone()),
                            stderr: Some(port_stderr.clone()),
                        });
                    }
                    
                    // 验证端口确实存在（重试几次，因为端口创建可能需要时间）
                    let mut _port_verified = false;
                    for _attempt in 1..=3 {
                        
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
                            .creation_flags(CREATE_NO_WINDOW)
                            .output();
                        
                        match verify_port {
                            Ok(verify_result) => {
                                let verify_stdout = decode_windows_string(&verify_result.stdout);
                                if verify_stdout.contains("PortVerified") {
                                    _port_verified = true;
                                    break;
                                } else {
                                    std::thread::sleep(std::time::Duration::from_millis(500));
                                }
                            }
                            Err(_) => {
                                std::thread::sleep(std::time::Duration::from_millis(500));
                            }
                        }
                    }
                }
                Err(e) => {
                    return Ok(InstallResult {
                        success: false,
                        message: format!("执行 Add-PrinterPort 命令失败: {}", e),
                        method: Some("Add-Printer".to_string()),
                        stdout: None,
                        stderr: Some(format!("执行 Add-PrinterPort 命令失败: {}", e)),
                    });
                }
            }
            
            // 查找驱动并添加打印机
            // 改进的错误处理：优先使用配置的驱动路径，否则根据打印机名称智能选择驱动
            // 构建驱动选择脚本
            // 计算应用目录（用于解析驱动路径）
            let exe_dir = std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
            
            // 计算完整的驱动路径（如果提供了配置的驱动路径）
            // 在 Rust 中计算完整路径，避免 PowerShell 获取错误的进程路径
            let full_driver_path = driver_path.as_ref().map(|dp| {
                exe_dir.join(dp)
            });
            
            let _driver_path_script = if let Some(ref full_path) = full_driver_path {
                // 使用配置的驱动路径安装驱动（完整路径已在 Rust 中计算）
                format!(
                    r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;
$ErrorActionPreference = 'Stop';
$driverName = $null;
$errorMsg = '';
$driverPath = '{}';
$printerName = '{}';
$printerNameUpper = $printerName.ToUpper();

# 尝试从配置的驱动路径安装驱动
try {{
    $driverInfo = Get-ItemProperty -Path $driverPath -ErrorAction SilentlyContinue;
    if ($driverInfo) {{
        # 使用 pnputil 安装 INF 驱动（如果需要）
        # 然后使用 Add-PrinterDriver 安装驱动
        $infPath = $driverPath;
        
        # 尝试从 INF 文件提取驱动名称
        $infContent = Get-Content $infPath -Raw -ErrorAction SilentlyContinue;
        if ($infContent -match 'DriverName\s*=\s*"([^"]+)"' -or $infContent -match 'ClassDriver\s*=\s*"([^"]+)"') {{
            $driverName = $matches[1];
            Write-Output ('从 INF 文件提取驱动名称: ' + $driverName);
        }} else {{
            # 如果无法提取，尝试安装驱动并获取名称
            $installResult = pnputil.exe /add-driver $infPath /install 2>&1;
            if ($LASTEXITCODE -eq 0) {{
                # 安装成功后，查找对应的驱动名称
                $infBaseName = [System.IO.Path]::GetFileNameWithoutExtension($infPath);
                $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                    $_.InfPath -like ('*' + $infBaseName + '*') -or
                    $_.Name -like ('*' + $infBaseName + '*')
                }} | Select-Object -First 1;
                
                if ($drivers) {{
                    $driverName = $drivers.Name;
                    Write-Output ('找到已安装的驱动: ' + $driverName);
                }}
            }}
        }}
        
        # 如果仍然没有驱动名称，尝试直接使用 INF 路径
        if (-not $driverName) {{
            # 尝试使用 Add-PrinterDriver 安装驱动
            try {{
                Add-PrinterDriver -InfPath $infPath -ErrorAction Stop;
                # 安装后查找驱动名称
                $infBaseName = [System.IO.Path]::GetFileNameWithoutExtension($infPath);
                $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                    $_.InfPath -like ('*' + $infBaseName + '*') -or
                    $_.Name -like ('*' + $infBaseName + '*')
                }} | Select-Object -First 1;
                
                if ($drivers) {{
                    $driverName = $drivers.Name;
                    Write-Output ('找到已安装的驱动: ' + $driverName);
                }}
            }} catch {{
                $errorMsg += ('安装驱动失败: ' + $_.Exception.Message + '; ');
            }}
        }}
    }}
}} catch {{
    $errorMsg += ('处理驱动路径错误: ' + $_.Exception.Message + '; ');
}}

# 如果配置的驱动路径失败，继续使用智能选择逻辑
"#,
                        full_path.to_string_lossy().replace("'", "''"),
                        name.replace("'", "''")
                    )
            } else {
                String::new() // 没有配置驱动路径或路径不存在，使用默认逻辑
            };
            
            // 由于 PowerShell 命令可能太长，导致 Windows 错误 206（文件名或扩展名太长）
            // 改为使用临时脚本文件的方式执行 PowerShell 脚本
            let temp_dir = std::env::temp_dir();
            let script_path = temp_dir.join(format!("install_printer_{}.ps1", std::process::id()));
            
            let ps_script = format!(
                r#"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8;
$ErrorActionPreference = 'Stop';
$driverName = $null;
$errorMsg = '';
$port = '{}';
$printerName = '{}';
$printerNameUpper = $printerName.ToUpper();
$driverPathFromConfig = '{}';
$printerModel = '{}';
$printerModelUpper = if ($printerModel -and $printerModel -ne '') {{ $printerModel.ToUpper() }} else {{ '' }};

# 步骤1: 确认打印机的型号和品牌
$printerBrand = $null;
# 首先从型号（model）中检测，型号通常包含更准确的品牌信息
if ($printerModelUpper -match 'RICOH|理光') {{
    $printerBrand = 'RICOH';
}} elseif ($printerModelUpper -match 'HP|HEWLETT|惠普') {{
    $printerBrand = 'HP';
}} elseif ($printerModelUpper -match 'CANON|佳能') {{
    $printerBrand = 'CANON';
}} elseif ($printerModelUpper -match 'EPSON|爱普生') {{
    $printerBrand = 'EPSON';
}} elseif ($printerModelUpper -match 'XEROX|施乐') {{
    $printerBrand = 'XEROX';
}} elseif ($printerModelUpper -match 'KYOCERA|京瓷') {{
    $printerBrand = 'KYOCERA';
}} elseif ($printerModelUpper -match 'BROTHER|兄弟') {{
    $printerBrand = 'BROTHER';
}}

# 如果型号中没有找到品牌，尝试从名称中检测
if (-not $printerBrand) {{
    if ($printerNameUpper -match 'RICOH|理光') {{
        $printerBrand = 'RICOH';
    }} elseif ($printerNameUpper -match 'HP|HEWLETT|惠普') {{
        $printerBrand = 'HP';
    }} elseif ($printerNameUpper -match 'CANON|佳能') {{
        $printerBrand = 'CANON';
    }} elseif ($printerNameUpper -match 'EPSON|爱普生') {{
        $printerBrand = 'EPSON';
    }} elseif ($printerNameUpper -match 'XEROX|施乐') {{
        $printerBrand = 'XEROX';
    }} elseif ($printerNameUpper -match 'KYOCERA|京瓷') {{
        $printerBrand = 'KYOCERA';
    }} elseif ($printerNameUpper -match 'BROTHER|兄弟') {{
        $printerBrand = 'BROTHER';
    }}
}}


# 步骤2: 在系统已有的打印机驱动中查找该打印机的品牌和型号
if ($printerBrand) {{
    try {{
        # 首先尝试精确匹配品牌和型号
        $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
            $_.Name -like ('*' + $printerBrand + '*') -and
            ($printerModel -eq '' -or $_.Name -like ('*' + $printerModel + '*') -or $_.Name -like ('*' + [System.IO.Path]::GetFileNameWithoutExtension($printerModel) + '*'))
        }} | Select-Object -First 1;
        
        if (-not $drivers) {{
            # 如果精确匹配失败，尝试只匹配品牌
            $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                $_.Name -like ('*' + $printerBrand + '*')
            }} | Select-Object -First 1;
        }}
        
        if ($drivers) {{
            $driverName = $drivers.Name;
        }} else {{
        }}
    }} catch {{
        $errorMsg += ('查找品牌驱动错误: ' + $_.Exception.Message + '; ');
    }}
}}

# 步骤3: 如果没找到,安装inf驱动
Write-Output '[步骤3] 检查是否使用配置文件中的 INF 驱动';
if (-not $driverName -and $driverPathFromConfig -and $driverPathFromConfig -ne '') {{
    try {{
        $fullDriverPath = $driverPathFromConfig;
        Write-Output ('[步骤3-1] 配置文件驱动路径: ' + $fullDriverPath);
        
        if (Test-Path $fullDriverPath) {{
            $infPath = $fullDriverPath;
            Write-Output ('[步骤3-2] INF 文件存在，开始处理');
            
            # 先读取 INF 文件内容，提取驱动名称
            $infContent = Get-Content $infPath -Raw -ErrorAction SilentlyContinue;
            
            # 策略：先从 INF 文件中提取驱动名称，然后直接使用该名称查找已安装的驱动
            # 如果驱动未安装，再尝试安装
            
            # 首先尝试从 INF 内容提取驱动名称（使用更准确的匹配）
            $extractedDriverName = $null;
            
            # 尝试多个模式匹配驱动名称和 INF 节
            if ($infContent) {{
                # 优先：从 [Strings] 节提取驱动名称（最可靠的方法）
                # 格式：PRINTER1="驱动名称" 或 PRINTER1=驱动名称
                if ($infContent -match '(?s)\[Strings\]\s*(?:[^[\r\n]+|\r?\n)*PRINTER1\s*=\s*"([^"]+)"') {{
                    $extractedDriverName = $matches[1];
                }} elseif ($infContent -match '(?s)\[Strings\.\d+\]\s*(?:[^[\r\n]+|\r?\n)*PRINTER1\s*=\s*"([^"]+)"') {{
                    $extractedDriverName = $matches[1];
                }}
                
                # 如果还没找到，尝试匹配 INF 文件中的模型定义节（Models节通常包含驱动名称）
                if (-not $extractedDriverName -and $infContent -match '(?m)^\s*\[Models[^\]]*\]\s*\r?\n([^\[]+)') {{
                    $modelsSection = $matches[1];
                    if ($modelsSection -match '(?m)^\s*"([^"]+)"\s*=\s*"([^"]+)"') {{
                        $extractedDriverName = $matches[2];
                    }}
                }}
            }}
            
            # 如果没找到，尝试直接匹配常见的驱动名称模式
            if (-not $extractedDriverName -and $infContent) {{
                if ($infContent -match '(?m)^\s*DriverName\s*=\s*"([^"]+)"') {{
                    $extractedDriverName = $matches[1];
                }} elseif ($infContent -match '(?m)^\s*ClassDriver\s*=\s*"([^"]+)"') {{
                    $extractedDriverName = $matches[1];
                }} elseif ($infContent -match '(?m)^\s*Model\s*=\s*"([^"]+)"') {{
                    $extractedDriverName = $matches[1];
                }}
            }}
            
            # 如果提取到驱动名称，先检查驱动是否已安装
            if ($extractedDriverName) {{
                $existingDriver = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                    $_.Name -eq $extractedDriverName -or
                    $_.Name -like ('*' + $extractedDriverName + '*')
                }} | Select-Object -First 1;
                
                if ($existingDriver) {{
                    $driverName = $existingDriver.Name;
                    Write-Output ('从 INF 文件找到已安装的驱动: ' + $driverName);
                }} else {{
                    # 尝试使用 pnputil（需要管理员权限，可能会失败）
                    $pnputilResult = pnputil.exe /add-driver $infPath /install 2>&1;
                    $pnputilExitCode = $LASTEXITCODE;
                    
                    # 如果 pnputil 失败（通常因为权限问题），尝试使用 Add-PrinterDriver（需要驱动名称）
                    if ($pnputilExitCode -ne 0) {{
                        try {{
                            # 使用提取的驱动名称安装打印机驱动
                            Add-PrinterDriver -InfPath $infPath -Name $extractedDriverName -ErrorAction Stop;
                            $driverName = $extractedDriverName;
                            Write-Output ('从 INF 文件安装驱动成功: ' + $driverName);
                        }} catch {{
                            $errorMsg += ('从配置文件安装 INF 驱动失败: ' + $_.Exception.Message + '; ');
                            Write-Output ('INF 驱动安装失败: ' + $_.Exception.Message);
                        }}
                    }} else {{
                        # pnputil 成功，驱动包已添加到系统，但需要安装为打印机驱动
                        # 立即使用提取的驱动名称或 model 安装打印机驱动
                        $driverNameToInstall = if ($extractedDriverName) {{ $extractedDriverName }} elseif ($printerModel -and $printerModel -ne '') {{ $printerModel }} else {{ $null }};
                        
                        if ($driverNameToInstall) {{
                            try {{
                                Add-PrinterDriver -Name $driverNameToInstall -ErrorAction Stop;
                                $driverName = $driverNameToInstall;
                                Write-Output ('从 INF 文件安装驱动成功（pnputil + Add-PrinterDriver）: ' + $driverName);
                            }} catch {{
                                # 如果安装失败，尝试查找已安装的驱动
                                $installedDriver = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                                    $_.Name -eq $driverNameToInstall -or
                                    $_.Name -like ('*' + $driverNameToInstall + '*')
                                }} | Select-Object -First 1;
                                
                                if ($installedDriver) {{
                                    $driverName = $installedDriver.Name;
                                    Write-Output ('从 INF 文件找到已安装的驱动（pnputil后）: ' + $driverName);
                                }} else {{
                                    $errorMsg += ('从配置文件安装 INF 驱动失败: pnputil 成功但无法安装为打印机驱动; ');
                                }}
                            }}
                        }} else {{
                            # 如果没有驱动名称，尝试通过 INF 路径查找
                            $installedDriver = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                                $_.InfPath -like ('*' + [System.IO.Path]::GetFileNameWithoutExtension($infPath) + '*') -or
                                $_.InfPath -like ('*' + [System.IO.Path]::GetFileName($infPath) + '*')
                            }} | Select-Object -First 1;
                            
                            if ($installedDriver) {{
                                $driverName = $installedDriver.Name;
                                Write-Output ('从 INF 文件找到已安装的驱动（pnputil后）: ' + $driverName);
                            }} else {{
                                $errorMsg += ('从配置文件安装 INF 驱动失败: pnputil 成功但无法找到驱动名称; ');
                            }}
                        }}
                        
                    }}
                }}
            }} else {{
                # 如果无法提取驱动名称，尝试使用 pnputil（可能会失败，因为需要权限）
                $pnputilResult = pnputil.exe /add-driver $infPath /install 2>&1;
                $pnputilExitCode = $LASTEXITCODE;
                $pnputilOutput = $pnputilResult -join ' ';
                
                # 检查 pnputil 是否成功（即使退出代码不为 0，也可能已成功添加）
                $pnputilSuccess = ($pnputilExitCode -eq 0) -or ($pnputilOutput -match 'Driver package added successfully|Already exists in the system|Published Name');
                
                if ($pnputilSuccess) {{
                    # pnputil 成功后，使用配置文件中的 model 字段作为驱动名称安装驱动
                    if ($printerModel -and $printerModel -ne '') {{
                        try {{
                            Add-PrinterDriver -Name $printerModel -ErrorAction Stop;
                            $driverName = $printerModel;
                            Write-Output ('从 INF 文件安装驱动成功: ' + $driverName);
                        }} catch {{
                            # 如果使用 model 失败，尝试查找已安装的驱动
                            $installedDriver = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                                $_.InfPath -like ('*oem7.inf*')
                            }} | Select-Object -First 1;
                            
                            if (-not $installedDriver) {{
                                $installedDriver = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                                    $_.InfPath -like ('*' + [System.IO.Path]::GetFileNameWithoutExtension($infPath) + '*') -or
                                    $_.InfPath -like ('*RXE6E*') -or
                                    $_.InfPath -like ('*' + [System.IO.Path]::GetFileName($infPath) + '*')
                                }} | Select-Object -First 1;
                            }}
                            
                            if ($installedDriver) {{
                                $driverName = $installedDriver.Name;
                                Write-Output ('从 INF 文件找到已安装的驱动: ' + $driverName);
                            }} else {{
                                $errorMsg += ('从配置文件安装 INF 驱动失败: 无法找到已安装的驱动; ');
                            }}
                        }}
                    }} else {{
                        $errorMsg += ('从配置文件安装 INF 驱动失败: 配置文件中没有 model 字段; ');
                    }}
                }} else {{
                    $errorMsg += ('从配置文件安装 INF 驱动失败: 需要管理员权限或无法提取驱动名称; ');
                }}
            }}
            
            # 安装后查找驱动名称（如果还没有找到）
            Write-Output '[步骤3-10] 安装后查找驱动名称';
            if (-not $driverName) {{
                # 重新尝试从 INF 内容提取驱动名称（使用更完整的逻辑）
                Write-Output '[步骤3-11] 重新提取驱动名称';
                $extractedDriverName = $null;
                if ($infContent) {{
                    # 优先：从 [Strings] 节提取（最可靠）
                    if ($infContent -match '(?s)\[Strings\]\s*(?:[^[\r\n]+|\r?\n)*PRINTER1\s*=\s*"([^"]+)"') {{
                        $extractedDriverName = $matches[1];
                    }} elseif ($infContent -match '(?s)\[Strings\.\d+\]\s*(?:[^[\r\n]+|\r?\n)*PRINTER1\s*=\s*"([^"]+)"') {{
                        $extractedDriverName = $matches[1];
                    }}
                    
                    # 如果还没找到，尝试其他模式
                    if (-not $extractedDriverName) {{
                        if ($infContent -match '(?m)^\s*DriverName\s*=\s*"([^"]+)"') {{
                            $extractedDriverName = $matches[1];
                        }} elseif ($infContent -match '(?m)^\s*ClassDriver\s*=\s*"([^"]+)"') {{
                            $extractedDriverName = $matches[1];
                        }} elseif ($infContent -match '(?m)^\s*Model\s*=\s*"([^"]+)"') {{
                            $extractedDriverName = $matches[1];
                        }}
                    }}
                }}
                
                # 如果提取到驱动名称，尝试使用它安装或查找
                if ($extractedDriverName) {{
                    # 先查找是否已安装
                    $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                        $_.Name -eq $extractedDriverName -or
                        $_.Name -like ('*' + $extractedDriverName + '*')
                    }} | Select-Object -First 1;
                    
                    if ($drivers) {{
                        $driverName = $drivers.Name;
                        Write-Output ('从 INF 文件找到已安装的驱动: ' + $driverName);
                    }} else {{
                        # 尝试使用提取的名称安装
                        try {{
                            Add-PrinterDriver -Name $extractedDriverName -ErrorAction Stop;
                            $driverName = $extractedDriverName;
                            Write-Output ('使用提取的驱动名称安装成功: ' + $driverName);
                        }} catch {{
                            Write-Output ('使用提取的驱动名称安装失败: ' + $_.Exception.Message);
                        }}
                    }}
                }}
                
                # 如果还没找到，尝试通过 INF 文件名和路径查找
                if (-not $driverName) {{
                    $infBaseName = [System.IO.Path]::GetFileNameWithoutExtension($infPath);
                    $infFileName = [System.IO.Path]::GetFileName($infPath);
                    
                    # 查找最近安装的驱动（通过 INF 路径匹配）
                    $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                        $_.InfPath -like ('*' + $infBaseName + '*') -or
                        $_.Name -like ('*' + $infBaseName + '*') -or
                        $_.InfPath -like ('*' + $infFileName + '*')
                    }} | Sort-Object -Property InfPath -Descending | Select-Object -First 1;
                    
                    if ($drivers) {{
                        $driverName = $drivers.Name;
                        Write-Output ('通过 INF 路径找到驱动: ' + $driverName);
                    }}
                }}
            }}
        }} else {{
            Write-Output ('配置文件中的驱动路径不存在: ' + $fullDriverPath);
        }}
    }} catch {{
        $errorMsg += ('处理配置文件驱动路径错误: ' + $_.Exception.Message + '; ');
    }}
}}

# 优先级3: 如果没有找到品牌驱动，查找通用驱动（Generic/Text Only）
if (-not $driverName) {{
    try {{
        $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
            $_.Name -like '*Generic / Text Only*' -or
            $_.Name -eq 'Generic / Text Only'
        }} | Select-Object -First 1;
        
        if ($drivers) {{
            $driverName = $drivers.Name;
            Write-Output ('找到通用驱动: ' + $driverName);
        }}
    }} catch {{
        $errorMsg += ('查找通用驱动错误: ' + $_.Exception.Message + '; ');
    }}
}}

# 优先级4: 标准协议驱动（PostScript/PCL，排除品牌特定驱动和Universal Print）
if (-not $driverName) {{
    try {{
        $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
            (($_.Name -like '*Generic*' -and $_.Name -notlike '*HP*' -and $_.Name -notlike '*RICOH*' -and $_.Name -notlike '*CANON*' -and $_.Name -notlike '*EPSON*' -and $_.Name -notlike '*XEROX*') -and $_.Name -notlike '*Universal Print*') -or
            (($_.Name -like '*PostScript*' -and $_.Name -notlike '*HP*' -and $_.Name -notlike '*RICOH*' -and $_.Name -notlike '*CANON*') -and $_.Name -notlike '*Universal Print*') -or
            (($_.Name -like '*PCL*' -and $_.Name -notlike '*HP*' -and $_.Name -notlike '*RICOH*' -and $_.Name -notlike '*CANON*') -and $_.Name -notlike '*Universal Print*')
        }} | Select-Object -First 1;
        
        if ($drivers) {{
            $driverName = $drivers.Name;
            Write-Output ('找到标准协议驱动: ' + $driverName);
        }}
    }} catch {{
        $errorMsg += ('查找标准协议驱动错误: ' + $_.Exception.Message + '; ');
    }}
}}

# 优先级5: 如果检测到品牌，尝试品牌的通用驱动（Universal/Generic），但排除通用的 Universal Print Class Driver
Write-Output '[步骤6] 查找品牌通用驱动';
if (-not $driverName -and $printerBrand) {{
    try {{
        $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
            # 优先匹配品牌特定的 Universal 驱动（必须包含品牌名）
            ($_.Name -like ('*' + $printerBrand + '*Universal*') -and $_.Name -notlike '*Universal Print Class Driver*') -or
            ($_.Name -like ('*' + $printerBrand + '*Generic*')) -or
            ($printerBrand -eq 'RICOH' -and $_.Name -like '*RICOH*PostScript*') -or
            ($printerBrand -eq 'HP' -and $_.Name -like '*HP*PCL*')
        }} | Select-Object -First 1;
        
        if ($drivers) {{
            $driverName = $drivers.Name;
            Write-Output ('[步骤6] 找到品牌通用驱动: ' + $driverName);
        }} else {{
            Write-Output ('[步骤6] 未找到品牌通用驱动');
        }}
    }} catch {{
        $errorMsg += ('查找品牌通用驱动错误: ' + $_.Exception.Message + '; ');
        Write-Output ('[步骤6-错误] ' + $_.Exception.Message);
    }}
}}

# 最后备选：使用第一个可用驱动（警告可能不兼容）
if (-not $driverName) {{
    try {{
        $allDrivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Select-Object -First 1;
        if ($allDrivers) {{
            $driverName = $allDrivers.Name;
            Write-Output ('使用默认驱动: ' + $driverName + ' (警告: 可能不兼容)');
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
        # 验证参数不为空
        if (-not $printerName -or $printerName.Trim() -eq '') {{
            Write-Error '打印机名称不能为空';
            exit 1;
        }}
        if (-not $driverName -or $driverName.Trim() -eq '') {{
            Write-Error '驱动名称不能为空';
            exit 1;
        }}
        if (-not $port -or $port.Trim() -eq '') {{
            Write-Error '端口名称不能为空';
            exit 1;
        }}
        
        # 清理参数值
        $printerName = $printerName.Trim();
        $driverName = $driverName.Trim();
        $port = $port.Trim();
        
        # 检查端口是否存在，不存在则创建
        $portExists = Get-PrinterPort -Name $port -ErrorAction SilentlyContinue;
        if (-not $portExists) {{
            # 尝试从端口名称提取 IP 地址（格式：IP_192_168_x_x）
            $ipAddress = $port -replace '^IP_', '' -replace '_', '.';
            if ($ipAddress -match '^\d+\.\d+\.\d+\.\d+$') {{
                try {{
                    Add-PrinterPort -Name $port -PrinterHostAddress $ipAddress -ErrorAction Stop;
                }} catch {{
                    # 端口创建失败（可能已存在），继续执行
                }}
            }}
        }}
        
        # 验证驱动和端口是否存在
        $driverCheck = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -eq $driverName }};
        if (-not $driverCheck) {{
            Write-Error ('驱动不存在: "' + $driverName + '"');
            exit 1;
        }}
        
        $portCheck = Get-PrinterPort -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -eq $port }};
        if (-not $portCheck) {{
            Write-Error ('端口不存在: "' + $port + '"');
            exit 1;
        }}
        
        # 安装打印机（使用哈希表参数，避免特殊字符问题）
        $printerNameEscaped = $printerName -replace "'", "''";
        $driverNameEscaped = $driverName -replace "'", "''";
        $portEscaped = $port -replace "'", "''";
        
        # 使用哈希表参数传递，避免特殊字符问题
        $params = @{{
            Name = $printerNameEscaped;
            DriverName = $driverNameEscaped;
            PortName = $portEscaped;
        }};
        
        Add-Printer @params -ErrorAction Stop;
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
                port_name.replace("'", "''"),
                name.replace("'", "''"),
                {
                    let path_str = full_driver_path.as_ref().map(|p| p.to_string_lossy().replace("'", "''")).unwrap_or_default();
                    eprintln!("[DEBUG] ========== 传递给 PowerShell 的驱动路径 ==========");
                    eprintln!("[DEBUG] full_driver_path 是否为空: {}", full_driver_path.is_none());
                    if let Some(ref p) = full_driver_path {
                        eprintln!("[DEBUG] full_driver_path 路径: {:?}", p);
                        eprintln!("[DEBUG] full_driver_path 字符串: {}", p.to_string_lossy());
                    }
                    eprintln!("[DEBUG] 准备传递给 PowerShell 的路径字符串: \"{}\"", path_str);
                    eprintln!("[DEBUG] 路径字符串长度: {}", path_str.len());
                    path_str
                },
                model.as_ref().map(|m| m.replace("'", "''")).unwrap_or_default()
            );
            
            // 写入临时脚本文件，使用 UTF-8 BOM 编码（PowerShell 需要 BOM 来正确识别 UTF-8）
            let mut file = fs::File::create(&script_path)
                .map_err(|e| format!("创建临时 PowerShell 脚本失败: {}", e))?;
            
            // 写入 UTF-8 BOM (0xEF 0xBB 0xBF)
            file.write_all(&[0xEF, 0xBB, 0xBF])
                .map_err(|e| format!("写入 UTF-8 BOM 失败: {}", e))?;
            
            // 写入脚本内容（UTF-8 编码）
            file.write_all(ps_script.as_bytes())
                .map_err(|e| format!("写入脚本内容失败: {}", e))?;
            
            file.sync_all()
                .map_err(|e| format!("同步脚本文件失败: {}", e))?;
            
            drop(file); // 确保文件已关闭
            
            eprintln!("[DEBUG] 已创建临时 PowerShell 脚本: {:?}", script_path);
            
            // 使用 -File 参数执行脚本文件，避免命令行长度限制
            // 使用 spawn 和超时控制，避免命令挂起
            let child_handle = {
                let child = Command::new("powershell")
                    .arg("-NoProfile")
                    .arg("-WindowStyle")
                    .arg("Hidden")
                    .args([
                        "-ExecutionPolicy",
                        "Bypass",
                        "-File",
                        script_path.to_str().unwrap(),
                    ])
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()
                    .map_err(|e| format!("启动 PowerShell 脚本失败: {}", e))?;
                
                // 保存子进程 ID 以便在超时时杀死进程
                #[cfg(windows)]
                let child_pid = child.id();
                
                // 使用 tokio::time::timeout 添加超时控制
                let join_handle = tokio::task::spawn_blocking(move || {
                    child.wait_with_output()
                });
                
                let printer_output = tokio::time::timeout(
                    std::time::Duration::from_secs(POWERSHELL_TIMEOUT_SECS),
                    join_handle
                ).await;
                
                // 处理结果
                // printer_output 的类型是 Result<Result<Result<Output, io::Error>, JoinError>, TimeoutError>
                match printer_output {
                    Ok(join_result) => {
                        // join_result 的类型是 Result<Result<Output, io::Error>, JoinError>
                        match join_result {
                            Ok(output_result) => {
                                // output_result 的类型是 Result<Output, io::Error>
                                match output_result {
                                    Ok(output) => Ok(output),
                                    Err(e) => Err(format!("执行 PowerShell 脚本失败: {}", e)),
                                }
                            }
                            Err(e) => Err(format!("PowerShell 脚本任务失败: {}", e)),
                        }
                    }
                    Err(_) => {
                        // 超时，尝试杀死进程（通过进程ID）
                        #[cfg(windows)]
                        {
                            use std::process::Command;
                            let _ = Command::new("taskkill")
                                .args(["/F", "/PID", &child_pid.to_string()])
                                .creation_flags(CREATE_NO_WINDOW)
                                .output();
                        }
                        Err(format!("PowerShell 脚本执行超时（{}秒）。可能是：1. 驱动安装时间过长；2. 网络打印机无法连接；3. 系统权限不足。请检查网络连接和打印机状态。", POWERSHELL_TIMEOUT_SECS))
                    }
                }
            };
            
            // 执行完毕后删除临时脚本文件
            let _ = fs::remove_file(&script_path);
            
            let printer_result = match child_handle {
                Ok(output) => output,
                Err(e) => return Err(e),
            };
            
            let printer_stdout = decode_windows_string(&printer_result.stdout);
            let printer_stderr = decode_windows_string(&printer_result.stderr);
            
            if printer_result.status.success() || printer_stdout.contains("Success") {
                Ok(InstallResult {
                    success: true,
                    message: format!("打印机 {} ({}) 安装成功", name, ip_address),
                    method: Some("Add-Printer".to_string()),
                    stdout: Some(printer_stdout.clone()),
                    stderr: if printer_stderr.trim().is_empty() { None } else { Some(printer_stderr.clone()) },
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
                    stdout: if printer_stdout.trim().is_empty() { None } else { Some(printer_stdout.clone()) },
                    stderr: if printer_stderr.trim().is_empty() { None } else { Some(printer_stderr.clone()) },
                })
            }
        } else {
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
                .creation_flags(CREATE_NO_WINDOW)
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
                        // 计算完整的驱动路径（如果提供了配置的驱动路径）
                        let exe_dir_vbs = std::env::current_exe()
                            .ok()
                            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
                        
                        let full_driver_path_vbs = driver_path.as_ref().and_then(|dp| {
                            let full_path = exe_dir_vbs.join(dp);
                            if full_path.exists() {
                                eprintln!("[DEBUG] 找到配置的驱动路径 (VBS方式): {:?}", full_path);
                                Some(full_path)
                            } else {
                                eprintln!("[DEBUG] 警告: 配置的驱动路径不存在 (VBS方式): {:?}", full_path);
                                None
                            }
                        });
                        
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
$printerName = '{}';
$printerNameUpper = $printerName.ToUpper();
$driverPathFromConfig = '{}';
$printerModel = '{}';
$printerModelUpper = if ($printerModel -and $printerModel -ne '') {{ $printerModel.ToUpper() }} else {{ '' }};

# 检测打印机品牌（从名称和型号中推断）
$printerBrand = $null;
# 首先从型号（model）中检测，型号通常包含更准确的品牌信息
 品牌检测: 打印机名称=' + $printerName + ', 型号=' + $printerModel);
 品牌检测: 型号是否为空: ' + ($printerModel -eq '' -or -not $printerModel));
if ($printerModelUpper -match 'RICOH|理光') {{
    $printerBrand = 'RICOH';
 品牌检测: 从型号中检测到品牌: RICOH');
}} elseif ($printerModelUpper -match 'HP|HEWLETT|惠普') {{
    $printerBrand = 'HP';
 品牌检测: 从型号中检测到品牌: HP');
}} elseif ($printerModelUpper -match 'CANON|佳能') {{
    $printerBrand = 'CANON';
 品牌检测: 从型号中检测到品牌: CANON');
}} elseif ($printerModelUpper -match 'EPSON|爱普生') {{
    $printerBrand = 'EPSON';
 品牌检测: 从型号中检测到品牌: EPSON');
}} elseif ($printerModelUpper -match 'XEROX|施乐') {{
    $printerBrand = 'XEROX';
 品牌检测: 从型号中检测到品牌: XEROX');
}} elseif ($printerModelUpper -match 'KYOCERA|京瓷') {{
    $printerBrand = 'KYOCERA';
 品牌检测: 从型号中检测到品牌: KYOCERA');
}} elseif ($printerModelUpper -match 'BROTHER|兄弟') {{
    $printerBrand = 'BROTHER';
 品牌检测: 从型号中检测到品牌: BROTHER');
}}

# 如果型号中没有找到品牌，尝试从名称中检测
if (-not $printerBrand) {{
    if ($printerNameUpper -match 'RICOH|理光') {{
        $printerBrand = 'RICOH';
 品牌检测: 从名称中检测到品牌: RICOH');
    }} elseif ($printerNameUpper -match 'HP|HEWLETT|惠普') {{
        $printerBrand = 'HP';
 品牌检测: 从名称中检测到品牌: HP');
    }} elseif ($printerNameUpper -match 'CANON|佳能') {{
        $printerBrand = 'CANON';
 品牌检测: 从名称中检测到品牌: CANON');
    }} elseif ($printerNameUpper -match 'EPSON|爱普生') {{
        $printerBrand = 'EPSON';
 品牌检测: 从名称中检测到品牌: EPSON');
    }} elseif ($printerNameUpper -match 'XEROX|施乐') {{
        $printerBrand = 'XEROX';
 品牌检测: 从名称中检测到品牌: XEROX');
    }} elseif ($printerNameUpper -match 'KYOCERA|京瓷') {{
        $printerBrand = 'KYOCERA';
 品牌检测: 从名称中检测到品牌: KYOCERA');
    }} elseif ($printerNameUpper -match 'BROTHER|兄弟') {{
        $printerBrand = 'BROTHER';
 品牌检测: 从名称中检测到品牌: BROTHER');
    }}
}}

if (-not $printerBrand) {{
 品牌检测: 未从型号或名称中检测到品牌');
}}

# 优先级1: 如果检测到品牌，优先查找对应品牌的驱动
if ($printerBrand) {{
    try {{
        $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
            $_.Name -like ('*' + $printerBrand + '*')
        }} | Select-Object -First 1;
        
        if ($drivers) {{
            $driverName = $drivers.Name;
            Write-Output ('找到品牌驱动 [' + $printerBrand + ']: ' + $driverName);
        }}
    }} catch {{
        Write-Error ('查找品牌驱动错误: ' + $_.Exception.Message);
    }}
}}

# 优先级1.5: 如果找不到品牌驱动，尝试从配置文件安装 INF 驱动文件
if (-not $driverName) {{
    # $driverPathFromConfig 已经是完整路径（在 Rust 中已计算）
    if ($driverPathFromConfig -and $driverPathFromConfig -ne '') {{
        try {{
            $fullDriverPath = $driverPathFromConfig;
            
            if (Test-Path $fullDriverPath) {{
                $infPath = $fullDriverPath;
                
                # 尝试使用 Add-PrinterDriver 安装驱动
                try {{
                    Add-PrinterDriver -InfPath $infPath -ErrorAction Stop;
                    
                    # 安装后查找驱动名称
                    $infBaseName = [System.IO.Path]::GetFileNameWithoutExtension($infPath);
                    $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                        $_.InfPath -like ('*' + $infBaseName + '*') -or
                        $_.Name -like ('*' + $infBaseName + '*') -or
                        $_.InfPath -like ('*' + [System.IO.Path]::GetFileName($infPath) + '*')
                    }} | Select-Object -First 1;
                    
                        if ($drivers) {{
                            $driverName = $drivers.Name;
                            Write-Output ('从 INF 文件找到驱动: ' + $driverName);
                        }} else {{
                        # 如果无法通过文件名找到，尝试从 INF 内容提取
                        $infContent = Get-Content $infPath -Raw -ErrorAction SilentlyContinue;
                        if ($infContent) {{
                            # 尝试多个模式匹配驱动名称
                            $extractedDriverName = $null;
                            if ($infContent -match '(?m)^\s*DriverName\s*=\s*"([^"]+)"') {{
                                $extractedDriverName = $matches[1];
                            }} elseif ($infContent -match '(?m)^\s*ClassDriver\s*=\s*"([^"]+)"') {{
                                $extractedDriverName = $matches[1];
                            }} elseif ($infContent -match '(?m)^\s*Model\s*=\s*"([^"]+)"') {{
                                $extractedDriverName = $matches[1];
                            }}
                            
                            if ($extractedDriverName) {{
                                # 尝试精确匹配
                                $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
                                    $_.Name -eq $extractedDriverName -or
                                    $_.Name -like ('*' + $extractedDriverName + '*')
                                }} | Select-Object -First 1;
                                
                                if ($drivers) {{
                                    $driverName = $drivers.Name;
                                    Write-Output ('从 INF 内容提取找到驱动: ' + $driverName);
                                }}
                            }}
                        }}
                    }}
                }} catch {{
                    Write-Error ('从配置文件安装 INF 驱动失败: ' + $_.Exception.Message);
                }}
            }} else {{
                Write-Output ('配置文件中的驱动路径不存在: ' + $fullDriverPath);
            }}
        }} catch {{
            Write-Error ('处理配置文件驱动路径错误: ' + $_.Exception.Message);
        }}
    }}
}}

# 优先级2: 如果没有找到品牌驱动，查找通用驱动（Generic/Text Only）
if (-not $driverName) {{
    try {{
        $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
            $_.Name -like '*Generic / Text Only*' -or
            $_.Name -eq 'Generic / Text Only'
        }} | Select-Object -First 1;
        
        if ($drivers) {{
            $driverName = $drivers.Name;
            Write-Output ('找到通用驱动: ' + $driverName);
        }}
    }} catch {{
        Write-Error ('查找通用驱动错误: ' + $_.Exception.Message);
    }}
}}

# 优先级3: 标准协议驱动（PostScript/PCL，排除品牌特定驱动）
if (-not $driverName) {{
    try {{
        $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
            ($_.Name -like '*Generic*' -and $_.Name -notlike '*HP*' -and $_.Name -notlike '*RICOH*' -and $_.Name -notlike '*CANON*' -and $_.Name -notlike '*EPSON*' -and $_.Name -notlike '*XEROX*') -or
            ($_.Name -like '*PostScript*' -and $_.Name -notlike '*HP*' -and $_.Name -notlike '*RICOH*' -and $_.Name -notlike '*CANON*') -or
            ($_.Name -like '*PCL*' -and $_.Name -notlike '*HP*' -and $_.Name -notlike '*RICOH*' -and $_.Name -notlike '*CANON*')
        }} | Select-Object -First 1;
        
        if ($drivers) {{
            $driverName = $drivers.Name;
            Write-Output ('找到标准协议驱动: ' + $driverName);
        }}
    }} catch {{
        Write-Error ('查找标准协议驱动错误: ' + $_.Exception.Message);
    }}
}}

# 优先级4: 如果检测到品牌，尝试品牌的通用驱动（Universal/Generic）
if (-not $driverName -and $printerBrand) {{
    try {{
        $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{
            ($_.Name -like ('*' + $printerBrand + '*Universal*')) -or
            ($_.Name -like ('*' + $printerBrand + '*Generic*')) -or
            ($printerBrand -eq 'RICOH' -and $_.Name -like '*RICOH*PostScript*') -or
            ($printerBrand -eq 'HP' -and $_.Name -like '*HP*PCL*')
        }} | Select-Object -First 1;
        
        if ($drivers) {{
            $driverName = $drivers.Name;
            Write-Output ('找到品牌通用驱动: ' + $driverName);
        }}
    }} catch {{
        Write-Error ('查找品牌通用驱动错误: ' + $_.Exception.Message);
    }}
}}

# 最后备选：使用第一个可用驱动（警告可能不兼容）
if (-not $driverName) {{
    try {{
        $allDrivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Select-Object -First 1;
        if ($allDrivers) {{
            $driverName = $allDrivers.Name;
            Write-Output ('使用默认驱动: ' + $driverName + ' (警告: 可能不兼容)');
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
                                    name.replace("'", "''"),
                                    full_driver_path_vbs.as_ref().map(|p| p.to_string_lossy().replace("'", "''")).unwrap_or_default(),
                                    name.replace("'", "''"),
                                    model.as_ref().map(|m| m.replace("'", "''")).unwrap_or_default()
                                )
                            ])
                            .stdin(Stdio::null())
                            .stdout(Stdio::piped())
                            .stderr(Stdio::piped())
                            .creation_flags(CREATE_NO_WINDOW)
                            .output();
                        
                        match ps_output {
                            Ok(ps_result) => {
                                let ps_stdout = decode_windows_string(&ps_result.stdout);
                                let ps_stderr = decode_windows_string(&ps_result.stderr);
                                
                                eprintln!("[DEBUG] ========== VBS方式：Add-Printer PowerShell 输出 ==========");
                                eprintln!("[DEBUG] VBS Add-Printer stdout (完整输出):\n{}", ps_stdout);
                                eprintln!("[DEBUG] VBS Add-Printer stderr (错误输出):\n{}", ps_stderr);
                                eprintln!("[DEBUG] VBS Add-Printer exit code: {:?}", ps_result.status.code());
                                
                                // 提取DEBUG日志中的关键信息
                                if ps_stdout.contains("[DEBUG]") {
                                    eprintln!("[DEBUG] VBS方式：从PowerShell输出中提取的关键DEBUG信息:");
                                    for line in ps_stdout.lines() {
                                        if line.contains("[DEBUG]") {
                                            eprintln!("  {}", line);
                                        }
                                    }
                                }
                                
                                let ps_stdout_decoded = decode_windows_string(&ps_result.stdout);
                                let ps_stderr_decoded = decode_windows_string(&ps_result.stderr);
                                
                                if ps_result.status.success() || ps_stdout_decoded.contains("Success") {
                                    Ok(InstallResult {
                                        success: true,
                                        message: format!("打印机 {} ({}) 安装成功", name, ip_address),
                                        method: Some("VBS".to_string()),
                                        stdout: Some(ps_stdout_decoded.clone()),
                                        stderr: if ps_stderr_decoded.trim().is_empty() { None } else { Some(ps_stderr_decoded.clone()) },
                                    })
                                } else {
                                    // 组合详细的错误信息
                                    let mut error_details = String::new();
                                    if !ps_stdout_decoded.trim().is_empty() {
                                        error_details.push_str(&format!("标准输出: {}; ", ps_stdout_decoded.trim()));
                                    }
                                    if !ps_stderr_decoded.trim().is_empty() {
                                        error_details.push_str(&format!("错误输出: {}", ps_stderr_decoded.trim()));
                                    }
                                    if error_details.is_empty() {
                                        error_details = format!("退出代码: {:?}", ps_result.status.code().unwrap_or(-1));
                                    }
                                    
                                    Ok(InstallResult {
                                        success: false,
                                        message: format!("端口添加成功，但打印机安装失败。{}请确保系统中已安装打印机驱动，或联系管理员安装驱动。", error_details),
                                        method: Some("VBS".to_string()),
                                        stdout: if ps_stdout_decoded.trim().is_empty() { None } else { Some(ps_stdout_decoded.clone()) },
                                        stderr: if ps_stderr_decoded.trim().is_empty() { None } else { Some(ps_stderr_decoded.clone()) },
                                    })
                                }
                            }
                            Err(e) => {
                                Ok(InstallResult {
                                    success: false,
                                    message: format!("端口添加成功，但执行 PowerShell 命令失败: {}。请确保系统中已安装打印机驱动，或联系管理员安装驱动。", e),
                                    method: Some("VBS".to_string()),
                                    stdout: None,
                                    stderr: Some(format!("PowerShell 命令执行失败: {}", e)),
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
                            stdout: if stdout.trim().is_empty() { None } else { Some(stdout.clone()) },
                            stderr: if stderr.trim().is_empty() { None } else { Some(stderr.clone()) },
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
                        stdout: None,
                        stderr: Some(format!("执行 prnport.vbs 脚本失败: {} | {}", e, script_info)),
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
        
        // 在调用系统命令前对 printer name 做额外校验：
        // lpadmin 常见报错为 "printer name can only contain printable characters"，
        // 因此拒绝包含控制字符的名称并返回友好错误给前端。
        let name_trim = name.trim();
        if name_trim.is_empty() {
            return Ok(InstallResult {
                success: false,
                message: "安装打印机失败: 打印机名称不能为空。 [方式: macOS]".to_string(),
                method: Some("macOS".to_string()),
                stdout: None,
                stderr: None,
            });
        }
        if name_trim.chars().any(|c| c.is_control()) {
            return Ok(InstallResult {
                success: false,
                message: "安装打印机失败: lpadmin: 打印机名称只能包含可打印字符。请确保已授予管理员权限，或联系管理员。 [方式: macOS]".to_string(),
                method: Some("macOS".to_string()),
                stdout: None,
                stderr: None,
            });
        }

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
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(InstallResult {
                success: true,
                message: format!("打印机 {} ({}) 安装成功", name, ip_address),
                method: Some("macOS".to_string()),
                stdout: if stdout.trim().is_empty() { None } else { Some(stdout.to_string()) },
                stderr: None,
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
                stdout: if stdout.trim().is_empty() { None } else { Some(stdout.to_string()) },
                stderr: if stderr.trim().is_empty() { None } else { Some(stderr.to_string()) },
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

// 打印测试页
#[tauri::command]
fn print_test_page(printer_name: String) -> Result<String, String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        use std::process::Stdio;
        use std::fs;
        use std::io::Write;
        
        // 先验证打印机是否存在
        let check_output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-WindowStyle", "Hidden",
                "-Command",
                &format!("Get-Printer -Name '{}' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name", 
                    printer_name.replace("'", "''"))
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        
        let printer_exists = match check_output {
            Ok(output) => {
                let stdout = decode_windows_string(&output.stdout);
                output.status.success() && !stdout.trim().is_empty()
            }
            Err(_) => false
        };
        
        if !printer_exists {
            return Err(format!("打印机不存在或未连接: {}", printer_name));
        }
        
        // 生成测试页内容
        let now = chrono::Local::now();
        let test_content = format!(
r#"
====================================================
    易点云打印机安装小精灵 测试页 
====================================================

别担心，我不是广告，我只是来测试你的打印机 


Hello 小伙伴！

恭喜你，你的打印机已经安装成功啦！


1 文字测试：

我是一行可爱的测试文字。

我也是一行幽默的测试文字。

请放心打印，我不会跑掉的。


2 ASCII 艺术测试：

     (\\_/)
     ( •_•)
     />  内网打印机助手向你问好！


3 信息测试：

打印机名称：{printer_name}

测试打印时间：{test_time}


4 字符集测试：

中文：你好世界！

English: Hello World!

数字：0123456789

特殊字符：!@#$%^&*()_+-=[]{{}}|;:',.<>?


====================================================
 小提示：

如果你能看到这个页面，说明打印机工作正常 

打印机越开心，工作效率越高哦!

— 易点云打印机安装小精灵，专注打印安全与快乐 —

来自开发者:易点云研发中心核心业务部 比心
====================================================
"#,
            printer_name = printer_name,
            test_time = now.format("%Y年%m月%d日 %H:%M:%S")
        );
        
        // 创建临时文件
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join(format!("printer_test_{}.txt", std::process::id()));
        
        // 写入测试内容（使用 UTF-8 BOM）
        {
            let mut file = fs::File::create(&temp_file)
                .map_err(|e| format!("创建临时文件失败: {}", e))?;
            
            // 写入 UTF-8 BOM
            file.write_all(&[0xEF, 0xBB, 0xBF])
                .map_err(|e| format!("写入 UTF-8 BOM 失败: {}", e))?;
            
            // 写入测试内容
            file.write_all(test_content.as_bytes())
                .map_err(|e| format!("写入测试内容失败: {}", e))?;
            
            file.sync_all()
                .map_err(|e| format!("同步文件失败: {}", e))?;
        }
        
        // 使用 PowerShell 打印文件到指定打印机
        let ps_command = format!(
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $content = Get-Content '{}' -Encoding UTF8 -Raw; $content | Out-Printer -Name '{}'; if ($LASTEXITCODE -eq 0 -or $?) {{ Write-Output 'Success' }} else {{ Write-Error '打印失败' }}",
            temp_file.to_str().unwrap().replace("'", "''").replace("\\", "\\\\"),
            printer_name.replace("'", "''").replace("\\", "\\\\")
        );
        
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-WindowStyle", "Hidden",
                "-Command",
                &ps_command
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .creation_flags(CREATE_NO_WINDOW)
            .output();
        
        // 清理临时文件
        let _ = fs::remove_file(&temp_file);
        
        let output = output.map_err(|e| format!("执行打印测试页命令失败: {}", e))?;
        let stdout = decode_windows_string(&output.stdout);
        let stderr = decode_windows_string(&output.stderr);
        
        if output.status.success() || stdout.contains("Success") {
            Ok(format!("测试页已发送到打印机: {}", printer_name))
        } else {
            // 如果 Out-Printer 失败，尝试使用 CIM 方法作为备选
            let ps_command2 = format!(
                "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $printer = Get-CimInstance -ClassName Win32_Printer -Filter \"Name='{}'\" -ErrorAction SilentlyContinue; if ($printer) {{ try {{ Invoke-CimMethod -InputObject $printer -MethodName PrintTestPage -ErrorAction Stop; Write-Output 'Success' }} catch {{ Write-Error $_.Exception.Message }} }} else {{ Write-Error '打印机不存在或未连接' }} }}",
                printer_name.replace("'", "''").replace("\\", "\\\\")
            );
            
            let output2 = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-WindowStyle", "Hidden",
                    "-Command",
                    &ps_command2
                ])
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .creation_flags(CREATE_NO_WINDOW)
                .output();
            
            match output2 {
                Ok(output) if output.status.success() || decode_windows_string(&output.stdout).contains("Success") => {
                    Ok(format!("测试页已发送到打印机: {}", printer_name))
                }
                Ok(output) => {
                    let stderr2 = decode_windows_string(&output.stderr);
                    Err(format!("打印测试页失败: {}. 请检查打印机是否在线且名称是否正确。", 
                        if !stderr2.trim().is_empty() { 
                            stderr2.trim() 
                        } else if !stderr.trim().is_empty() { 
                            stderr.trim() 
                        } else { 
                            "未知错误" 
                        }))
                }
                Err(e) => {
                    Err(format!("打印测试页失败: {}. 请确保打印机已连接并可以访问。", e))
                }
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        use std::process::Stdio;
        
        // macOS 使用 lp 命令打印测试页
        // 创建一个简单的测试页内容
        let test_content = format!("打印机测试页\n打印机名称: {}\n\n这是一个测试页面，用于验证打印机是否正常工作。\n", 
            printer_name
        );
        
        let mut output = Command::new("lp")
            .arg("-d")
            .arg(&printer_name)
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("执行 lp 命令失败: {}", e))?;
        
        if let Some(mut stdin) = output.stdin.take() {
            use std::io::Write;
            stdin.write_all(test_content.as_bytes())
                .map_err(|e| format!("写入测试内容失败: {}", e))?;
            stdin.flush().map_err(|e| format!("刷新输入流失败: {}", e))?;
        }
        
        let result = output.wait_with_output()
            .map_err(|e| format!("等待打印命令完成失败: {}", e))?;
        
        if result.status.success() {
            Ok(format!("测试页已发送到打印机: {}", printer_name))
        } else {
            let stderr = String::from_utf8_lossy(&result.stderr);
            Err(format!("打印测试页失败: {}", stderr))
        }
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("当前平台不支持打印测试页功能".to_string())
    }
}

// 检查软件版本更新
#[tauri::command]
async fn check_version_update() -> Result<VersionCheckResult, String> {
    // 获取当前版本
    let current_version = env!("CARGO_PKG_VERSION");
    
    // 加载远程版本配置
    let version_config_url = VERSION_CONFIG_REMOTE_URL;
    
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_SECS))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
    
    let response = client
        .get(version_config_url)
        .send()
        .await
        .map_err(|e| {
            let error_msg = format!("网络请求失败: {}", e);
            if e.is_timeout() {
                format!("{} (请求超时)", error_msg)
            } else {
                error_msg
            }
        })?;
    
    if !response.status().is_success() {
        return Err(format!("服务器返回错误: {}", response.status()));
    }
    
    let version_config: VersionConfig = response
        .json()
        .await
        .map_err(|e| format!("解析版本配置失败: {}", e))?;
    
    // 比较版本
    let has_update = compare_versions(current_version, &version_config.app_version);
    
    Ok(VersionCheckResult {
        has_update,
        current_version: current_version.to_string(),
        latest_version: version_config.app_version.clone(),
        update_url: version_config.update_url.clone(),
        update_type: version_config.update_type.clone(),
        update_description: version_config.update_description.clone(),
        force_update: version_config.force_update,
        changelog: version_config.changelog.clone(),
        download_size: version_config.download_size.clone(),
    })
}

// 比较版本号（简单版本，支持语义化版本）
fn compare_versions(current: &str, latest: &str) -> bool {
    // 简单的版本号比较，支持 x.y.z 格式
    let current_parts: Vec<u32> = current
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    let latest_parts: Vec<u32> = latest
        .split('.')
        .filter_map(|s| s.parse().ok())
        .collect();
    
    // 补齐长度
    let max_len = current_parts.len().max(latest_parts.len());
    let mut current_parts = current_parts;
    let mut latest_parts = latest_parts;
    
    while current_parts.len() < max_len {
        current_parts.push(0);
    }
    while latest_parts.len() < max_len {
        latest_parts.push(0);
    }
    
    // 逐位比较
    for (c, l) in current_parts.iter().zip(latest_parts.iter()) {
        if l > c {
            return true;
        } else if l < c {
            return false;
        }
    }
    
    false
}

// 下载并更新软件
#[tauri::command]
async fn download_update(update_url: String) -> Result<String, String> {
    use std::fs;
    use std::io::Write;
    
    // 创建 HTTP 客户端
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(HTTP_TIMEOUT_DOWNLOAD_SECS))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
    
    // 下载更新文件
    let response = client
        .get(&update_url)
        .send()
        .await
        .map_err(|e| format!("下载更新文件失败: {}", e))?;
    
    if !response.status().is_success() {
        return Err(format!("下载失败: {}", response.status()));
    }
    
    // 获取文件内容
    let bytes = response.bytes().await
        .map_err(|e| format!("读取更新文件失败: {}", e))?;
    
    // 保存到临时目录
    let temp_dir = std::env::temp_dir();
    let _exe_name = std::path::Path::new(&update_url)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("update.exe");
    let temp_file = temp_dir.join(format!("easyPrinter_update_{}.exe", std::process::id()));
    
    {
        let mut file = fs::File::create(&temp_file)
            .map_err(|e| format!("创建临时文件失败: {}", e))?;
        
        file.write_all(&bytes)
            .map_err(|e| format!("写入更新文件失败: {}", e))?;
        
        file.sync_all()
            .map_err(|e| format!("同步文件失败: {}", e))?;
    }
    
    Ok(format!("更新文件已下载到: {}", temp_file.to_string_lossy()))
}

#[cfg(windows)]
fn is_elevated() -> bool {
    use winapi::um::winnt::TOKEN_ELEVATION;
    use winapi::um::winnt::HANDLE;
    use winapi::um::processthreadsapi::GetCurrentProcess;
    use winapi::um::processthreadsapi::OpenProcessToken;
    use winapi::um::winnt::TOKEN_QUERY;
    use winapi::um::handleapi::CloseHandle;
    use std::mem;
    use std::ptr;
    
    unsafe {
        let mut token: HANDLE = ptr::null_mut();
        let process = GetCurrentProcess();
        
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 {
            return false;
        }
        
        let mut elevation: TOKEN_ELEVATION = mem::zeroed();
        let mut ret_size: u32 = 0;
        
        use winapi::um::securitybaseapi::GetTokenInformation;
        use winapi::um::winnt::TokenElevation;
        
        let result = GetTokenInformation(
            token,
            TokenElevation,
            &mut elevation as *mut _ as *mut _,
            mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut ret_size,
        );
        
        CloseHandle(token);
        
        if result != 0 {
            elevation.TokenIsElevated != 0
        } else {
            false
        }
    }
}

#[cfg(not(windows))]
fn is_elevated() -> bool {
    false // 非 Windows 平台不需要权限提升
}

#[cfg(windows)]
fn check_webview2_installed() -> bool {
    use std::process::Command;
    use std::process::Stdio;
    
    // 方法1: 检查注册表中的 WebView2 运行时（64位）
    match Command::new("reg")
        .arg("query")
        .arg("HKLM\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}")
        .arg("/v")
        .arg("pv")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                // 检查版本号（WebView2 需要 90.0+）
                if let Ok(output_str) = String::from_utf8(output.stdout) {
                    if let Some(version_line) = output_str.lines().find(|l| l.contains("pv")) {
                        if let Some(version_str) = version_line.split_whitespace().last() {
                            // 提取主版本号（例如 "92.0.902.67" -> 92）
                            if let Some(major_version) = version_str.split('.').next() {
                                if let Ok(major) = major_version.parse::<u32>() {
                                    if major >= 90 {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                } else {
                    // 即使无法解析版本，如果注册表存在也认为已安装
                    return true;
                }
            }
        }
        Err(_) => {}
    }
    
    // 方法1.5: 检查注册表中的 WebView2 运行时（32位）
    match Command::new("reg")
        .arg("query")
        .arg("HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}")
        .arg("/v")
        .arg("pv")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                return true;
            }
        }
        Err(_) => {}
    }
    
    // 方法2: 检查 WebView2 DLL 文件是否存在（最可靠的方法）
    let webview2_dll_paths = vec![
        "C:\\Program Files (x86)\\Microsoft\\EdgeWebView\\Application",
        "C:\\Program Files\\Microsoft\\EdgeWebView\\Application",
        // 检查每个版本目录下的 WebView2Loader.dll
    ];
    
    for base_path in &webview2_dll_paths {
        if let Ok(entries) = std::fs::read_dir(base_path) {
            for entry in entries.flatten() {
                let version_dir = entry.path();
                if version_dir.is_dir() {
                    let loader_dll = version_dir.join("WebView2Loader.dll");
                    if loader_dll.exists() {
                        return true;
                    }
                }
            }
        }
    }
    
    // 方法3: 检查 EdgeWebView2Loader.dll 在系统目录中
    let system_loader_paths = vec![
        "C:\\Windows\\System32\\EdgeWebView2Loader.dll",
        "C:\\Windows\\SysWOW64\\EdgeWebView2Loader.dll",
    ];
    
    for path in &system_loader_paths {
        if std::path::Path::new(path).exists() {
            return true;
        }
    }
    
    // 方法4: 检查 Edge 浏览器版本（Edge 90+ 应该包含 WebView2，但需要独立的运行时）
    match Command::new("reg")
        .arg("query")
        .arg("HKLM\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{56EB18F8-B008-4CBD-B6D2-8C97FE7E9382}")
        .arg("/v")
        .arg("pv")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .output()
    {
        Ok(output) => {
            if output.status.success() {
                // Edge 存在，但可能版本太旧或需要独立的 WebView2 运行时
                // 这里返回 false，因为即使安装了 Edge，也可能需要独立的 WebView2 运行时
                // 让 Tauri 尝试加载，如果失败会给出更详细的错误信息
                return false; // 不依赖 Edge 的存在来判断 WebView2
            }
        }
        Err(_) => {}
    }
    
    false
}

#[cfg(not(windows))]
fn check_webview2_installed() -> bool {
    true // 非 Windows 平台不需要检查
}

fn main() {
    // 在 Windows 上检查是否有管理员权限
    #[cfg(windows)]
    {
        if !is_elevated() {
            eprintln!("[警告] 应用未以管理员权限运行");
            eprintln!("[警告] 安装打印机驱动可能需要管理员权限");
            eprintln!("[警告] 如果遇到权限错误，请以管理员身份运行应用");
            // 注意：如果使用了 manifest，应用启动时会自动弹出 UAC 提示
            // 这里只是记录警告，不会阻止应用运行
        } else {
            eprintln!("[信息] 应用以管理员权限运行");
        }
        
        // 检查 WebView2 是否安装（仅记录日志，不阻止启动）
        // 让 Tauri 自己尝试加载 WebView2，如果失败会给出更准确的错误信息
        if !check_webview2_installed() {
            eprintln!("[警告] 未检测到 WebView2 运行时，但将继续尝试启动");
            eprintln!("[警告] 如果启动失败，请安装 WebView2 运行时: {}", WEBVIEW2_DOWNLOAD_URL);
            eprintln!("[警告] 即使已安装 Edge 浏览器，也可能需要单独安装 WebView2 运行时");
        } else {
            eprintln!("[信息] 检测到 WebView2 运行时");
        }
    }
    
    // 改进错误处理，避免程序静默退出
    let result = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_config,
            list_printers,
            install_printer,
            open_url,
            confirm_update_config,
            print_test_page,
            check_version_update,
            download_update
        ])
        .run(tauri::generate_context!());
    
    // 如果启动失败，显示错误信息
    if let Err(e) = result {
        let error_str = e.to_string();
        let mut error_msg = String::new();
        
          // 如果是 WebView2 错误，提供更具体的帮助信息
          // NOTE: ERROR_FILE_NOT_FOUND 常量仅在 windows cfg 下定义，避免非 Windows 平台引用未定义符号
          #[cfg(windows)]
          let is_webview2_error = error_str.contains("WebView2")
              || error_str.contains(ERROR_FILE_NOT_FOUND)
              || error_str.contains("failed to create webview")
              || error_str.contains("cannot find the file specified");

          #[cfg(not(windows))]
          let is_webview2_error = error_str.contains("WebView2")
              || error_str.contains("failed to create webview")
              || error_str.contains("cannot find the file specified");

          if is_webview2_error {
            error_msg = format!(
                "WebView2 运行时未安装或版本不兼容\n\n\
                此应用需要 Microsoft Edge WebView2 运行时（版本 90.0 或更高）才能运行。\n\n\
                常见情况：\n\
                • 已安装 Edge 浏览器但缺少独立的 WebView2 运行时\n\
                • Edge 版本过旧（需要 Edge 90+ 且安装独立的 WebView2 运行时）\n\
                • WebView2 运行时损坏或未正确安装\n\n\
                解决方案：\n\
                1. 【推荐】下载并安装独立的 WebView2 运行时：\n\
                   {}\n\
                2. 更新 Microsoft Edge 浏览器到最新版本\n\
                3. 安装完成后，请重启计算机，然后重新运行此应用\n\n\
                错误代码：{}\n\
                错误详情：{}\n\n\
                提示：即使已安装 Edge 浏览器，也可能需要单独安装 WebView2 运行时。", 
                WEBVIEW2_DOWNLOAD_URL, error_str, e
            );
    } else {
            error_msg = format!(
                "应用启动失败\n\n\
                错误信息：{}\n\n\
                请检查：\n\
                1. 是否缺少必要的依赖文件\n\
                2. 配置文件是否存在\n\
                3. 网络连接是否正常\n\
                4. 是否有足够的权限\n\n\
                错误详情：{}", 
                error_str, e
            );
        }
        
        // 在 Windows 上使用消息框显示错误
        #[cfg(windows)]
        {
            use winapi::um::winuser::{MessageBoxW, MB_OK, MB_ICONERROR};
            use std::ffi::OsStr;
            use std::os::windows::ffi::OsStrExt;
            
            let title: Vec<u16> = OsStr::new("易点云打印机安装小精灵 - 启动错误")
                .encode_wide()
                .chain(Some(0))
                .collect();
            
            let message: Vec<u16> = OsStr::new(&error_msg)
                .encode_wide()
                .chain(Some(0))
                .collect();
            
            unsafe {
                MessageBoxW(
                    std::ptr::null_mut(),
                    message.as_ptr(),
                    title.as_ptr(),
                    MB_OK | MB_ICONERROR,
                );
            }
        }
        
        // 非 Windows 平台或备用方式：输出到 stderr
        #[cfg(not(windows))]
        {
            eprintln!("{}", error_msg);
        }
        
        // 尝试写入错误日志文件（如果可能）
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let log_path = exe_dir.join("error.log");
                if let Ok(mut file) = std::fs::File::create(&log_path) {
                    use std::io::Write;
                    let _ = writeln!(file, "{}", error_msg);
                }
            }
        }
        
        // 退出程序
        std::process::exit(1);
    }
}

