// Windows 平台驱动包引导（Bootstrap）模块
// 
// 提供从远程下载、解压到 materialize 的完整流程

use std::path::{Path, PathBuf};
use std::fs;
use tauri::Manager;

use crate::RemoteDriverResolved;

/// Materialize 结果
#[derive(Debug, Clone)]
pub struct MaterializeResult {
    pub copied_files: usize,
    pub top_entries: Vec<String>,
    pub src_root: PathBuf,
    pub dest_root: PathBuf,
}

/// Materialize 错误类型
#[derive(Debug)]
pub enum MaterializeError {
    /// IO 错误
    IoError {
        step: &'static str,
        operation: &'static str,
        error: String,
    },
    /// 源目录不存在
    SourceNotFound {
        src_root: String,
    },
    /// 目标目录创建失败
    DestCreationFailed {
        dest_root: String,
        error: String,
    },
}

impl std::fmt::Display for MaterializeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MaterializeError::IoError { step, operation, error } => {
                write!(f, "IO 错误 (step={}, operation={}): {}", step, operation, error)
            }
            MaterializeError::SourceNotFound { src_root } => {
                write!(f, "源目录不存在: {}", src_root)
            }
            MaterializeError::DestCreationFailed { dest_root, error } => {
                write!(f, "目标目录创建失败: {} | 错误: {}", dest_root, error)
            }
        }
    }
}

impl std::error::Error for MaterializeError {}

/// 将 extracted 内容合并到 drivers_root
/// 
/// # 参数
/// - `extracted_root`: 解压后的根目录（通常是 `drivers_root/<driver_uuid>/extracted/`）
/// - `drivers_root`: 驱动根目录（目标目录）
/// - `layout`: 布局类型（可选）
///   - `Some("contains_drivers_dir")`: 源目录包含 `drivers/` 子目录，需要从 `extracted_root/drivers/` 开始复制
///   - `None` 或其他: 直接从 `extracted_root` 开始复制
/// 
/// # 返回
/// - `Ok(MaterializeResult)`: 合并成功
/// - `Err(MaterializeError)`: 合并失败
/// 
/// # 行为
/// - 覆盖同名文件（copy overwrite）
/// - 不删除 drivers_root 中其他文件
/// - 递归复制所有文件和目录
pub fn materialize_driver_tree(
    extracted_root: &Path,
    drivers_root: &Path,
    layout: Option<&str>,
) -> Result<MaterializeResult, MaterializeError> {
    eprintln!("[MaterializeDriverTree] start extracted_root=\"{}\" drivers_root=\"{}\" layout={:?}", 
        extracted_root.display(), drivers_root.display(), layout);
    
    // ============================================================================
    // Step 1: 确定源目录（src_root）
    // ============================================================================
    let src_root = if layout == Some("contains_drivers_dir") {
        extracted_root.join("drivers")
    } else {
        PathBuf::from(extracted_root)
    };
    
    eprintln!("[MaterializeDriverTree] step=determine_src_root inputs=extracted_root=\"{}\" layout={:?} outputs=src_root=\"{}\"", 
        extracted_root.display(), layout, src_root.display());
    
    // 检查源目录是否存在
    if !src_root.exists() {
        return Err(MaterializeError::SourceNotFound {
            src_root: src_root.display().to_string(),
        });
    }
    
    if !src_root.is_dir() {
        return Err(MaterializeError::IoError {
            step: "determine_src_root",
            operation: "检查源目录",
            error: format!("源路径不是目录: {}", src_root.display()),
        });
    }
    
    // ============================================================================
    // Step 2: 确保目标目录存在
    // ============================================================================
    eprintln!("[MaterializeDriverTree] step=ensure_dest_root inputs=drivers_root=\"{}\"", drivers_root.display());
    
    if let Err(e) = fs::create_dir_all(drivers_root) {
        return Err(MaterializeError::DestCreationFailed {
            dest_root: drivers_root.display().to_string(),
            error: format!("无法创建目标目录: {}", e),
        });
    }
    
    eprintln!("[MaterializeDriverTree] step=ensure_dest_root result=success drivers_root=\"{}\"", drivers_root.display());
    
    // ============================================================================
    // Step 3: 递归复制文件
    // ============================================================================
    eprintln!("[MaterializeDriverTree] step=copy_files inputs=src_root=\"{}\" dest_root=\"{}\"", 
        src_root.display(), drivers_root.display());
    
    let mut copied_files = 0usize;
    let mut top_entries = Vec::new();
    
    // 使用 walkdir 递归遍历源目录
    use walkdir::WalkDir;
    
    for entry in WalkDir::new(&src_root).into_iter() {
        let entry = entry.map_err(|e| MaterializeError::IoError {
            step: "copy_files",
            operation: "遍历源目录",
            error: format!("无法遍历源目录 {}: {}", src_root.display(), e),
        })?;
        
        let src_path = entry.path();
        let src_relative = src_path.strip_prefix(&src_root)
            .map_err(|e| MaterializeError::IoError {
                step: "copy_files",
                operation: "计算相对路径",
                error: format!("无法计算相对路径 {}: {}", src_path.display(), e),
            })?;
        
        let dest_path = drivers_root.join(src_relative);
        
        // 记录顶层条目（仅第一层）
        if src_relative.components().count() == 1 {
            top_entries.push(src_relative.display().to_string());
        }
        
        if src_path.is_dir() {
            // 创建目标目录
            if let Err(e) = fs::create_dir_all(&dest_path) {
                return Err(MaterializeError::IoError {
                    step: "copy_files",
                    operation: "创建目标目录",
                    error: format!("无法创建目标目录 {}: {}", dest_path.display(), e),
                });
            }
        } else if src_path.is_file() {
            // 创建父目录（如果不存在）
            if let Some(parent) = dest_path.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    return Err(MaterializeError::IoError {
                        step: "copy_files",
                        operation: "创建父目录",
                        error: format!("无法创建父目录 {}: {}", parent.display(), e),
                    });
                }
            }
            
            // 复制文件（覆盖同名文件）
            if let Err(e) = fs::copy(src_path, &dest_path) {
                return Err(MaterializeError::IoError {
                    step: "copy_files",
                    operation: "复制文件",
                    error: format!("无法复制文件 {} -> {}: {}", src_path.display(), dest_path.display(), e),
                });
            }
            
            copied_files += 1;
        }
    }
    
    eprintln!("[MaterializeDriverTree] step=copy_files result=success copied_files={} top_entries_count={}", 
        copied_files, top_entries.len());
    
    // ============================================================================
    // Step 4: 输出摘要
    // ============================================================================
    let evidence = format!(
        "step=summary src_root=\"{}\" dest_root=\"{}\" copied_files={} top_entries_count={}",
        src_root.display(), drivers_root.display(), copied_files, top_entries.len()
    );
    
    eprintln!("[MaterializeDriverTree] step=summary result=success evidence=\"{}\"", evidence);
    
    Ok(MaterializeResult {
        copied_files,
        top_entries,
        src_root,
        dest_root: drivers_root.to_path_buf(),
    })
}

/// Bootstrap 结果
#[derive(Debug, Clone)]
pub struct BootstrapResult {
    pub driver_uuid: String,
    pub payload_zip: PathBuf,
    pub extracted_root: PathBuf,
    pub materialize_result: MaterializeResult,
}

/// Bootstrap 错误类型
#[derive(Debug)]
pub enum BootstrapError {
    /// 远程驱动信息缺失
    RemoteDriverMissing {
        reason: String,
    },
    /// 下载失败
    FetchFailed {
        error: String,
    },
    /// 解压失败
    ExtractFailed {
        error: String,
    },
    /// Materialize 失败
    MaterializeFailed {
        error: String,
    },
    /// INF 文件在 bootstrap 后仍不存在
    InfNotFoundAfterBootstrap {
        effective_path: String,
        inf_abs_path: String,
        evidence: String,
    },
}

impl std::fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BootstrapError::RemoteDriverMissing { reason } => {
                write!(f, "远程驱动信息缺失: {}", reason)
            }
            BootstrapError::FetchFailed { error } => {
                write!(f, "下载失败: {}", error)
            }
            BootstrapError::ExtractFailed { error } => {
                write!(f, "解压失败: {}", error)
            }
            BootstrapError::MaterializeFailed { error } => {
                write!(f, "Materialize 失败: {}", error)
            }
            BootstrapError::InfNotFoundAfterBootstrap { effective_path, inf_abs_path, evidence } => {
                write!(f, "Bootstrap 后 INF 文件仍不存在\n有效路径: {}\n绝对路径: {}\n证据: {}", 
                    effective_path, inf_abs_path, evidence)
            }
        }
    }
}

impl std::error::Error for BootstrapError {}

/// 执行完整的 bootstrap 流程：下载 → 解压 → materialize
/// 
/// # 参数
/// - `remote_driver`: 远程驱动信息
/// - `drivers_root`: 驱动根目录
/// - `effective_driver_path`: 有效驱动路径（用于最终验证）
/// 
/// # 返回
/// - `Ok(BootstrapResult)`: Bootstrap 成功
/// - `Err(BootstrapError)`: Bootstrap 失败
pub async fn bootstrap_driver_from_remote(
    remote_driver: &RemoteDriverResolved,
    drivers_root: &Path,
    effective_driver_path: &str,
    app: Option<&tauri::AppHandle>,  // 用于发送进度事件（可选）
    printer_name: Option<&str>,  // 打印机名称（用于进度事件）
    job_id: &str,  // 安装任务 ID
) -> Result<BootstrapResult, BootstrapError> {
    eprintln!("[DriverBootstrap] start driver_key=\"{}\" effective_driver_path=\"{}\" drivers_root=\"{}\"", 
        remote_driver.driver_key, effective_driver_path, drivers_root.display());
    
    // ============================================================================
    // Step 1: fetch_payload - 下载 payload.zip
    // ============================================================================
    eprintln!("[DriverBootstrap] step=fetch_payload inputs=url=\"{}\" sha256=\"{}\"", 
        remote_driver.url, remote_driver.sha256);
    
    let fetch_result = crate::platform::windows::driver_fetch::ensure_payload_zip(
        drivers_root,
        &remote_driver.url,
        &remote_driver.sha256,
        app,
        printer_name,
        job_id,
    ).await.map_err(|e| BootstrapError::FetchFailed {
        error: format!("{}", e),
    })?;
    
    eprintln!("[DriverBootstrap] step=fetch_payload result=success driver_uuid=\"{}\" payload_zip=\"{}\" source_used=\"{}\" bytes={}", 
        fetch_result.driver_uuid, fetch_result.payload_zip.display(), fetch_result.source_used, fetch_result.bytes);
    
    // ============================================================================
    // Step 2: extract_payload - 解压 payload.zip
    // ============================================================================
    eprintln!("[DriverBootstrap] step=extract_payload inputs=payload_zip=\"{}\" driver_uuid=\"{}\"", 
        fetch_result.payload_zip.display(), fetch_result.driver_uuid);
    
    let extract_result = crate::platform::windows::archive::extract_zip_for_driver(
        &fetch_result.payload_zip,
        drivers_root,
        &fetch_result.driver_uuid,
        app,
        printer_name,
        job_id,
    ).map_err(|e| BootstrapError::ExtractFailed {
        error: format!("{}", e),
    })?;
    
    eprintln!("[DriverBootstrap] step=extract_payload result=success extracted_root=\"{}\" file_count={}", 
        extract_result.extracted_root.display(), extract_result.file_count);
    
    // ============================================================================
    // Step 3: materialize - 将 extracted 内容合并到 drivers_root
    // ============================================================================
    eprintln!("[DriverBootstrap] step=materialize inputs=extracted_root=\"{}\" drivers_root=\"{}\" layout={:?}", 
        extract_result.extracted_root.display(), drivers_root.display(), remote_driver.layout);
    
    let materialize_result = materialize_driver_tree(
        &extract_result.extracted_root,
        drivers_root,
        remote_driver.layout.as_deref(),
    ).map_err(|e| BootstrapError::MaterializeFailed {
        error: format!("{}", e),
    })?;
    
    eprintln!("[DriverBootstrap] step=materialize result=success copied_files={} top_entries_count={}", 
        materialize_result.copied_files, materialize_result.top_entries.len());
    
    // 发送 materialize 成功事件
    if let (Some(app_handle), Some(printer)) = (app, printer_name) {
        let reporter = crate::platform::windows::step_reporter::StepReporter::start(
            std::sync::Arc::new(app_handle.clone()),
            job_id.to_string(),
            printer.to_string(),
            "driver.extract".to_string(),
            "正在合并文件到驱动目录".to_string(),
        );
        let meta = serde_json::json!({
            "copied_files": materialize_result.copied_files,
            "top_entries_count": materialize_result.top_entries.len(),
        });
        let _ = reporter.success(
            format!("已合并 {} 个文件到驱动目录", materialize_result.copied_files),
            Some(meta),
        );
    }
    
    // ============================================================================
    // Step 4: summary
    // ============================================================================
    let evidence = format!(
        "step=summary driver_uuid=\"{}\" payload_zip=\"{}\" extracted_root=\"{}\" copied_files={}",
        fetch_result.driver_uuid, fetch_result.payload_zip.display(), 
        extract_result.extracted_root.display(), materialize_result.copied_files
    );
    
    eprintln!("[DriverBootstrap] step=summary result=success evidence=\"{}\"", evidence);
    
    Ok(BootstrapResult {
        driver_uuid: fetch_result.driver_uuid,
        payload_zip: fetch_result.payload_zip,
        extracted_root: extract_result.extracted_root,
        materialize_result,
    })
}
