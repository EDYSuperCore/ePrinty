/// Rust 原生 ZIP 解压工具模块
///
/// 用于替代 PowerShell Expand-Archive，提供：
/// - 防 Zip Slip 的路径验证
/// - 可观测性（日志记录、进度回调）
/// - 取消支持（AtomicBool）
/// - 详细的错误信息

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf, Component};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// 解压结果信息
#[derive(Debug, Clone)]
pub struct ExtractReport {
    pub files_extracted: usize,
    pub directories_created: usize,
    pub bytes_written: u64,
    pub elapsed_ms: u128,
}

/// 解压错误类型
#[derive(Debug)]
pub enum ExtractError {
    /// ZIP 文件不存在或无法打开
    ZipOpenFailed {
        path: String,
        reason: String,
    },
    /// ZIP 格式错误或损坏
    ZipFormatError {
        reason: String,
    },
    /// 检测到 Zip Slip 攻击（路径逃逸）
    ZipSlipDetected {
        entry_name: String,
        resolved_path: String,
    },
    /// I/O 错误
    IoError {
        operation: String,
        path: String,
        reason: String,
    },
    /// 权限错误
    PermissionDenied {
        path: String,
        reason: String,
    },
    /// 用户取消操作
    Cancelled,
    /// 其他错误
    Other(String),
}

impl std::fmt::Display for ExtractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExtractError::ZipOpenFailed { path, reason } => {
                write!(f, "无法打开 ZIP 文件 '{}': {}", path, reason)
            }
            ExtractError::ZipFormatError { reason } => {
                write!(f, "ZIP 格式错误或损坏: {}", reason)
            }
            ExtractError::ZipSlipDetected { entry_name, resolved_path } => {
                write!(f, "检测到路径逃逸攻击 | Entry: '{}' | Resolved: '{}' | 该条目将突破目标目录范围", entry_name, resolved_path)
            }
            ExtractError::IoError { operation, path, reason } => {
                write!(f, "I/O 错误 | 操作: {} | 路径: '{}' | 原因: {}", operation, path, reason)
            }
            ExtractError::PermissionDenied { path, reason } => {
                write!(f, "权限被拒绝 | 路径: '{}' | 原因: {}", path, reason)
            }
            ExtractError::Cancelled => {
                write!(f, "用户取消解压操作")
            }
            ExtractError::Other(msg) => {
                write!(f, "其他错误: {}", msg)
            }
        }
    }
}

impl std::error::Error for ExtractError {}

/// 检查路径是否尝试逃逸出 dest_dir（防 Zip Slip）
///
/// # 参数
/// - `entry_path`: ZIP 中的路径（可能包含 `..` 或绝对路径）
/// - `dest_dir`: 目标目录
///
/// # 返回
/// - `Ok(resolved_path)`: 安全的解析后路径
/// - `Err(ExtractError)`: 检测到路径逃逸
fn validate_entry_path(entry_path: &str, dest_dir: &Path) -> Result<PathBuf, ExtractError> {
    // 将 ZIP 路径转换为 PathBuf
    let entry_relative = PathBuf::from(entry_path);
    
    // 检查是否为绝对路径或包含 ".."
    for component in entry_relative.components() {
        match component {
            Component::ParentDir => {
                return Err(ExtractError::ZipSlipDetected {
                    entry_name: entry_path.to_string(),
                    resolved_path: format!("路径包含 '..' 组件"),
                });
            }
            Component::Prefix(_) | Component::RootDir => {
                // Windows 驱动器前缀或 Unix root
                return Err(ExtractError::ZipSlipDetected {
                    entry_name: entry_path.to_string(),
                    resolved_path: format!("路径为绝对路径"),
                });
            }
            _ => {}
        }
    }
    
    // 构造最终路径
    let resolved = dest_dir.join(&entry_relative);
    
    // 验证解析后的路径是否仍在 dest_dir 范围内
    match resolved.canonicalize() {
        Ok(canonical) => {
            match dest_dir.canonicalize() {
                Ok(dest_canonical) => {
                    if !canonical.starts_with(&dest_canonical) {
                        return Err(ExtractError::ZipSlipDetected {
                            entry_name: entry_path.to_string(),
                            resolved_path: canonical.display().to_string(),
                        });
                    }
                }
                Err(_) => {
                    // dest_dir 可能尚不存在，仅做相对路径检查
                    // 确保没有 .. 逃逸
                }
            }
        }
        Err(_) => {
            // 路径尚不存在，进行相对路径检查
            // 已在上面的 component 检查中验证
        }
    }
    
    Ok(resolved)
}

/// 提取 ZIP 到指定目录
///
/// # 参数
/// - `zip_path`: ZIP 文件路径
/// - `dest_dir`: 目标目录（不需提前存在，会自动创建）
/// - `cancel`: 可选的取消标志（原子操作）
/// - `progress_cb`: 可选的进度回调：`fn(done: usize, total: usize)`
///
/// # 返回
/// - `Ok(ExtractReport)`: 解压成功的统计信息
/// - `Err(ExtractError)`: 解压失败的详细错误
///
/// # 例子
/// ```ignore
/// let cancel = Arc::new(AtomicBool::new(false));
/// let report = extract_zip_to_dir(
///     Path::new("payload.zip"),
///     Path::new("target/extracted"),
///     Some(&cancel),
///     None,
/// )?;
/// println!("Extracted {} files in {}ms", report.files_extracted, report.elapsed_ms);
/// ```
pub fn extract_zip_to_dir(
    zip_path: &Path,
    dest_dir: &Path,
    cancel: Option<&AtomicBool>,
    progress_cb: Option<&dyn Fn(usize, usize)>,
) -> Result<ExtractReport, ExtractError> {
    let start_time = Instant::now();
    
    eprintln!("[ZipExtract] step=start zip_path=\"{}\" dest_dir=\"{}\"", 
        zip_path.display(), dest_dir.display());
    
    // 检查取消标志
    if let Some(cancel_flag) = cancel {
        if cancel_flag.load(Ordering::Relaxed) {
            return Err(ExtractError::Cancelled);
        }
    }
    
    // 验证 ZIP 文件存在
    if !zip_path.exists() {
        eprintln!("[ZipExtract] error=file_not_found zip_path=\"{}\"", zip_path.display());
        return Err(ExtractError::ZipOpenFailed {
            path: zip_path.display().to_string(),
            reason: "文件不存在".to_string(),
        });
    }
    
    // 创建目标目录
    fs::create_dir_all(dest_dir).map_err(|e| {
        let reason = if e.kind() == io::ErrorKind::PermissionDenied {
            "权限被拒绝".to_string()
        } else {
            e.to_string()
        };
        eprintln!("[ZipExtract] error=create_dest_dir dest_dir=\"{}\" reason=\"{}\"", 
            dest_dir.display(), reason);
        ExtractError::IoError {
            operation: "创建目标目录".to_string(),
            path: dest_dir.display().to_string(),
            reason,
        }
    })?;
    
    // 打开 ZIP 文件
    let file = fs::File::open(zip_path).map_err(|e| {
        let reason = match e.kind() {
            io::ErrorKind::NotFound => "文件不存在".to_string(),
            io::ErrorKind::PermissionDenied => "权限被拒绝".to_string(),
            _ => e.to_string(),
        };
        eprintln!("[ZipExtract] error=open_zip_file zip_path=\"{}\" reason=\"{}\"", 
            zip_path.display(), reason);
        ExtractError::ZipOpenFailed {
            path: zip_path.display().to_string(),
            reason,
        }
    })?;
    
    // 使用 zip crate 读取 ZIP
    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        ExtractError::ZipFormatError {
            reason: format!("无法解析 ZIP: {}", e),
        }
    })?;
    
    let total_entries = archive.len();
    let mut files_extracted = 0;
    let mut directories_created = 0;
    let mut bytes_written = 0u64;
    
    // 遍历 ZIP 条目
    for i in 0..archive.len() {
        // 检查取消标志
        if let Some(cancel_flag) = cancel {
            if cancel_flag.load(Ordering::Relaxed) {
                return Err(ExtractError::Cancelled);
            }
        }
        
        // 触发进度回调
        if let Some(cb) = progress_cb {
            cb(i, total_entries);
        }
        
        let mut entry = archive.by_index(i).map_err(|e| {
            ExtractError::ZipFormatError {
                reason: format!("无法读取 ZIP 条目 #{}: {}", i, e),
            }
        })?;
        
        let entry_name = entry.name().to_string();
        
        // 验证路径安全性（防 Zip Slip）
        let target_path = validate_entry_path(&entry_name, dest_dir)?;
        
        if entry.is_dir() {
            // 创建目录
            fs::create_dir_all(&target_path).map_err(|e| {
                let reason = if e.kind() == io::ErrorKind::PermissionDenied {
                    "权限被拒绝".to_string()
                } else {
                    e.to_string()
                };
                ExtractError::IoError {
                    operation: "创建目录".to_string(),
                    path: target_path.display().to_string(),
                    reason,
                }
            })?;
            directories_created += 1;
        } else {
            // 确保父目录存在
            if let Some(parent) = target_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent).map_err(|e| {
                        let reason = if e.kind() == io::ErrorKind::PermissionDenied {
                            "权限被拒绝".to_string()
                        } else {
                            e.to_string()
                        };
                        ExtractError::IoError {
                            operation: "创建父目录".to_string(),
                            path: parent.display().to_string(),
                            reason,
                        }
                    })?;
                }
            }
            
            // 创建文件并写入内容
            let mut out_file = fs::File::create(&target_path).map_err(|e| {
                let reason = if e.kind() == io::ErrorKind::PermissionDenied {
                    "权限被拒绝".to_string()
                } else {
                    e.to_string()
                };
                ExtractError::IoError {
                    operation: "创建文件".to_string(),
                    path: target_path.display().to_string(),
                    reason,
                }
            })?;
            
            // 使用 buffered copy，大文件用 2MB buffer
            let buffer_size = if entry.size() > 10 * 1024 * 1024 {
                2 * 1024 * 1024 // 2MB for large files
            } else {
                64 * 1024 // 64KB for normal files
            };
            
            let mut buffer = vec![0u8; buffer_size];
            loop {
                let bytes_read = entry.read(&mut buffer).map_err(|e| {
                    ExtractError::IoError {
                        operation: "读取 ZIP 条目".to_string(),
                        path: entry_name.clone(),
                        reason: e.to_string(),
                    }
                })?;
                
                if bytes_read == 0 {
                    break;
                }
                
                out_file.write_all(&buffer[..bytes_read]).map_err(|e| {
                    let reason = if e.kind() == io::ErrorKind::PermissionDenied {
                        "权限被拒绝".to_string()
                    } else {
                        e.to_string()
                    };
                    ExtractError::IoError {
                        operation: "写入文件".to_string(),
                        path: target_path.display().to_string(),
                        reason,
                    }
                })?;
                
                bytes_written += bytes_read as u64;
            }
            
            files_extracted += 1;
        }
    }
    
    // 最终进度回调
    if let Some(cb) = progress_cb {
        cb(total_entries, total_entries);
    }
    
    let elapsed_ms = start_time.elapsed().as_millis();
    
    eprintln!("[ZipExtract] step=complete files_extracted={} dirs_created={} bytes_written={} elapsed_ms={} zip_path=\"{}\" dest_dir=\"{}\"", 
        files_extracted, directories_created, bytes_written, elapsed_ms, zip_path.display(), dest_dir.display());
    
    Ok(ExtractReport {
        files_extracted,
        directories_created,
        bytes_written,
        elapsed_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;
    
    /// 创建测试用 ZIP 文件
    fn create_test_zip(zip_path: &Path, content: Vec<(&str, &[u8])>) -> io::Result<()> {
        let file = fs::File::create(zip_path)?;
        let mut zip = zip::ZipWriter::new(file);
        
        for (path, data) in content {
            zip.start_file(path, Default::default())?;
            zip.write_all(data)?;
        }
        
        zip.finish()?;
        Ok(())
    }
    
    #[test]
    fn test_normal_extraction() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let zip_path = temp_dir.path().join("test.zip");
        let extract_dir = temp_dir.path().join("extracted");
        
        // 创建测试 ZIP：包含目录和文件
        create_test_zip(&zip_path, vec![
            ("dir1/", b""),
            ("dir1/file1.txt", b"Hello, World!"),
            ("dir1/subdir/", b""),
            ("dir1/subdir/file2.txt", b"Test content"),
            ("root.txt", b"Root file"),
        ])?;
        
        // 执行解压
        let report = extract_zip_to_dir(&zip_path, &extract_dir, None, None)?;
        
        // 验证
        assert_eq!(report.files_extracted, 3, "应该提取 3 个文件");
        assert_eq!(report.directories_created, 3, "应该创建 3 个目录");
        assert!(extract_dir.join("dir1/file1.txt").exists());
        assert!(extract_dir.join("dir1/subdir/file2.txt").exists());
        assert!(extract_dir.join("root.txt").exists());
        
        // 验证内容
        let content = fs::read_to_string(extract_dir.join("dir1/file1.txt"))?;
        assert_eq!(content, "Hello, World!");
        
        Ok(())
    }
    
    #[test]
    fn test_zip_slip_detection() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let zip_path = temp_dir.path().join("malicious.zip");
        let extract_dir = temp_dir.path().join("extracted");
        
        // 创建恶意 ZIP：包含 .. 路径逃逸
        let file = fs::File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("../evil.txt", Default::default())?;
        zip.write_all(b"This should not be extracted outside")?;
        zip.finish()?;
        
        // 执行解压，应该失败
        let result = extract_zip_to_dir(&zip_path, &extract_dir, None, None);
        
        assert!(result.is_err(), "应该检测到 Zip Slip 攻击");
        if let Err(ExtractError::ZipSlipDetected { .. }) = result {
            // 正确的错误类型
        } else {
            panic!("应该返回 ZipSlipDetected 错误");
        }
        
        // 验证文件未被创建在目标目录外
        let evil_file = temp_dir.path().join("evil.txt");
        assert!(!evil_file.exists(), "恶意文件不应该被创建");
        
        Ok(())
    }
    
    #[test]
    fn test_cancellation() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let zip_path = temp_dir.path().join("test.zip");
        let extract_dir = temp_dir.path().join("extracted");
        
        // 创建较大的测试 ZIP
        create_test_zip(&zip_path, vec![
            ("file1.txt", b"Content 1"),
            ("file2.txt", b"Content 2"),
            ("file3.txt", b"Content 3"),
        ])?;
        
        // 创建取消标志并立即触发
        let cancel = AtomicBool::new(true);
        
        // 执行解压，应该立即返回
        let result = extract_zip_to_dir(&zip_path, &extract_dir, Some(&cancel), None);
        
        assert!(result.is_err(), "应该返回取消错误");
        if let Err(ExtractError::Cancelled) = result {
            // 正确的错误类型
        } else {
            panic!("应该返回 Cancelled 错误");
        }
        
        Ok(())
    }
}
