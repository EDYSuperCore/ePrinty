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
    #[serde(rename = "driverCatalog", skip_serializing_if = "Option::is_none")]
    pub driver_catalog: Option<std::collections::HashMap<String, DriverCatalogEntry>>,
    pub cities: Vec<City>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct City {
    #[serde(rename = "cityId")]
    pub city_id: String,
    #[serde(rename = "cityName")]
    pub city_name: String,
    pub areas: Vec<Area>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Area {
    #[serde(rename = "areaName")]
    pub area_name: String,
    pub printers: Vec<Printer>,
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
    #[serde(rename = "driverKey", skip_serializing_if = "Option::is_none")]
    pub driver_key: Option<String>, // 驱动目录键（可选，用于引用 driverCatalog）
}

/// 驱动目录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverCatalogEntry {
    #[serde(rename = "installMode", skip_serializing_if = "Option::is_none")]
    pub install_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local: Option<DriverLocalSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<DriverRemoteSpec>, // M1 只解析不使用
}

/// 本地驱动规格
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverLocalSpec {
    #[serde(rename = "infRel", skip_serializing_if = "Option::is_none")]
    pub inf_rel: Option<String>, // INF 文件相对路径（相对于 driversRoot）
    #[serde(rename = "driverNames", skip_serializing_if = "Option::is_none")]
    pub driver_names: Option<Vec<String>>, // 驱动名称列表
}

/// 远程驱动规格（M1 只解析不使用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriverRemoteSpec {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,
}

/// 远程驱动解析结果（M2.5/M3 使用）
#[derive(Debug, Clone)]
pub struct RemoteDriverResolved {
    pub url: String,
    pub sha256: String,
    pub version: Option<String>,
    pub layout: Option<String>,
    pub driver_key: String,
}

/// 有效驱动规格（推导结果）
#[derive(Debug, Clone)]
pub struct EffectiveDriverSpec {
    pub source: String, // "catalog" 或 "legacy"
    pub effective_install_mode: Option<String>,
    pub effective_driver_path: Option<String>, // 相对路径（以 driversRoot 为基准）
    pub effective_driver_names: Vec<String>,
    pub driver_key_used: Option<String>,
    /// 远程驱动信息（M2.5：仅解析，M3 使用）
    /// 只有当 source=catalog 且 catalog_entry.remote.url+sha256 同时存在时才为 Some
    pub remote_driver: Option<RemoteDriverResolved>,
}

/// 系统信息响应
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub platform: String, // "windows" | "macos" | "linux"
    pub os_version: String, // 例如 "Windows 11 23H2" 或 "macOS 14.0" 或 "Ubuntu 22.04" (向后兼容)
    pub arch: String, // "x64" | "x86" | "arm64"
    pub app_version: String, // 应用版本
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel_version: Option<String>, // 内核版本（可选）
    // Windows 详细版本信息（新增）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_name: Option<String>, // "Windows 11" / "Windows 10"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_display_version: Option<String>, // "24H2" / "23H2"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub build_number: Option<String>, // "26100"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ubr: Option<u32>, // 7171 (Update Build Revision)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_display: Option<String>, // "Windows 11 24H2 (Build 26100.7171)"
}

#[derive(Debug, Serialize, Deserialize)]
struct InstallResult {
    success: bool,
    message: String,
    method: Option<String>, // 安装方式："VBS" 或 "Add-Printer"
    stdout: Option<String>,  // PowerShell 标准输出
    stderr: Option<String>,  // PowerShell 错误输出
    /// 实际使用的 dry_run 值（用于前端判断是否显示"模拟"提示）
    #[serde(rename = "effectiveDryRun")]
    effective_dry_run: bool,
    /// 安装任务 ID（与后端发出的进度事件中的 jobId 一致）
    #[serde(rename = "jobId")]
    job_id: String,
}

// ============================================================================
// 安装进度事件模型（重构版本：基于 jobId + stepId）
// ============================================================================

/// 进度载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressPayload {
    pub current: Option<u64>,
    pub total: Option<u64>,
    pub unit: Option<String>, // "bytes" | "files" | "percent"
    pub percent: Option<f64>,
}

/// 错误载荷（仅 failed 时必须有）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorPayload {
    pub code: String, // 稳定错误码
    pub detail: String, // 人类可读
    pub stderr: Option<String>,
    pub stdout: Option<String>,
}

/// 安装进度事件（新版本：基于 jobId + stepId）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgressEvent {
    pub job_id: String,
    pub printer_name: String,
    pub step_id: String, // 例如 "driver.download" / "driver.extract" / "device.ensureQueue"
    pub state: String, // "running" | "success" | "failed" | "skipped"
    pub message: String,
    pub ts_ms: i64,
    pub progress: Option<ProgressPayload>,
    pub error: Option<ErrorPayload>,
    pub meta: Option<serde_json::Value>, // 可选：附加 evidence
    /// 兼容期：映射旧的 phase 名称（download/stageDriver 等）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub legacy_phase: Option<String>,
}

// ============================================================================
// 兼容旧版本的事件结构（保留用于过渡期）
// ============================================================================

/// 安装阶段枚举（旧版本，保留用于兼容）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InstallPhase {
    Download,
    Verify,
    Extract,
    StageDriver,
    RegisterDriver,
    EnsurePort,
    EnsureQueue,
    FinalVerify,
}

/// 阶段状态枚举（旧版本，保留用于兼容）
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PhaseState {
    Pending,
    Running,
    Success,
    Failed,
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

/// 调试解压 ZIP 的返回结果（可序列化）
#[derive(Debug, Serialize)]
struct DebugExtractZipResponse {
    dest_dir: String,
    file_count: usize,
    dir_count: usize,
    top_entries: Vec<String>,
    evidence: String,
}

/// 调试下载驱动包的返回结果（可序列化）
#[derive(Debug, Serialize)]
struct DebugFetchDriverPayloadResponse {
    driver_uuid: String,
    uuid_root: String,
    payload_zip: String,
    source_used: String,
    bytes: u64,
    sha256_actual: String,
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

/// 【强校验】验证配置的完整性和一致性
/// 检查项：
/// 1. driverCatalog 必须存在
/// 2. 每个 printer 的 driverKey 必须存在且在 catalog 中有对应条目
/// 3. printer 节点中不允许出现 driver_path/driver_names/install_mode 等字段（已迁移至 driverCatalog）
pub fn validate_printer_config_v2(config: &PrinterConfig) -> Result<(), String> {
    // 1. 检查 driverCatalog 存在
    let catalog = config.driver_catalog.as_ref()
        .ok_or_else(|| {
            "【配置校验失败】缺少 driverCatalog。v2.0.0+ 要求所有驱动信息必须集中到 driverCatalog。\n\n请升级 printer_config.json：\n- 将驱动信息移至 driverCatalog\n- printers[] 仅保留 name, path, model, driverKey 字段".to_string()
        })?;
    
    if catalog.is_empty() {
        return Err("【配置校验失败】driverCatalog 为空。请添加至少一个驱动条目".to_string());
    }
    
    // 2. 检查所有 printer 的 driverKey
    for city in &config.cities {
        for area in &city.areas {
            for printer in &area.printers {
                // 检查 driverKey 必须存在
                let driver_key = printer.driver_key.as_ref()
                    .ok_or_else(|| format!("【配置校验失败】打印机 '{}' (路径: {}) 缺少 driverKey。请在 printer_config.json 中补齐此字段", printer.name, printer.path))?;
                
                // 检查 driverKey 在 catalog 中是否存在
                if !catalog.contains_key(driver_key) {
                    return Err(format!(
                        "【配置校验失败】打印机 '{}' 引用的 driverKey='{}' 在 driverCatalog 中不存在。\n\n已定义的 driverKey：[{}]",
                        printer.name,
                        driver_key,
                        catalog.keys().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(", ")
                    ));
                }
            }
        }
    }
    
    // 3. 检查是否有残留的旧字段（driver_path/driver_names/install_mode）
    // 这些字段应该已从 printer 中删除，迁移至 driverCatalog
    for city in &config.cities {
        for area in &city.areas {
            for printer in &area.printers {
                let mut old_fields = vec![];
                if printer.driver_path.is_some() {
                    old_fields.push("driver_path");
                }
                if printer.driver_names.is_some() && printer.driver_names.as_ref().map(|v| !v.is_empty()).unwrap_or(false) {
                    old_fields.push("driver_names");
                }
                if printer.install_mode.is_some() {
                    old_fields.push("install_mode");
                }
                
                if !old_fields.is_empty() {
                    return Err(format!(
                        "【配置校验失败】打印机 '{}' (路径: {}) 仍包含已废弃的字段: {}。\n\n这些字段已迁移至 driverCatalog，请从 printer 节点删除",
                        printer.name,
                        printer.path,
                        old_fields.join(", ")
                    ));
                }
            }
        }
    }
    
    Ok(())
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
            
            // 【强校验】验证配置完整性
            validate_printer_config_v2(&config)?;
            
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

/// 推导有效驱动规格
/// 
/// 根据 printer 的 driver_key 和 catalog 推导出 effective_* 字段
/// 如果 driver_key 存在且 catalog 中存在对应条目，使用 catalog 中的配置
/// 否则使用 legacy 字段（printer 的原始字段）
pub fn resolve_effective_driver_spec(
    printer: &Printer,
    catalog: Option<&std::collections::HashMap<String, DriverCatalogEntry>>,
) -> EffectiveDriverSpec {
    // 检查是否可以使用 catalog
    let use_catalog = if let Some(driver_key) = &printer.driver_key {
        if let Some(cat) = catalog {
            if cat.contains_key(driver_key) {
                true
            } else {
                eprintln!("[EffectiveDriverSpec] driverKey=\"{}\" not found in catalog", driver_key);
                false
            }
        } else {
            eprintln!("[EffectiveDriverSpec] driverKey=\"{}\" specified but catalog is missing", driver_key);
            false
        }
    } else {
        false
    };

    if use_catalog {
        // 使用 catalog
        let driver_key = printer.driver_key.as_ref().unwrap();
        let catalog_entry = catalog.unwrap().get(driver_key).unwrap();
        
        let effective_install_mode = catalog_entry.install_mode.clone()
            .or_else(|| printer.install_mode.clone());
        
        let effective_driver_path = if let Some(local) = &catalog_entry.local {
            local.inf_rel.clone()
        } else {
            printer.driver_path.clone()
        };
        
        let effective_driver_names = if let Some(local) = &catalog_entry.local {
            if let Some(names) = &local.driver_names {
                if !names.is_empty() {
                    names.clone()
                } else {
                    printer.driver_names.clone().unwrap_or_default()
                }
            } else {
                printer.driver_names.clone().unwrap_or_default()
            }
        } else {
            printer.driver_names.clone().unwrap_or_default()
        };
        
        // M2.5: 解析 remote_driver 信息（仅解析，M3 使用）
        let remote_driver = if let Some(remote) = &catalog_entry.remote {
            // 只有当 url 和 sha256 同时存在时，才设置 remote_driver
            if let (Some(url), Some(sha256)) = (&remote.url, &remote.sha256) {
                if !url.trim().is_empty() && !sha256.trim().is_empty() {
                    Some(RemoteDriverResolved {
                        url: url.clone(),
                        sha256: sha256.clone(),
                        version: remote.version.clone(),
                        layout: remote.layout.clone(),
                        driver_key: driver_key.clone(),
                    })
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };
        
        EffectiveDriverSpec {
            source: "catalog".to_string(),
            effective_install_mode,
            effective_driver_path,
            effective_driver_names,
            driver_key_used: Some(driver_key.clone()),
            remote_driver,
        }
    } else {
        // 使用 legacy 字段
        EffectiveDriverSpec {
            source: "legacy".to_string(),
            effective_install_mode: printer.install_mode.clone(),
            effective_driver_path: printer.driver_path.clone(),
            effective_driver_names: printer.driver_names.clone().unwrap_or_default(),
            driver_key_used: None,
            remote_driver: None, // legacy 模式没有 remote_driver
        }
    }
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
    
    // 比较配置内容（cities）
    // 如果 cities 数量不同，肯定不同
    if local.cities.len() != remote.cities.len() {
        return true;
    }
    
    // 比较每个 city, area 和 printer
    for (local_city, remote_city) in local.cities.iter().zip(remote.cities.iter()) {
        if local_city.city_id != remote_city.city_id || local_city.areas.len() != remote_city.areas.len() {
            return true;
        }
        
        for (local_area, remote_area) in local_city.areas.iter().zip(remote_city.areas.iter()) {
            if local_area.area_name != remote_area.area_name || local_area.printers.len() != remote_area.printers.len() {
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

/// 调试：解压 ZIP 文件（仅用于开发/调试）
/// 
/// # 参数
/// - `zip_path`: ZIP 文件的绝对路径
/// - `driver_uuid`: 可选，驱动 UUID。如果为 None，自动生成 UUID 并打印出来
/// 
/// # 返回
/// - `Ok(DebugExtractZipResponse)`: 解压成功
/// - `Err(String)`: 解压失败
/// 
/// # 注意
/// - 解压输出固定为 drivers_root/<driver_uuid>/extracted/
/// - 不再支持自定义 destRel/destDir
#[tauri::command]
#[cfg(windows)]
fn debug_extract_zip(zip_path: String, driver_uuid: Option<String>) -> Result<DebugExtractZipResponse, String> {
    use std::path::Path;
    
    eprintln!("[DebugExtractZip] start zip_path=\"{}\" driver_uuid={:?}", zip_path, driver_uuid);
    
    // 解析 zip_path
    let zip_path_buf = Path::new(&zip_path);
    if !zip_path_buf.exists() {
        return Err(format!("zip_not_found: {}", zip_path));
    }
    
    // 获取 AppDir 和 drivers_root
    let app_dir = match crate::platform::windows::install::get_app_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return Err(format!("无法获取应用目录: {}", e));
        }
    };
    
    let drivers_root = app_dir.join("drivers");
    
    // 确定 driver_uuid
    let final_driver_uuid = if let Some(uuid) = driver_uuid {
        uuid
    } else {
        // 自动生成 UUID
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let generated_uuid = format!("debug_{:016x}", timestamp);
        eprintln!("[DebugExtractZip] auto_generated_uuid=\"{}\"", generated_uuid);
        generated_uuid
    };
    
    eprintln!("[DebugExtractZip] zip_path=\"{}\" driver_uuid=\"{}\" drivers_root=\"{}\"", 
        zip_path, final_driver_uuid, drivers_root.display());
    
    // 调用 extract_zip_for_driver（debug 命令不需要进度事件）
    match crate::platform::windows::archive::extract_zip_for_driver(zip_path_buf, &drivers_root, &final_driver_uuid, None, None, "debug_job") {
        Ok(result) => {
            eprintln!("[DebugExtractZip] success driver_uuid=\"{}\" uuid_root=\"{}\" extracted_root=\"{}\" file_count={}", 
                result.driver_uuid, result.uuid_root.display(), result.extracted_root.display(), result.file_count);
            
            Ok(DebugExtractZipResponse {
                dest_dir: result.extracted_root.display().to_string(),
                file_count: result.file_count,
                dir_count: 0, // ExtractForDriverResult 不包含 dir_count，设为 0
                top_entries: result.top_entries,
                evidence: format!("driver_uuid={} uuid_root={} extracted_root={}", 
                    result.driver_uuid, result.uuid_root.display(), result.extracted_root.display()),
            })
        }
        Err(e) => {
            let error_msg = format!("解压失败: {}", e);
            eprintln!("[DebugExtractZip] failed error=\"{}\"", error_msg);
            Err(error_msg)
        }
    }
}

#[cfg(not(windows))]
#[tauri::command]
fn debug_extract_zip(_zip_path: String, _dest_rel: Option<String>) -> Result<DebugExtractZipResponse, String> {
    Err("debug_extract_zip 仅在 Windows 平台可用".to_string())
}

/// 调试：下载驱动包（仅用于开发/调试）
/// 
/// # 参数
/// - `remote_url`: 远程 ZIP 文件 URL
/// - `sha256`: 期望的 SHA256 哈希值（64 字符十六进制）
/// 
/// # 返回
/// - `Ok(DebugFetchDriverPayloadResponse)`: 下载/缓存成功
/// - `Err(String)`: 下载/校验失败
/// 
/// # 注意
/// - 下载输出固定为 drivers_root/<driver_uuid>/payload/payload.zip
/// - driver_uuid = "drv_" + sha256[0..12]
#[tauri::command]
#[cfg(windows)]
async fn debug_fetch_driver_payload(remote_url: String, sha256: String) -> Result<DebugFetchDriverPayloadResponse, String> {
    eprintln!("[DebugFetchDriverPayload] start remote_url=\"{}\" sha256=\"{}\"", remote_url, sha256);
    
    // 获取 AppDir 和 drivers_root
    let app_dir = match crate::platform::windows::install::get_app_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return Err(format!("无法获取应用目录: {}", e));
        }
    };
    
    let drivers_root = app_dir.join("drivers");
    
    eprintln!("[DebugFetchDriverPayload] remote_url=\"{}\" sha256=\"{}\" drivers_root=\"{}\"", 
        remote_url, sha256, drivers_root.display());
    
    // 调用 ensure_payload_zip（debug 命令不需要进度事件）
    match crate::platform::windows::driver_fetch::ensure_payload_zip(&drivers_root, &remote_url, &sha256, None, None, "debug_job").await {
        Ok(result) => {
            eprintln!("[DebugFetchDriverPayload] success driver_uuid=\"{}\" uuid_root=\"{}\" payload_zip=\"{}\" source_used=\"{}\" bytes={} sha256_actual=\"{}\"", 
                result.driver_uuid, result.uuid_root.display(), result.payload_zip.display(), 
                result.source_used, result.bytes, result.sha256_actual);
            
            Ok(DebugFetchDriverPayloadResponse {
                driver_uuid: result.driver_uuid,
                uuid_root: result.uuid_root.display().to_string(),
                payload_zip: result.payload_zip.display().to_string(),
                source_used: result.source_used,
                bytes: result.bytes,
                sha256_actual: result.sha256_actual,
            })
        }
        Err(e) => {
            let error_msg = format!("下载失败: {}", e);
            eprintln!("[DebugFetchDriverPayload] failed error=\"{}\"", error_msg);
            Err(error_msg)
        }
    }
}

#[cfg(not(windows))]
#[tauri::command]
async fn debug_fetch_driver_payload(_remote_url: String, _sha256: String) -> Result<DebugFetchDriverPayloadResponse, String> {
    Err("debug_fetch_driver_payload 仅在 Windows 平台可用".to_string())
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

// 安装打印机（v2.0.0+ 使用 driverKey 从 driverCatalog 获取驱动规格）
#[tauri::command]
#[allow(non_snake_case)]
async fn install_printer(
    app: tauri::AppHandle,
    name: String, 
    path: String, 
    driverKey: Option<String>,  // v2.0.0+：使用 driverKey 替代 driverPath
    _driverPath: Option<String>,  // 向后兼容：仅在 driverKey 为空时使用
    model: Option<String>,
    driverInstallPolicy: Option<String>,  // 驱动安装策略："always" | "reuse_if_installed"
    installMode: Option<String>,  // 安装方式："auto" | "package" | "installer" | "ipp" | "legacy_inf"
    dryRun: Option<bool>  // 测试模式
) -> Result<InstallResult, String> {
    // 参数校验
    if name.trim().is_empty() {
        return Err("打印机名称不能为空".to_string());
    }
    
    // v2.0.0+ 强制要求 driverKey
    let effective_driver_key = driverKey.ok_or_else(|| {
        "【必填参数】driverKey 为空。v2.0.0+ 要求所有安装请求必须提供 driverKey。\n\n请检查：\n1. printer_config.json 中是否每个打印机都有 driverKey\n2. 前端是否正确传递了 driverKey".to_string()
    })?;
    
    // 加载配置以获取驱动规格和 driverCatalog
    let (config, _) = load_local_config()
        .map_err(|e| format!("加载配置失败: {}", e))?;
    
    // 从 driverCatalog 中查找驱动条目
    let catalog = config.driver_catalog.as_ref()
        .ok_or_else(|| "配置缺少 driverCatalog".to_string())?;
    
    let _catalog_entry = catalog.get(&effective_driver_key)
        .ok_or_else(|| format!(
            "driverKey='{}' 在 driverCatalog 中不存在。已定义的 driverKey: [{}]",
            effective_driver_key,
            catalog.keys().map(|s| format!("'{}'", s)).collect::<Vec<_>>().join(", ")
        ))?;
    
    // ========== installMode 决策逻辑（新） ==========
    // 规范化 installMode（遵循 "前端 → driverCatalog → auto" 优先级）
    let valid_modes = ["auto", "package", "installer", "ipp", "legacy_inf"];
    
    let install_mode = if let Some(frontend_mode) = installMode {
        // 1. 前端明确传入 → 直接使用（经过校验）
        if !valid_modes.contains(&frontend_mode.as_str()) {
            return Err(format!(
                "invalid installMode=\"{}\" from frontend. allowed: auto|package|installer|ipp|legacy_inf",
                frontend_mode
            ));
        }
        eprintln!("[InstallRequest] using frontend installMode=\"{}\"", frontend_mode);
        frontend_mode
    } else if let Some(catalog_mode) = &_catalog_entry.install_mode {
        // 2. 前端未传 → 尝试从 driverCatalog 读取
        if !valid_modes.contains(&catalog_mode.as_str()) {
            eprintln!("[InstallRequest] WARNING: invalid driverCatalog.installMode=\"{}\", fallback to auto", catalog_mode);
            "auto".to_string()
        } else {
            eprintln!("[InstallRequest] using driverCatalog installMode=\"{}\"", catalog_mode);
            catalog_mode.clone()
        }
    } else {
        // 3. 两者都没有 → 默认 auto
        eprintln!("[InstallRequest] no installMode specified, default to auto");
        "auto".to_string()
    };

    // 验证 driverInstallPolicy（禁止静默回退）
    let valid_policies = ["always", "reuse_if_installed"];
    let driver_install_policy = match &driverInstallPolicy {
        Some(policy) if valid_policies.contains(&policy.as_str()) => policy.clone(),
        Some(invalid_policy) => {
            return Err(format!(
                "invalid driverInstallPolicy=\"{}\". allowed: always|reuse_if_installed",
                invalid_policy
            ));
        }
        None => {
            return Err("driverInstallPolicy is required. allowed: always|reuse_if_installed".to_string());
        }
    };
    
    // 获取 dryRun 值（默认 true，安全策略）
    let dry_run_value = dryRun.unwrap_or(true);
    
    // 打印安装请求日志（最终确定的值）
    eprintln!("[InstallRequest] FINAL printer=\"{}\" path=\"{}\" driverKey=\"{}\" installMode=\"{}\" policy=\"{}\" dryRun={}", 
        name, path, effective_driver_key, install_mode, driver_install_policy, dry_run_value);
    
    // 构造 Printer 对象以便 resolve_effective_driver_spec 能使用
    let printer = Printer {
        name: name.clone(),
        path: path.clone(),
        model: model.clone(),
        driver_path: None,  // v2.0.0+ 从 driverCatalog 获取
        driver_names: None, // v2.0.0+ 从 driverCatalog 获取
        install_mode: None, // v2.0.0+ 从 driverCatalog 获取
        driver_key: Some(effective_driver_key.clone()),
    };
    
    // 推导有效驱动规格
    let effective_spec = resolve_effective_driver_spec(&printer, config.driver_catalog.as_ref());
    
    eprintln!("[InstallRequest] resolved_spec: source={}, install_mode={:?}, driver_path={:?}, driver_names={:?}",
        effective_spec.source,
        effective_spec.effective_install_mode,
        effective_spec.effective_driver_path,
        effective_spec.effective_driver_names
    );
    
    // 调用平台统一的安装入口（使用 resolved 字段）
    crate::platform::install_printer(
        app,
        name,
        path,
        effective_spec.effective_driver_path,  // 从 driverCatalog 解析
        model,
        Some(driver_install_policy),
        Some(effective_driver_key),  // v2.0.0+：传递 driverKey 给后端，写入 job.init meta
        Some(install_mode),
        dry_run_value,
    )
    .await
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
    app: tauri::AppHandle,  // 用于发送进度事件
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
        app,
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

/// 获取系统信息
#[tauri::command]
fn get_system_info() -> Result<SystemInfo, String> {
    // 获取应用版本
    let app_version = env!("CARGO_PKG_VERSION").to_string();
    
    // 获取架构信息
    #[cfg(target_arch = "x86_64")]
    let arch = "x64".to_string();
    #[cfg(target_arch = "x86")]
    let arch = "x86".to_string();
    #[cfg(target_arch = "aarch64")]
    let arch = "arm64".to_string();
    #[cfg(not(any(target_arch = "x86_64", target_arch = "x86", target_arch = "aarch64")))]
    let arch = std::env::consts::ARCH.to_string();
    
    // 获取平台和系统版本信息
    #[cfg(target_os = "windows")]
    {
        let platform = "windows".to_string();
        
        // 获取 Windows 版本信息（从注册表）
        let (os_version, os_name, os_display_version, build_number, ubr, os_display) = 
            get_windows_version().unwrap_or_else(|_| (
                "Windows (未知版本)".to_string(),
                None,
                None,
                None,
                None,
                None,
            ));
        
        Ok(SystemInfo {
            platform,
            os_version,
            arch,
            app_version,
            kernel_version: None,
            os_name,
            os_display_version,
            build_number,
            ubr,
            os_display,
        })
    }
    
    #[cfg(target_os = "macos")]
    {
        let platform = "macos".to_string();
        
        // 获取 macOS 版本信息
        let os_version = get_macos_version()
            .unwrap_or_else(|_| "macOS (未知版本)".to_string());
        
        Ok(SystemInfo {
            platform,
            os_version,
            arch,
            app_version,
            kernel_version: None,
            os_name: None,
            os_display_version: None,
            build_number: None,
            ubr: None,
            os_display: None,
        })
    }
    
    #[cfg(target_os = "linux")]
    {
        let platform = "linux".to_string();
        
        // 获取 Linux 版本信息
        let os_version = get_linux_version()
            .unwrap_or_else(|_| "Linux (未知版本)".to_string());
        
        Ok(SystemInfo {
            platform,
            os_version,
            arch,
            app_version,
            kernel_version: None,
            os_name: None,
            os_display_version: None,
            build_number: None,
            ubr: None,
            os_display: None,
        })
    }
    
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Ok(SystemInfo {
            platform: std::env::consts::OS.to_string(),
            os_version: "未知系统".to_string(),
            arch,
            app_version,
            kernel_version: None,
            os_name: None,
            os_display_version: None,
            build_number: None,
            ubr: None,
            os_display: None,
        })
    }
}

#[cfg(windows)]
fn get_windows_version() -> Result<(String, Option<String>, Option<String>, Option<String>, Option<u32>, Option<String>), String> {
    use winapi::um::winreg::*;
    use winapi::shared::minwindef::*;
    use std::ptr;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    
    unsafe {
        let mut h_key: HKEY = ptr::null_mut();
        let subkey = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\0"
            .encode_utf16()
            .collect::<Vec<u16>>();
        
        // 打开注册表键
        let result = RegOpenKeyExW(
            winapi::um::winreg::HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            0,
            winapi::um::winnt::KEY_READ,
            &mut h_key,
        );
        
        if result != 0 {
            return Err(format!("无法打开注册表键: {}", result));
        }
        
        // 读取字符串值的辅助函数
        let read_string_value = |value_name: &str| -> Option<String> {
            let value_name_wide = format!("{}\0", value_name)
                .encode_utf16()
                .collect::<Vec<u16>>();
            
            let mut buffer = vec![0u16; 256];
            let mut size = (buffer.len() * 2) as DWORD;
            
            let result = RegQueryValueExW(
                h_key,
                value_name_wide.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                buffer.as_mut_ptr() as *mut u8,
                &mut size,
            );
            
            if result == 0 {
                let len = (size as usize) / 2;
                let s = OsString::from_wide(&buffer[..len])
                    .to_string_lossy()
                    .trim_end_matches('\0')
                    .to_string();
                Some(s)
            } else {
                None
            }
        };
        
        // 读取 DWORD 值的辅助函数
        let read_dword_value = |value_name: &str| -> Option<u32> {
            let value_name_wide = format!("{}\0", value_name)
                .encode_utf16()
                .collect::<Vec<u16>>();
            
            let mut value: DWORD = 0;
            let mut size = std::mem::size_of::<DWORD>() as DWORD;
            
            let result = RegQueryValueExW(
                h_key,
                value_name_wide.as_ptr(),
                ptr::null_mut(),
                ptr::null_mut(),
                &mut value as *mut _ as *mut u8,
                &mut size,
            );
            
            if result == 0 {
                Some(value)
            } else {
                None
            }
        };
        
        // 读取各个字段
        let product_name = read_string_value("ProductName");
        let display_version = read_string_value("DisplayVersion");
        let current_build = read_string_value("CurrentBuildNumber");
        let ubr = read_dword_value("UBR");
        
        // 关闭注册表键
        winapi::um::winreg::RegCloseKey(h_key);
        
        // 解析 OS 名称（Windows 10/11）
        let os_name = if let Some(ref build) = current_build {
            if let Ok(build_num) = build.parse::<u32>() {
                if build_num >= 22000 {
                    Some("Windows 11".to_string())
                } else {
                    Some("Windows 10".to_string())
                }
            } else if let Some(ref pn) = product_name {
                if pn.contains("Windows 11") {
                    Some("Windows 11".to_string())
                } else if pn.contains("Windows 10") {
                    Some("Windows 10".to_string())
                } else {
                    Some("Windows".to_string())
                }
            } else {
                Some("Windows".to_string())
            }
        } else if let Some(ref pn) = product_name {
            if pn.contains("Windows 11") {
                Some("Windows 11".to_string())
            } else if pn.contains("Windows 10") {
                Some("Windows 10".to_string())
            } else {
                Some("Windows".to_string())
            }
        } else {
            Some("Windows".to_string())
        };
        
        // 组装显示字符串
        let os_display = match (&os_name, &display_version, &current_build, &ubr) {
            (Some(name), Some(ver), Some(build), Some(ubr_val)) => {
                Some(format!("{} {} (Build {}.{})", name, ver, build, ubr_val))
            }
            (Some(name), None, Some(build), Some(ubr_val)) => {
                Some(format!("{} (Build {}.{})", name, build, ubr_val))
            }
            (Some(name), Some(ver), Some(build), None) => {
                Some(format!("{} {} (Build {})", name, ver, build))
            }
            (Some(name), None, Some(build), None) => {
                Some(format!("{} (Build {})", name, build))
            }
            (Some(name), _, _, _) => Some(name.clone()),
            _ => product_name.clone(),
        };
        
        // 向后兼容的 os_version 字段
        let os_version = os_display.clone().unwrap_or_else(|| "Windows".to_string());
        
        Ok((os_version, os_name, display_version, current_build, ubr, os_display))
    }
}

#[cfg(target_os = "macos")]
fn get_macos_version() -> Result<String, String> {
    use std::process::Command;
    
    let output = Command::new("sw_vers")
        .args(&["-productName"])
        .output()
        .map_err(|e| format!("执行 sw_vers 失败: {}", e))?;
    
    if output.status.success() {
        let mut version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        
        // 获取版本号
        let version_output = Command::new("sw_vers")
            .args(&["-productVersion"])
            .output()
            .map_err(|e| format!("获取 macOS 版本号失败: {}", e))?;
        
        if version_output.status.success() {
            let version_num = String::from_utf8_lossy(&version_output.stdout).trim().to_string();
            version.push(' ');
            version.push_str(&version_num);
        }
        
        return Ok(version);
    }
    
    Err("无法获取 macOS 版本信息".to_string())
}

#[cfg(target_os = "linux")]
fn get_linux_version() -> Result<String, String> {
    use std::fs;
    
    // 尝试读取 /etc/os-release 文件
    if let Ok(content) = fs::read_to_string("/etc/os-release") {
        let mut name = String::new();
        let mut version = String::new();
        
        for line in content.lines() {
            if line.starts_with("NAME=") {
                name = line[5..].trim_matches('"').to_string();
            } else if line.starts_with("VERSION_ID=") {
                version = line[11..].trim_matches('"').to_string();
            }
        }
        
        if !name.is_empty() {
            if !version.is_empty() {
                return Ok(format!("{} {}", name, version));
            }
            return Ok(name);
        }
    }
    
    // 备选方案：使用 lsb_release 命令
    use std::process::Command;
    let output = Command::new("lsb_release")
        .args(&["-d"])
        .output();
    
    if let Ok(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // 从输出中提取版本信息，例如 "Description:    Ubuntu 22.04.1 LTS"
            if let Some(line) = stdout.lines().next() {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() > 1 {
                    return Ok(parts[1].trim().to_string());
                }
            }
        }
    }
    
    Err("无法获取 Linux 版本信息".to_string())
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
            get_system_info,
            reinstall_printer,
            debug_extract_zip,
            debug_fetch_driver_payload
        ])
        .setup(|app| {
            // 启动后延迟 800ms 发送进度事件自检
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;
                
                let timestamp_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                
                let selfcheck_event = InstallProgressEvent {
                    job_id: "selfcheck".to_string(),
                    printer_name: "selfcheck".to_string(),
                    step_id: "job.init".to_string(),
                    state: "running".to_string(),
                    message: "selfcheck".to_string(),
                    ts_ms: timestamp_ms as i64,
                    progress: Some(ProgressPayload {
                        current: None,
                        total: None,
                        unit: None,
                        percent: Some(1.0),
                    }),
                    error: None,
                    meta: None,
                    legacy_phase: Some("download".to_string()),
                };
                
                match app_handle.emit_all("install_progress", &selfcheck_event) {
                    Ok(_) => {
                        eprintln!("[ProgressEmit] selfcheck emitted");
                    }
                    Err(e) => {
                        eprintln!("[ProgressEmit] selfcheck emit failed: {}", e);
                    }
                }
            });
            
            Ok(())
        })
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

