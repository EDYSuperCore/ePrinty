// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(windows)]
use std::os::windows::process::CommandExt;

use serde::{Deserialize, Serialize};
use std::fs;
use tauri::Manager;

mod exec;
mod platform;

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
pub struct PrinterConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>, // 配置文件版本号（可选，兼容旧版本）
    areas: Vec<Area>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    name: String,
    printers: Vec<Printer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Printer {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>, // 打印机型号（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub driver_path: Option<String>, // 驱动路径（可选，相对于应用目录）
    #[serde(default)]
    pub driver_names: Option<Vec<String>>, // 驱动名称列表（可选，用于 Windows 安装校验）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_mode: Option<String>, // 安装方式（可选）："auto" | "package" | "installer" | "ipp" | "legacy_inf"
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
pub fn load_local_config() -> Result<(PrinterConfig, std::path::PathBuf), String> {
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
// 当本地配置存在时：立即返回，不等待远程请求；远程检查在后台执行并通过事件通知
#[tauri::command]
async fn load_config(app: tauri::AppHandle) -> Result<LoadConfigResult, String> {
    // 优先加载本地配置
    match load_local_config() {
        Ok((local_config, _config_path)) => {
            let local_version = local_config.version.clone();
            
            // 本地配置加载成功，立即返回，不等待远程请求
            // 远程配置检查在后台执行，通过 tauri event 通知前端
            
            // 后台执行远程配置检查（non-blocking）
            let app_clone = app.clone();
            let local_config_clone = local_config.clone();
            let local_version_clone = local_version.clone(); // 克隆用于闭包
            tauri::async_runtime::spawn(async move {
                // 使用较短的 timeout（1500ms）避免后台任务长期占用
                let remote_result = tokio::time::timeout(
                    std::time::Duration::from_millis(1500),
                    load_remote_config()
                ).await;
                
                match remote_result {
                    Ok(Ok(remote_config)) => {
                        let remote_version = remote_config.version.clone();
                        
                        // 比较配置是否不同
                        if config_different(&local_config_clone, &remote_config) {
                            eprintln!("[INFO] 检测到远程配置更新 (本地: {:?}, 远程: {:?})", 
                                local_version_clone.as_ref().unwrap_or(&"未知".to_string()),
                                remote_version.as_ref().unwrap_or(&"未知".to_string())
                            );
                            
                            // 发送更新事件通知前端
                            let payload = serde_json::json!({
                                "has_update": true,
                                "local_version": local_version_clone,
                                "remote_version": remote_version,
                            });
                            
                            if let Err(e) = app_clone.emit_all("config_remote_update", payload) {
                                eprintln!("[WARN] 发送 config_remote_update 事件失败: {}", e);
                            }
                        }
                    }
                    Ok(Err(e)) => {
                        // 远程加载失败，只记录日志
                        eprintln!("[WARN] 远程配置加载失败: {}（已使用本地配置）", e);
                        
                        // 可选：发送错误事件
                        let payload = serde_json::json!({
                            "error": format!("远程配置加载失败: {}", e),
                        });
                        
                        if let Err(emit_err) = app_clone.emit_all("config_remote_error", payload) {
                            eprintln!("[WARN] 发送 config_remote_error 事件失败: {}", emit_err);
                        }
                    }
                    Err(_) => {
                        // 超时，只记录日志
                        eprintln!("[WARN] 远程配置加载超时（已使用本地配置）");
                        
                        // 可选：发送错误事件
                        let payload = serde_json::json!({
                            "error": "远程配置加载超时",
                        });
                        
                        if let Err(emit_err) = app_clone.emit_all("config_remote_error", payload) {
                            eprintln!("[WARN] 发送 config_remote_error 事件失败: {}", emit_err);
                        }
                    }
                }
            });
            
            // 立即返回本地配置，不等待远程请求
            Ok(LoadConfigResult {
                config: local_config,
                source: "local".to_string(),
                remote_error: None, // 本地配置存在时，不返回远程错误（后台处理）
                has_remote_update: false, // 初始为 false，后续通过事件通知
                remote_config: None, // 初始为 None，后续通过事件通知
                local_version,
                remote_version: None, // 初始为 None，后续通过事件通知
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
    crate::platform::list_printers()
}

// 获取本地已安装的打印机详细列表（包含 comment 和 location）
#[tauri::command]
fn list_printers_detailed() -> Result<Vec<crate::platform::windows::list::DetailedPrinterInfo>, String> {
    crate::platform::list_printers_detailed()
}

// 安装打印机（根据 Windows 版本选择安装方式）
// 注意：尝试使用 camelCase 参数名 driverPath，因为 Tauri 可能对带下划线的参数名 driver_path 有问题
#[tauri::command]
#[allow(non_snake_case)]  // 允许使用 camelCase，因为前端传递的是 driverPath
async fn install_printer(
    name: String, 
    path: String, 
    driverPath: Option<String>,  // 改为 camelCase，看看是否能解决问题
    model: Option<String>,
    driverInstallPolicy: Option<String>,  // 驱动安装策略："always" | "reuse_if_installed"
    installMode: Option<String>,  // 安装方式："auto" | "package" | "installer" | "ipp" | "legacy_inf"（使用 camelCase 匹配前端）
    dryRun: Option<bool>  // 测试模式：true 表示仅模拟，不执行真实安装（使用 camelCase 匹配前端）
) -> Result<InstallResult, String> {
    // 参数校验
    if name.trim().is_empty() {
        return Err("打印机名称不能为空".to_string());
    }
    
    // 验证并规范化 installMode
    let valid_modes = ["auto", "package", "installer", "ipp", "legacy_inf"];
    let normalized_mode = match &installMode {
        Some(mode) if valid_modes.contains(&mode.as_str()) => mode.clone(),
        Some(invalid_mode) => {
            eprintln!("[InstallRequest] invalid installMode=\"{}\", fallback to auto", invalid_mode);
            "auto".to_string()
        },
        None => {
            eprintln!("[InstallRequest] missing installMode, fallback to auto (raw: {:?})", installMode);
            "auto".to_string()
        }
    };
    
    // 获取 dryRun 值（默认 true，安全策略）
    let dry_run_value = dryRun.unwrap_or(true);
    
    // 打印安装请求日志（包含原始参数和解析后的值）
    eprintln!("[InstallRequest] printer=\"{}\" path=\"{}\" mode=\"{}\" dryRun={} (raw installMode: {:?}, raw dryRun: {:?})", 
        name, path, normalized_mode, dry_run_value, installMode, dryRun);
    
    // 调用平台统一的安装入口（传入 dry_run）
    crate::platform::install_printer(name, path, driverPath, model, driverInstallPolicy, installMode.clone(), dry_run_value).await
}

#[tauri::command]
fn open_url(url: String) -> Result<String, String> {
    crate::platform::open_url(&url)?;
    Ok("已打开".to_string())
}

// 打印测试页
#[tauri::command]
fn print_test_page(printer_name: String) -> Result<String, String> {
    // 参数校验
    if printer_name.trim().is_empty() {
        return Err("[PrintTestPage] ERROR step=VALIDATE message=打印机名称不能为空".to_string());
    }
    
    // 打印入参（JSON 风格，显示可见字符和长度）
    let printer_name_json = printer_name
        .chars()
        .map(|c| {
            if c.is_control() {
                format!("\\u{:04x}", c as u32)
            } else if c == '"' {
                "\\\"".to_string()
            } else if c == '\\' {
                "\\\\".to_string()
            } else {
                c.to_string()
            }
        })
        .collect::<String>();
    eprintln!("[Command] print_test_page received printer_name=\"{}\" len={} bytes={}", 
        printer_name_json, printer_name.len(), printer_name.as_bytes().len());
    
    // 调用平台统一的打印测试页入口
    crate::platform::print_test_page(printer_name)
}

// 重装打印机
#[tauri::command]
#[allow(non_snake_case)]
async fn reinstall_printer(
    configPrinterKey: String,
    configPrinterPath: String,
    configPrinterName: String,
    driverPath: Option<String>,
    model: Option<String>,
    removePort: bool,
    removeDriver: bool,
    driverInstallStrategy: Option<String>,
) -> Result<InstallResult, String> {
    if configPrinterKey.trim().is_empty() {
        return Err("配置打印机标识不能为空".to_string());
    }
    if configPrinterName.trim().is_empty() {
        return Err("配置打印机名称不能为空".to_string());
    }
    crate::platform::reinstall_printer(
        configPrinterKey, configPrinterPath, configPrinterName,
        driverPath, model, removePort, removeDriver, driverInstallStrategy,
    ).await
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
            list_printers_detailed,
            install_printer,
            open_url,
            confirm_update_config,
            print_test_page,
            check_version_update,
            download_update,
            reinstall_printer
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
            
            let title: Vec<u16> = OsStr::new("ePrinty - 启动错误")
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

