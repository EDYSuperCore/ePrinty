// Windows 平台 ZIP 解压模块
// 
// 提供 ZIP 解压功能，包含 zip-slip 安全检测

use std::path::{Path, PathBuf};
use std::fs;
use std::io;
use tauri::Manager;

/// 解压结果（旧版，保留用于兼容）
#[derive(Debug, Clone)]
pub struct ExtractResult {
    pub extracted_files: usize,
    pub extracted_dirs: usize,
    pub evidence: String,
    pub dest_dir: PathBuf,
    pub top_level_items: Vec<String>,
}

/// 驱动解压结果（新版，固定落点）
#[derive(Debug, Clone)]
pub struct ExtractForDriverResult {
    pub driver_uuid: String,
    pub uuid_root: PathBuf,
    pub extracted_root: PathBuf,
    pub file_count: usize,
    pub top_entries: Vec<String>,
}

/// 解压错误类型
#[derive(Debug)]
pub enum ExtractError {
    /// ZIP 文件不存在
    ZipNotFound {
        zip_path: String,
    },
    /// 解压失败（PowerShell Expand-Archive 执行失败）
    ExtractFailed {
        step: &'static str,
        zip_path: String,
        dest_dir: String,
        stdout: String,
        stderr: String,
        exit_code: Option<i32>,
    },
    /// 检测到 zip-slip 攻击
    ZipSlipDetected {
        offending_path: String,
        canonical_dest_dir: String,
        canonical_offending_path: String,
    },
    /// 目标目录不安全
    UnsafeDestination {
        dest_dir: String,
        drivers_root: String,
        cache_root: String,
        reason: String,
    },
    /// 驱动 UUID 格式无效
    InvalidDriverUuid {
        driver_uuid: String,
        reason: String,
    },
    /// IO 错误
    IoError {
        step: &'static str,
        operation: &'static str,
        error: String,
    },
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractError::ZipNotFound { zip_path } => {
                write!(f, "ZIP 文件不存在: {}", zip_path)
            }
            ExtractError::ExtractFailed { step, zip_path, dest_dir, stdout, stderr, exit_code } => {
                let exit_msg = match exit_code {
                    Some(code) => format!("退出代码: {}", code),
                    None => "无法获取退出代码".to_string(),
                };
                write!(f, "解压失败 (step={}): {} | {} | ZIP: {} | Dest: {}", 
                    step, exit_msg, stderr, zip_path, dest_dir)
            }
            ExtractError::ZipSlipDetected { offending_path, canonical_dest_dir, canonical_offending_path } => {
                write!(f, "检测到 zip-slip 攻击: 文件路径 \"{}\" 越界\n规范化的目标目录: {}\n规范化的文件路径: {}", 
                    offending_path, canonical_dest_dir, canonical_offending_path)
            }
            ExtractError::UnsafeDestination { dest_dir, drivers_root, cache_root, reason } => {
                write!(f, "目标目录不安全\n原因: {}\n目标目录: {}\n驱动根目录: {}\n缓存根目录: {}", 
                    reason, dest_dir, drivers_root, cache_root)
            }
            ExtractError::InvalidDriverUuid { driver_uuid, reason } => {
                write!(f, "驱动 UUID 格式无效: {}\n原因: {}\nUUID: {}", reason, driver_uuid, driver_uuid)
            }
            ExtractError::IoError { step, operation, error } => {
                write!(f, "IO 错误 (step={}, operation={}): {}", step, operation, error)
            }
        }
    }
}

impl std::error::Error for ExtractError {}

/// 规范化路径用于比较（去除 verbatim 前缀）
fn normalize_for_compare(p: &Path) -> Result<PathBuf, String> {
    let path_str = p.to_string_lossy();
    
    // 检查是否是 verbatim path (\\?\...)
    if path_str.starts_with(r"\\?\") {
        let without_prefix = &path_str[4..];
        
        // 处理 UNC 路径：\\?\UNC\server\share -> \\server\share
        if without_prefix.starts_with(r"UNC\") {
            let unc_path = format!(r"\\{}", &without_prefix[4..]);
            Ok(PathBuf::from(unc_path))
        } else {
            // 普通 verbatim 路径：\\?\C:\... -> C:\...
            Ok(PathBuf::from(without_prefix))
        }
    } else {
        // 已经是普通路径，直接返回
        Ok(p.to_path_buf())
    }
}

/// 校验目标目录是否安全
/// 
/// 规则：
/// 1. dest_dir 必须是 drivers_root 的子路径（canonical 后 starts_with）
/// 2. dest_dir 必须位于 drivers_root/.cache/ 下
/// 3. dest_dir 不能等于 drivers_root、不能等于 cache_root 本身（必须更深一层）
pub fn assert_safe_dest_dir(dest_dir: &Path, drivers_root: &Path) -> Result<(), ExtractError> {
    let cache_root = drivers_root.join(".cache");
    
    // 规范化路径
    let norm_dest = match dest_dir.canonicalize() {
        Ok(path) => normalize_for_compare(&path)
            .map_err(|e| ExtractError::IoError {
                step: "validate_dest",
                operation: "规范化目标目录",
                error: format!("无法规范化目标目录: {}", e),
            })?,
        Err(_) => {
            // 如果目录不存在，使用原始路径（但需要规范化）
            normalize_for_compare(dest_dir)
                .map_err(|e| ExtractError::IoError {
                    step: "validate_dest",
                    operation: "规范化目标目录",
                    error: format!("无法规范化目标目录: {}", e),
                })?
        }
    };
    
    let norm_drivers_root = match drivers_root.canonicalize() {
        Ok(path) => normalize_for_compare(&path)
            .map_err(|e| ExtractError::IoError {
                step: "validate_dest",
                operation: "规范化驱动根目录",
                error: format!("无法规范化驱动根目录: {}", e),
            })?,
        Err(_) => normalize_for_compare(drivers_root)
            .map_err(|e| ExtractError::IoError {
                step: "validate_dest",
                operation: "规范化驱动根目录",
                error: format!("无法规范化驱动根目录: {}", e),
            })?,
    };
    
    let norm_cache_root = match cache_root.canonicalize() {
        Ok(path) => normalize_for_compare(&path)
            .map_err(|e| ExtractError::IoError {
                step: "validate_dest",
                operation: "规范化缓存根目录",
                error: format!("无法规范化缓存根目录: {}", e),
            })?,
        Err(_) => normalize_for_compare(&cache_root)
            .map_err(|e| ExtractError::IoError {
                step: "validate_dest",
                operation: "规范化缓存根目录",
                error: format!("无法规范化缓存根目录: {}", e),
            })?,
    };
    
    // 规则 1: dest_dir 必须是 drivers_root 的子路径
    if !norm_dest.starts_with(&norm_drivers_root) {
        return Err(ExtractError::UnsafeDestination {
            dest_dir: dest_dir.display().to_string(),
            drivers_root: drivers_root.display().to_string(),
            cache_root: cache_root.display().to_string(),
            reason: "目标目录不在驱动根目录下".to_string(),
        });
    }
    
    // 规则 2: dest_dir 必须位于 drivers_root/.cache/ 下
    if !norm_dest.starts_with(&norm_cache_root) {
        return Err(ExtractError::UnsafeDestination {
            dest_dir: dest_dir.display().to_string(),
            drivers_root: drivers_root.display().to_string(),
            cache_root: cache_root.display().to_string(),
            reason: "目标目录不在缓存目录下（必须位于 drivers_root/.cache/ 下）".to_string(),
        });
    }
    
    // 规则 3: dest_dir 不能等于 drivers_root 或 cache_root 本身
    if norm_dest == norm_drivers_root {
        return Err(ExtractError::UnsafeDestination {
            dest_dir: dest_dir.display().to_string(),
            drivers_root: drivers_root.display().to_string(),
            cache_root: cache_root.display().to_string(),
            reason: "目标目录不能等于驱动根目录".to_string(),
        });
    }
    
    if norm_dest == norm_cache_root {
        return Err(ExtractError::UnsafeDestination {
            dest_dir: dest_dir.display().to_string(),
            drivers_root: drivers_root.display().to_string(),
            cache_root: cache_root.display().to_string(),
            reason: "目标目录不能等于缓存根目录（必须更深一层）".to_string(),
        });
    }
    
    Ok(())
}

/// 校验驱动 UUID 格式
/// 
/// 规则：^[a-zA-Z0-9_-]{8,64}$
/// - 只能包含字母、数字、下划线、连字符
/// - 长度 8-64 字符
/// - 不能包含路径分隔符（防止路径注入）
pub fn validate_driver_uuid(driver_uuid: &str) -> Result<(), ExtractError> {
    if driver_uuid.is_empty() {
        return Err(ExtractError::InvalidDriverUuid {
            driver_uuid: driver_uuid.to_string(),
            reason: "UUID 不能为空".to_string(),
        });
    }
    
    if driver_uuid.len() < 8 {
        return Err(ExtractError::InvalidDriverUuid {
            driver_uuid: driver_uuid.to_string(),
            reason: format!("UUID 长度不足（至少 8 字符，当前 {} 字符）", driver_uuid.len()),
        });
    }
    
    if driver_uuid.len() > 64 {
        return Err(ExtractError::InvalidDriverUuid {
            driver_uuid: driver_uuid.to_string(),
            reason: format!("UUID 长度过长（最多 64 字符，当前 {} 字符）", driver_uuid.len()),
        });
    }
    
    // 检查是否是危险的路径（"." 或 ".."）
    if driver_uuid == "." || driver_uuid == ".." {
        return Err(ExtractError::InvalidDriverUuid {
            driver_uuid: driver_uuid.to_string(),
            reason: "UUID 不能是 \".\" 或 \"..\"".to_string(),
        });
    }
    
    // 检查是否包含非法字符（路径分隔符等）
    if driver_uuid.contains('/') || driver_uuid.contains('\\') {
        return Err(ExtractError::InvalidDriverUuid {
            driver_uuid: driver_uuid.to_string(),
            reason: "UUID 不能包含路径分隔符（/ 或 \\）".to_string(),
        });
    }
    
    // 检查是否只包含允许的字符（字母、数字、下划线、连字符）
    if !driver_uuid.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(ExtractError::InvalidDriverUuid {
            driver_uuid: driver_uuid.to_string(),
            reason: "UUID 只能包含字母、数字、下划线和连字符".to_string(),
        });
    }
    
    Ok(())
}

/// 生成 UUID（简单实现，不依赖外部 crate）
fn generate_driver_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // 生成 16 字符的十六进制字符串（满足 8-64 字符要求）
    format!("{:016x}", timestamp)
}

/// 解压 ZIP 文件到指定目录（使用 staging 目录，安全解压）
/// 
/// # 参数
/// - `zip_path`: ZIP 文件路径
/// - `dest_dir`: 目标解压目录（必须位于 drivers_root/.cache/ 下）
/// - `drivers_root`: 驱动根目录
/// 
/// # 返回
/// - `Ok(ExtractResult)`: 解压成功
/// - `Err(ExtractError)`: 解压失败
/// 
/// # 步骤
/// 1. validate_dest: 校验目标目录安全性
/// 2. prepare_staging: 创建 staging 目录
/// 3. expand_archive: 使用 PowerShell Expand-Archive 解压到 staging
/// 4. zip_slip_check: 检测路径越界
/// 5. materialize_to_dest: 将 staging 内容复制到 dest_dir
/// 6. cleanup_staging: 清理 staging 目录
/// 
/// # 已弃用
/// 该函数已被新的 `extract_zip_for_driver` 函数（使用 Rust 原生 zip crate）替代。
/// 新函数提供更好的性能、错误处理和可观测性。
#[deprecated(
    since = "1.4.1",
    note = "使用 extract_zip_for_driver() 代替，它使用 Rust 原生 zip crate 而不依赖 PowerShell"
)]
pub fn extract_zip(zip_path: &Path, dest_dir: &Path, drivers_root: &Path) -> Result<ExtractResult, ExtractError> {
    eprintln!("[ExtractZip] DEPRECATED: 请使用 extract_zip_for_driver() 代替"); 
    eprintln!("[ExtractZip] start zip_path=\"{}\" dest_dir=\"{}\" drivers_root=\"{}\"", 
        zip_path.display(), dest_dir.display(), drivers_root.display());
    
    // ============================================================================
    // Step 1: validate_dest - 校验目标目录安全性
    // ============================================================================
    eprintln!("[ExtractZip] step=validate_dest inputs=dest_dir=\"{}\" drivers_root=\"{}\"", 
        dest_dir.display(), drivers_root.display());
    
    assert_safe_dest_dir(dest_dir, drivers_root)?;
    
    eprintln!("[ExtractZip] step=validate_dest result=passed dest_dir=\"{}\"", dest_dir.display());
    
    // ============================================================================
    // Step 2: prepare_staging - 创建 staging 目录
    // ============================================================================
    let staging_root = drivers_root.join(".cache").join("_extract_staging");
    let staging_uuid = generate_driver_uuid();
    let staging_dir = staging_root.join(&staging_uuid);
    
    eprintln!("[ExtractZip] step=prepare_staging inputs=staging_root=\"{}\" staging_uuid=\"{}\" staging_dir=\"{}\"", 
        staging_root.display(), staging_uuid, staging_dir.display());
    
    // 创建 staging_root（如果不存在）
    if let Err(e) = fs::create_dir_all(&staging_root) {
        return Err(ExtractError::IoError {
            step: "prepare_staging",
            operation: "创建 staging 根目录",
            error: format!("无法创建 staging 根目录 {}: {}", staging_root.display(), e),
        });
    }
    
    // 如果 staging_dir 已存在（理论上不应该），删除它
    if staging_dir.exists() {
        eprintln!("[ExtractZip] step=prepare_staging action=remove_existing_staging staging_dir=\"{}\"", staging_dir.display());
        if let Err(e) = fs::remove_dir_all(&staging_dir) {
            return Err(ExtractError::IoError {
                step: "prepare_staging",
                operation: "删除现有 staging 目录",
                error: format!("无法删除现有 staging 目录 {}: {}", staging_dir.display(), e),
            });
        }
    }
    
    // 创建 staging_dir
    if let Err(e) = fs::create_dir_all(&staging_dir) {
        return Err(ExtractError::IoError {
            step: "prepare_staging",
            operation: "创建 staging 目录",
            error: format!("无法创建 staging 目录 {}: {}", staging_dir.display(), e),
        });
    }
    
    eprintln!("[ExtractZip] step=prepare_staging result=created staging_dir=\"{}\"", staging_dir.display());
    
    // 确保在函数返回前清理 staging_dir（使用 defer 模式）
    struct StagingCleanup {
        staging_dir: PathBuf,
    }
    
    impl Drop for StagingCleanup {
        fn drop(&mut self) {
            if self.staging_dir.exists() {
                let _ = fs::remove_dir_all(&self.staging_dir);
            }
        }
    }
    
    let _cleanup = StagingCleanup {
        staging_dir: staging_dir.clone(),
    };
    
    // ============================================================================
    // Step 3: expand_archive - 使用 PowerShell Expand-Archive 解压到 staging
    // ============================================================================
    // 检查 ZIP 文件是否存在
    if !zip_path.exists() {
        return Err(ExtractError::ZipNotFound {
            zip_path: zip_path.display().to_string(),
        });
    }
    
    let zip_path_str = zip_path.to_string_lossy();
    let staging_dir_str = staging_dir.to_string_lossy();
    
    // 转义路径中的单引号（PowerShell 需要）
    let zip_path_escaped = zip_path_str.replace("'", "''");
    let staging_dir_escaped = staging_dir_str.replace("'", "''");
    
    let script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Expand-Archive -LiteralPath '{}' -DestinationPath '{}' -Force",
        zip_path_escaped, staging_dir_escaped
    );
    
    eprintln!("[ExtractZip] step=expand_archive inputs=zip_path=\"{}\" staging_dir=\"{}\"", 
        zip_path.display(), staging_dir.display());
    
    let output = match super::ps::run_powershell(&script) {
        Ok(output) => output,
        Err(e) => {
            return Err(ExtractError::ExtractFailed {
                step: "expand_archive",
                zip_path: zip_path_str.to_string(),
                dest_dir: staging_dir_str.to_string(),
                stdout: String::new(),
                stderr: e,
                exit_code: None,
            });
        }
    };
    
    let stdout = crate::platform::windows::encoding::decode_windows_string(&output.stdout);
    let stderr = crate::platform::windows::encoding::decode_windows_string(&output.stderr);
    let exit_code = output.status.code();
    
    if !output.status.success() {
        let evidence = format!(
            "step=expand_archive zip_path=\"{}\" staging_dir=\"{}\" exit_code={:?} stdout_len={} stderr_len={}",
            zip_path.display(), staging_dir.display(), exit_code, stdout.len(), stderr.len()
        );
        eprintln!("[ExtractZip] step=expand_archive result=failed evidence=\"{}\"", evidence);
        
        return Err(ExtractError::ExtractFailed {
            step: "expand_archive",
            zip_path: zip_path_str.to_string(),
            dest_dir: staging_dir_str.to_string(),
            stdout,
            stderr,
            exit_code,
        });
    }
    
    eprintln!("[ExtractZip] step=expand_archive result=success exit_code={:?} stdout_len={} stderr_len={}", 
        exit_code, stdout.len(), stderr.len());
    
    // ============================================================================
    // Step 4: zip_slip_check - 检测路径越界（对 staging_dir）
    // ============================================================================
    let canonical_staging_dir = match staging_dir.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            return Err(ExtractError::IoError {
                step: "zip_slip_check",
                operation: "规范化 staging 目录",
                error: format!("无法规范化 staging 目录 {}: {}", staging_dir.display(), e),
            });
        }
    };
    
    eprintln!("[ExtractZip] step=zip_slip_check inputs=staging_dir=\"{}\" canonical_staging=\"{}\"", 
        staging_dir.display(), canonical_staging_dir.display());
    
    // 递归遍历所有文件
    let mut file_count = 0;
    let mut dir_count = 0;
    let mut offending_path: Option<PathBuf> = None;
    
    fn walk_dir(
        dir: &Path,
        canonical_staging_dir: &Path,
        file_count: &mut usize,
        dir_count: &mut usize,
        offending_path: &mut Option<PathBuf>,
    ) -> Result<(), ExtractError> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                return Err(ExtractError::IoError {
                    step: "zip_slip_check",
                    operation: "读取目录",
                    error: format!("无法读取目录 {}: {}", dir.display(), e),
                });
            }
        };
        
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    return Err(ExtractError::IoError {
                        step: "zip_slip_check",
                        operation: "读取目录项",
                        error: format!("无法读取目录项: {}", e),
                    });
                }
            };
            
            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(meta) => meta,
                Err(e) => {
                    return Err(ExtractError::IoError {
                        step: "zip_slip_check",
                        operation: "获取文件元数据",
                        error: format!("无法获取文件元数据 {}: {}", path.display(), e),
                    });
                }
            };
            
            // 规范化路径并检查是否在 staging 目录内（必须在递归之前检查）
            let canonical_path = match path.canonicalize() {
                Ok(canonical) => canonical,
                Err(e) => {
                    return Err(ExtractError::IoError {
                        step: "zip_slip_check",
                        operation: "规范化文件路径",
                        error: format!("无法规范化文件路径 {}: {}", path.display(), e),
                    });
                }
            };
            
            if !canonical_path.starts_with(&canonical_staging_dir) {
                *offending_path = Some(path.clone());
                let evidence = format!(
                    "step=zip_slip_check offending_path=\"{}\" canonical_staging=\"{}\" canonical_offending=\"{}\"",
                    path.display(), canonical_staging_dir.display(), canonical_path.display()
                );
                eprintln!("[ExtractZip] step=zip_slip_check result=failed evidence=\"{}\"", evidence);
                
                return Err(ExtractError::ZipSlipDetected {
                    offending_path: path.display().to_string(),
                    canonical_dest_dir: canonical_staging_dir.display().to_string(),
                    canonical_offending_path: canonical_path.display().to_string(),
                });
            }
            
            if metadata.is_dir() {
                *dir_count += 1;
                // 递归检查子目录（只有在路径检查通过后才递归）
                walk_dir(&path, canonical_staging_dir, file_count, dir_count, offending_path)?;
            } else {
                *file_count += 1;
            }
        }
        
        Ok(())
    }
    
    match walk_dir(&staging_dir, &canonical_staging_dir, &mut file_count, &mut dir_count, &mut offending_path) {
        Ok(_) => {
            eprintln!("[ExtractZip] step=zip_slip_check result=passed files={} dirs={}", file_count, dir_count);
        }
        Err(e) => {
            return Err(e);
        }
    }
    
    // ============================================================================
    // Step 5: materialize_to_dest - 将 staging 内容复制到 dest_dir
    // ============================================================================
    eprintln!("[ExtractZip] step=materialize_to_dest inputs=staging_dir=\"{}\" dest_dir=\"{}\"", 
        staging_dir.display(), dest_dir.display());
    
    // 创建 dest_dir（如果不存在）
    if let Err(e) = fs::create_dir_all(dest_dir) {
        return Err(ExtractError::IoError {
            step: "materialize_to_dest",
            operation: "创建目标目录",
            error: format!("无法创建目标目录 {}: {}", dest_dir.display(), e),
        });
    }
    
    let mut copied_files = 0;
    
    fn copy_tree(
        src: &Path,
        dest: &Path,
        staging_base: &Path,
        dest_base: &Path,
        copied_files: &mut usize,
    ) -> Result<(), ExtractError> {
        let entries = match fs::read_dir(src) {
            Ok(entries) => entries,
            Err(e) => {
                return Err(ExtractError::IoError {
                    step: "materialize_to_dest",
                    operation: "读取 staging 目录",
                    error: format!("无法读取 staging 目录 {}: {}", src.display(), e),
                });
            }
        };
        
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    return Err(ExtractError::IoError {
                        step: "materialize_to_dest",
                        operation: "读取目录项",
                        error: format!("无法读取目录项: {}", e),
                    });
                }
            };
            
            let src_path = entry.path();
            let relative_path = src_path.strip_prefix(staging_base)
                .map_err(|e| ExtractError::IoError {
                    step: "materialize_to_dest",
                    operation: "计算相对路径",
                    error: format!("无法计算相对路径: {}", e),
                })?;
            
            let dest_path = dest_base.join(relative_path);
            
            let metadata = match entry.metadata() {
                Ok(meta) => meta,
                Err(e) => {
                    return Err(ExtractError::IoError {
                        step: "materialize_to_dest",
                        operation: "获取文件元数据",
                        error: format!("无法获取文件元数据 {}: {}", src_path.display(), e),
                    });
                }
            };
            
            if metadata.is_dir() {
                // 创建目标目录
                if let Err(e) = fs::create_dir_all(&dest_path) {
                    return Err(ExtractError::IoError {
                        step: "materialize_to_dest",
                        operation: "创建目标目录",
                        error: format!("无法创建目标目录 {}: {}", dest_path.display(), e),
                    });
                }
                // 递归复制子目录
                copy_tree(&src_path, &dest_path, staging_base, dest_base, copied_files)?;
            } else {
                // 复制文件（允许覆盖）
                if let Err(e) = fs::copy(&src_path, &dest_path) {
                    return Err(ExtractError::IoError {
                        step: "materialize_to_dest",
                        operation: "复制文件",
                        error: format!("无法复制文件 {} -> {}: {}", src_path.display(), dest_path.display(), e),
                    });
                }
                *copied_files += 1;
            }
        }
        
        Ok(())
    }
    
    match copy_tree(&staging_dir, dest_dir, &staging_dir, dest_dir, &mut copied_files) {
        Ok(_) => {
            eprintln!("[ExtractZip] step=materialize_to_dest result=success copied_files={}", copied_files);
        }
        Err(e) => {
            return Err(e);
        }
    }
    
    // ============================================================================
    // Step 6: cleanup_staging - 清理 staging 目录
    // ============================================================================
    eprintln!("[ExtractZip] step=cleanup_staging inputs=staging_dir=\"{}\"", staging_dir.display());
    
    // 注意：StagingCleanup 会在函数返回时自动清理，但我们可以显式清理
    if staging_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&staging_dir) {
            eprintln!("[ExtractZip] step=cleanup_staging warning=failed error=\"{}\"", e);
            // 不返回错误，因为解压已经成功
        } else {
            eprintln!("[ExtractZip] step=cleanup_staging result=removed staging_dir=\"{}\"", staging_dir.display());
        }
    }
    
    // ============================================================================
    // Step 7: summary - 统计并输出摘要
    // ============================================================================
    let mut top_level_items = Vec::new();
    let entries = match fs::read_dir(dest_dir) {
        Ok(entries) => entries,
        Err(e) => {
            return Err(ExtractError::IoError {
                step: "summary",
                operation: "读取目录",
                error: format!("无法读取目录 {}: {}", dest_dir.display(), e),
            });
        }
    };
    
    for entry in entries {
        match entry {
            Ok(entry) => {
                let name = entry.file_name().to_string_lossy().to_string();
                top_level_items.push(name);
            }
            Err(e) => {
                eprintln!("[ExtractZip] step=summary warning=read_entry_failed error=\"{}\"", e);
            }
        }
    }
    
    // 限制顶层项目数量（最多显示 10 个）
    let top_level_preview: Vec<String> = top_level_items.iter().take(10).cloned().collect();
    let top_level_display = if top_level_items.len() > 10 {
        format!("[{} items: {}...]", top_level_items.len(), top_level_preview.join(", "))
    } else {
        format!("[{} items: {}]", top_level_items.len(), top_level_preview.join(", "))
    };
    
    let evidence = format!(
        "step=summary files={} dirs={} copied_files={} top_level={} dest_dir=\"{}\" staging_dir=\"{}\"",
        file_count, dir_count, copied_files, top_level_display, dest_dir.display(), staging_dir.display()
    );
    
    eprintln!("[ExtractZip] step=summary result=success evidence=\"{}\"", evidence);
    
    Ok(ExtractResult {
        extracted_files: file_count,
        extracted_dirs: dir_count,
        evidence,
        dest_dir: dest_dir.to_path_buf(),
        top_level_items,
    })
}

/// 解压 ZIP 文件到指定驱动目录（固定落点，禁止自定义目标目录）
/// 
/// # 参数
/// - `zip_path`: ZIP 文件路径
/// - `drivers_root`: 驱动根目录（AppDir/drivers）
/// - `driver_uuid`: 驱动 UUID（必须满足格式：^[a-zA-Z0-9_-]{8,64}$）
/// 
/// # 返回
/// - `Ok(ExtractForDriverResult)`: 解压成功
/// - `Err(ExtractError)`: 解压失败
/// 
/// # 目录结构
/// - uuid_root = drivers_root.join(driver_uuid)
/// - staging_root = uuid_root.join("_staging")
/// - staging_dir = staging_root.join(<run_id_uuid>)
/// - extracted_root = uuid_root.join("extracted")
/// 
/// # 步骤
/// 1. compute_roots: 计算所有根目录
/// 2. prepare_dirs: 创建必要的目录
/// 3. expand_archive: 解压到 staging
/// 4. zip_slip_check: 检测路径越界（相对于 uuid_root）
/// 5. materialize: 将 staging 内容复制到 extracted_root
/// 6. cleanup_staging: 清理 staging 目录
/// 7. summary: 统计并输出摘要
pub fn extract_zip_for_driver(
    zip_path: &Path,
    drivers_root: &Path,
    driver_uuid: &str,
    app: Option<&tauri::AppHandle>,
    printer_name: Option<&str>,
    job_id: &str,  // 安装任务 ID
) -> Result<ExtractForDriverResult, ExtractError> {
    eprintln!("[ExtractZipForDriver] start zip_path=\"{}\" drivers_root=\"{}\" driver_uuid=\"{}\"", 
        zip_path.display(), drivers_root.display(), driver_uuid);
    
    // ============================================================================
    // Step 0: validate_driver_uuid - 校验驱动 UUID 格式
    // ============================================================================
    eprintln!("[ExtractZipForDriver] step=validate_driver_uuid inputs=driver_uuid=\"{}\"", driver_uuid);
    
    validate_driver_uuid(driver_uuid)?;
    
    eprintln!("[ExtractZipForDriver] step=validate_driver_uuid result=passed driver_uuid=\"{}\"", driver_uuid);
    
    // ============================================================================
    // Step 1: compute_roots - 计算所有根目录
    // ============================================================================
    let uuid_root = drivers_root.join(driver_uuid);
    let staging_root = uuid_root.join("_staging");
    let run_id = generate_driver_uuid();
    let staging_dir = staging_root.join(&run_id);
    let extracted_root = uuid_root.join("extracted");
    
    eprintln!("[ExtractZipForDriver] step=compute_roots inputs=drivers_root=\"{}\" driver_uuid=\"{}\" run_id=\"{}\"", 
        drivers_root.display(), driver_uuid, run_id);
    eprintln!("[ExtractZipForDriver] step=compute_roots outputs=uuid_root=\"{}\" staging_root=\"{}\" staging_dir=\"{}\" extracted_root=\"{}\"", 
        uuid_root.display(), staging_root.display(), staging_dir.display(), extracted_root.display());
    
    // ============================================================================
    // Step 2: prepare_dirs - 创建必要的目录
    // ============================================================================
    eprintln!("[ExtractZipForDriver] step=prepare_dirs inputs=staging_dir=\"{}\" extracted_root=\"{}\"", 
        staging_dir.display(), extracted_root.display());
    
    // 创建 staging_root（如果不存在）
    if let Err(e) = fs::create_dir_all(&staging_root) {
        return Err(ExtractError::IoError {
            step: "prepare_dirs",
            operation: "创建 staging 根目录",
            error: format!("无法创建 staging 根目录 {}: {}", staging_root.display(), e),
        });
    }
    
    // 如果 staging_dir 已存在，删除它（只删除 staging_dir 自身，不删除 uuid_root）
    if staging_dir.exists() {
        eprintln!("[ExtractZipForDriver] step=prepare_dirs action=remove_existing_staging staging_dir=\"{}\"", staging_dir.display());
        if let Err(e) = fs::remove_dir_all(&staging_dir) {
            return Err(ExtractError::IoError {
                step: "prepare_dirs",
                operation: "删除现有 staging 目录",
                error: format!("无法删除现有 staging 目录 {}: {}", staging_dir.display(), e),
            });
        }
    }
    
    // 创建 staging_dir
    if let Err(e) = fs::create_dir_all(&staging_dir) {
        return Err(ExtractError::IoError {
            step: "prepare_dirs",
            operation: "创建 staging 目录",
            error: format!("无法创建 staging 目录 {}: {}", staging_dir.display(), e),
        });
    }
    
    // 创建 extracted_root（如果不存在，但不删除现有内容）
    if let Err(e) = fs::create_dir_all(&extracted_root) {
        return Err(ExtractError::IoError {
            step: "prepare_dirs",
            operation: "创建 extracted 根目录",
            error: format!("无法创建 extracted 根目录 {}: {}", extracted_root.display(), e),
        });
    }
    
    eprintln!("[ExtractZipForDriver] step=prepare_dirs result=created staging_dir=\"{}\" extracted_root=\"{}\"", 
        staging_dir.display(), extracted_root.display());
    
    // 确保在函数返回前清理 staging_dir（使用 Drop trait）
    // 支持环境变量 EPRINTY_KEEP_STAGING 来保留 staging 以便排查
    struct StagingCleanup {
        staging_dir: PathBuf,
        should_cleanup: bool,
    }
    
    impl Drop for StagingCleanup {
        fn drop(&mut self) {
            if self.staging_dir.exists() {
                if self.should_cleanup {
                    if let Err(e) = std::fs::remove_dir_all(&self.staging_dir) {
                        eprintln!("[ExtractZipForDriver] cleanup_failed staging_dir=\"{}\" error=\"{}\"", 
                            self.staging_dir.display(), e);
                    } else {
                        eprintln!("[ExtractZipForDriver] cleanup_success staging_dir=\"{}\"", 
                            self.staging_dir.display());
                    }
                } else {
                    eprintln!("[ExtractZipForDriver] cleanup_skipped staging_dir=\"{}\" reason=\"EPRINTY_KEEP_STAGING 已设置\"", 
                        self.staging_dir.display());
                }
            }
        }
    }
    
    // 检查环境变量决定是否清理 staging
    let keep_staging = std::env::var("EPRINTY_KEEP_STAGING").is_ok();
    let should_cleanup = !keep_staging;
    
    let _cleanup = StagingCleanup {
        staging_dir: staging_dir.clone(),
        should_cleanup,
    };
    
    // ============================================================================
    // Step 3: expand_archive - 解压到 staging（使用 Rust 原生 zip crate）
    // ============================================================================
    // 检查 ZIP 文件是否存在
    if !zip_path.exists() {
        return Err(ExtractError::ZipNotFound {
            zip_path: zip_path.display().to_string(),
        });
    }
    
    let zip_path_str = zip_path.to_string_lossy().to_string();
    let staging_dir_str = staging_dir.to_string_lossy().to_string();
    
    eprintln!("[ExtractZipForDriver] step=expand_archive inputs=zip_path=\"{}\" staging_dir=\"{}\"", 
        zip_path.display(), staging_dir.display());
    
    // 创建 StepReporter（用于整个解压流程）
    let mut step_reporter_opt: Option<crate::platform::windows::step_reporter::StepReporter> = if let (Some(app_handle), Some(printer)) = (app, printer_name) {
        Some(crate::platform::windows::step_reporter::StepReporter::start(
            std::sync::Arc::new(app_handle.clone()),
            job_id.to_string(),
            printer.to_string(),
            "driver.extract".to_string(),
            "正在解压驱动包".to_string(),
        ))
    } else {
        None
    };
    
    // 使用 Rust 原生 zip 解压器
    let extract_result = match crate::utils::zip_extract::extract_zip_to_dir(
        zip_path,
        &staging_dir,
        None, // cancel flag (暂不支持)
        None, // progress callback (暂不集成，后续可加)
    ) {
        Ok(report) => report,
        Err(e) => {
            let error_msg = format!("{}", e);
            let staging_hint = if should_cleanup {
                format!("（staging 目录将在函数返回时自动清理）")
            } else {
                format!("（staging 目录已保留供排查：{}）", staging_dir.display())
            };
            eprintln!("[ExtractZipForDriver] step=expand_archive result=failed error=\"{}\" {}", error_msg, staging_hint);
            
            // 发送失败事件
            if let Some(reporter) = step_reporter_opt.take() {
                let _ = reporter.failed(
                    "EXTRACT_FAILED".to_string(),
                    format!("解压失败: {} | ZIP: {} | Dest: {} | {}", error_msg, zip_path.display(), staging_dir.display(), staging_hint),
                    None,
                    Some(error_msg.clone()),
                    None,
                );
            }
            
            return Err(ExtractError::ExtractFailed {
                step: "expand_archive",
                zip_path: zip_path_str,
                dest_dir: staging_dir_str,
                stdout: String::new(),
                stderr: error_msg,
                exit_code: None,
            });
        }
    };
    
    eprintln!(
        "[ExtractZipForDriver] step=expand_archive result=success files_extracted={} dirs_created={} bytes_written={} elapsed_ms={}",
        extract_result.files_extracted,
        extract_result.directories_created,
        extract_result.bytes_written,
        extract_result.elapsed_ms
    );
    
    // ============================================================================
    // Step 4: zip_slip_check - 检测路径越界（相对于 uuid_root）
    // ============================================================================
    let canonical_uuid_root = match uuid_root.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            return Err(ExtractError::IoError {
                step: "zip_slip_check",
                operation: "规范化 uuid 根目录",
                error: format!("无法规范化 uuid 根目录 {}: {}", uuid_root.display(), e),
            });
        }
    };
    
    eprintln!("[ExtractZipForDriver] step=zip_slip_check inputs=uuid_root=\"{}\" canonical_uuid_root=\"{}\" staging_dir=\"{}\"", 
        uuid_root.display(), canonical_uuid_root.display(), staging_dir.display());
    
    // 递归遍历所有文件
    let mut file_count = 0;
    let mut dir_count = 0;
    let mut offending_path: Option<PathBuf> = None;
    
    fn walk_dir_for_uuid(
        dir: &Path,
        canonical_uuid_root: &Path,
        file_count: &mut usize,
        dir_count: &mut usize,
        offending_path: &mut Option<PathBuf>,
    ) -> Result<(), ExtractError> {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(e) => {
                return Err(ExtractError::IoError {
                    step: "zip_slip_check",
                    operation: "读取目录",
                    error: format!("无法读取目录 {}: {}", dir.display(), e),
                });
            }
        };
        
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    return Err(ExtractError::IoError {
                        step: "zip_slip_check",
                        operation: "读取目录项",
                        error: format!("无法读取目录项: {}", e),
                    });
                }
            };
            
            let path = entry.path();
            let metadata = match entry.metadata() {
                Ok(meta) => meta,
                Err(e) => {
                    return Err(ExtractError::IoError {
                        step: "zip_slip_check",
                        operation: "获取文件元数据",
                        error: format!("无法获取文件元数据 {}: {}", path.display(), e),
                    });
                }
            };
            
            // 规范化路径并检查是否在 uuid_root 内（必须在递归之前检查）
            let canonical_path = match path.canonicalize() {
                Ok(canonical) => canonical,
                Err(e) => {
                    return Err(ExtractError::IoError {
                        step: "zip_slip_check",
                        operation: "规范化文件路径",
                        error: format!("无法规范化文件路径 {}: {}", path.display(), e),
                    });
                }
            };
            
            if !canonical_path.starts_with(canonical_uuid_root) {
                *offending_path = Some(path.clone());
                let evidence = format!(
                    "step=zip_slip_check offending_path=\"{}\" canonical_uuid_root=\"{}\" canonical_offending=\"{}\"",
                    path.display(), canonical_uuid_root.display(), canonical_path.display()
                );
                eprintln!("[ExtractZipForDriver] step=zip_slip_check result=failed evidence=\"{}\"", evidence);
                
                return Err(ExtractError::ZipSlipDetected {
                    offending_path: path.display().to_string(),
                    canonical_dest_dir: canonical_uuid_root.display().to_string(),
                    canonical_offending_path: canonical_path.display().to_string(),
                });
            }
            
            if metadata.is_dir() {
                *dir_count += 1;
                // 递归检查子目录（只有在路径检查通过后才递归）
                walk_dir_for_uuid(&path, canonical_uuid_root, file_count, dir_count, offending_path)?;
            } else {
                *file_count += 1;
            }
        }
        
        Ok(())
    }
    
    match walk_dir_for_uuid(&staging_dir, &canonical_uuid_root, &mut file_count, &mut dir_count, &mut offending_path) {
        Ok(_) => {
            eprintln!("[ExtractZipForDriver] step=zip_slip_check result=passed files={} dirs={}", file_count, dir_count);
            
            // 发送 zip-slip 检查通过事件
            // 更新进度：zip-slip 检查通过
            if let Some(ref mut reporter) = step_reporter_opt {
                reporter.update_progress(
                    Some(file_count as u64),
                    Some(file_count as u64),
                    Some("files".to_string()),
                    Some(100.0),
                    Some("安全检查通过".to_string()),
                );
            }
        }
        Err(e) => {
            // 发送 zip-slip 检查失败事件
            if let Some(reporter) = step_reporter_opt.take() {
                let _ = reporter.failed(
                    "ZIP_SLIP_DETECTED".to_string(),
                    "检测到路径越界攻击".to_string(),
                    None,
                    None,
                    None,
                );
            }
            return Err(e);
        }
    }
    
    // ============================================================================
    // Step 5: materialize - 将 staging 内容复制到 extracted_root
    // ============================================================================
    eprintln!("[ExtractZipForDriver] step=materialize inputs=staging_dir=\"{}\" extracted_root=\"{}\"", 
        staging_dir.display(), extracted_root.display());
    
    // 发送 materialize 开始事件
    // 更新进度：开始合并文件
    if let Some(ref mut reporter) = step_reporter_opt {
        reporter.update_progress(
            Some(0),
            Some(file_count as u64),
            Some("files".to_string()),
            Some(0.0),
            Some("正在合并文件".to_string()),
        );
    }
    
    let mut copied_files = 0;
    
    fn copy_tree_to_extracted(
        src: &Path,
        dest: &Path,
        staging_base: &Path,
        extracted_base: &Path,
        copied_files: &mut usize,
    ) -> Result<(), ExtractError> {
        let entries = match fs::read_dir(src) {
            Ok(entries) => entries,
            Err(e) => {
                return Err(ExtractError::IoError {
                    step: "materialize",
                    operation: "读取 staging 目录",
                    error: format!("无法读取 staging 目录 {}: {}", src.display(), e),
                });
            }
        };
        
        for entry in entries {
            let entry = match entry {
                Ok(entry) => entry,
                Err(e) => {
                    return Err(ExtractError::IoError {
                        step: "materialize",
                        operation: "读取目录项",
                        error: format!("无法读取目录项: {}", e),
                    });
                }
            };
            
            let src_path = entry.path();
            let relative_path = src_path.strip_prefix(staging_base)
                .map_err(|e| ExtractError::IoError {
                    step: "materialize",
                    operation: "计算相对路径",
                    error: format!("无法计算相对路径: {}", e),
                })?;
            
            let dest_path = extracted_base.join(relative_path);
            
            let metadata = match entry.metadata() {
                Ok(meta) => meta,
                Err(e) => {
                    return Err(ExtractError::IoError {
                        step: "materialize",
                        operation: "获取文件元数据",
                        error: format!("无法获取文件元数据 {}: {}", src_path.display(), e),
                    });
                }
            };
            
            if metadata.is_dir() {
                // 创建目标目录
                if let Err(e) = fs::create_dir_all(&dest_path) {
                    return Err(ExtractError::IoError {
                        step: "materialize",
                        operation: "创建目标目录",
                        error: format!("无法创建目标目录 {}: {}", dest_path.display(), e),
                    });
                }
                // 递归复制子目录
                copy_tree_to_extracted(&src_path, &dest_path, staging_base, extracted_base, copied_files)?;
            } else {
                // 复制文件（允许覆盖）
                if let Err(e) = fs::copy(&src_path, &dest_path) {
                    return Err(ExtractError::IoError {
                        step: "materialize",
                        operation: "复制文件",
                        error: format!("无法复制文件 {} -> {}: {}", src_path.display(), dest_path.display(), e),
                    });
                }
                *copied_files += 1;
            }
        }
        
        Ok(())
    }
    
    match copy_tree_to_extracted(&staging_dir, &extracted_root, &staging_dir, &extracted_root, &mut copied_files) {
        Ok(_) => {
            eprintln!("[ExtractZipForDriver] step=materialize result=success copied_files={}", copied_files);
            
            // 发送解压成功事件
            if let Some(reporter) = step_reporter_opt.take() {
                let meta = serde_json::json!({
                    "copied_files": copied_files,
                    "file_count": file_count,
                });
                let _ = reporter.success(
                    format!("解压完成：已合并 {} 个文件", copied_files),
                    Some(meta),
                );
            }
        }
        Err(e) => {
            // 发送 materialize 失败事件
            if let Some(reporter) = step_reporter_opt.take() {
                let _ = reporter.failed(
                    "MATERIALIZE_FAILED".to_string(),
                    format!("合并文件失败: {}", e),
                    None,
                    None,
                    None,
                );
            }
            return Err(e);
        }
    }
    
    // ============================================================================
    // Step 6: cleanup_staging - 清理 staging 目录
    // ============================================================================
    eprintln!("[ExtractZipForDriver] step=cleanup_staging inputs=staging_dir=\"{}\"", staging_dir.display());
    
    // 注意：StagingCleanup 会在函数返回时自动清理，但我们可以显式清理
    if staging_dir.exists() {
        if let Err(e) = fs::remove_dir_all(&staging_dir) {
            eprintln!("[ExtractZipForDriver] step=cleanup_staging warning=failed error=\"{}\"", e);
            // 不返回错误，因为解压已经成功
        } else {
            eprintln!("[ExtractZipForDriver] step=cleanup_staging result=removed staging_dir=\"{}\"", staging_dir.display());
        }
    }
    
    // ============================================================================
    // Step 7: summary - 统计并输出摘要
    // ============================================================================
    let mut top_entries = Vec::new();
    let entries = match fs::read_dir(&extracted_root) {
        Ok(entries) => entries,
        Err(e) => {
            return Err(ExtractError::IoError {
                step: "summary",
                operation: "读取 extracted 目录",
                error: format!("无法读取 extracted 目录 {}: {}", extracted_root.display(), e),
            });
        }
    };
    
    for entry in entries {
        match entry {
            Ok(entry) => {
                let name = entry.file_name().to_string_lossy().to_string();
                top_entries.push(name);
            }
            Err(e) => {
                eprintln!("[ExtractZipForDriver] step=summary warning=read_entry_failed error=\"{}\"", e);
            }
        }
    }
    
    // 限制顶层项目数量（最多显示 10 个）
    let top_entries_preview: Vec<String> = top_entries.iter().take(10).cloned().collect();
    let top_entries_display = if top_entries.len() > 10 {
        format!("[{} items: {}...]", top_entries.len(), top_entries_preview.join(", "))
    } else {
        format!("[{} items: {}]", top_entries.len(), top_entries_preview.join(", "))
    };
    
    let evidence = format!(
        "step=summary driver_uuid=\"{}\" uuid_root=\"{}\" extracted_root=\"{}\" files={} dirs={} copied_files={} top_entries={}",
        driver_uuid, uuid_root.display(), extracted_root.display(), file_count, dir_count, copied_files, top_entries_display
    );
    
    eprintln!("[ExtractZipForDriver] step=summary result=success evidence=\"{}\"", evidence);
    
    Ok(ExtractForDriverResult {
        driver_uuid: driver_uuid.to_string(),
        uuid_root,
        extracted_root,
        file_count,
        top_entries,
    })
}
