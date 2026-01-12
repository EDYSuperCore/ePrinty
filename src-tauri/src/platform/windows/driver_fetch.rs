// Windows 平台驱动包下载模块
// 
// 提供远程驱动包下载、sha256 校验和缓存功能

use std::path::{Path, PathBuf};
use std::fs;
use std::io::Read;
use std::time::Duration;
use url::Url;
use tauri::Manager;

/// 下载结果
#[derive(Debug, Clone)]
pub struct FetchResult {
    pub driver_uuid: String,
    pub uuid_root: PathBuf,
    pub payload_zip: PathBuf,
    pub source_used: String, // "cache" | "download"
    pub bytes: u64,
    pub sha256_actual: String,
}

/// 下载错误类型
#[derive(Debug)]
pub enum FetchError {
    /// 下载失败
    DownloadFailed {
        step: &'static str,
        url: String,
        attempt: u32,
        http_status: Option<u16>,
        error: String,
    },
    /// SHA256 不匹配
    Sha256Mismatch {
        expected: String,
        actual: String,
        payload_zip: String,
    },
    /// SHA256 格式无效
    InvalidSha256 {
        sha256: String,
        reason: String,
    },
    /// IO 错误
    IoError {
        step: &'static str,
        operation: &'static str,
        error: String,
    },
    /// 远程 URL 无效
    InvalidRemoteUrl {
        url: String,
        reason: String,
    },
    /// 下载失败（空响应体）
    DownloadFailedEmptyBody {
        status: u16,
        content_length: Option<u64>,
        bytes: u64,
        url: String,
    },
    /// 下载失败（HTTP 状态码错误）
    DownloadFailedStatus {
        status: u16,
        url: String,
    },
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::DownloadFailed { step, url, attempt, http_status, error } => {
                let status_msg = match http_status {
                    Some(status) => format!("HTTP {}", status),
                    None => "无状态码".to_string(),
                };
                write!(f, "下载失败 (step={}, attempt={}): {} | URL: {} | {}", 
                    step, attempt, status_msg, url, error)
            }
            FetchError::Sha256Mismatch { expected, actual, payload_zip } => {
                write!(f, "SHA256 校验失败\n期望: {}\n实际: {}\n文件: {}", 
                    expected, actual, payload_zip)
            }
            FetchError::InvalidSha256 { sha256, reason } => {
                write!(f, "SHA256 格式无效: {}\n原因: {}", sha256, reason)
            }
            FetchError::IoError { step, operation, error } => {
                write!(f, "IO 错误 (step={}, operation={}): {}", step, operation, error)
            }
            FetchError::InvalidRemoteUrl { url, reason } => {
                write!(f, "远程 URL 无效: {}\n原因: {}", url, reason)
            }
            FetchError::DownloadFailedEmptyBody { status, content_length, bytes, url } => {
                write!(f, "下载失败（空响应体）\n状态码: {}\nContent-Length: {:?}\n实际字节数: {}\nURL: {}", 
                    status, content_length, bytes, url)
            }
            FetchError::DownloadFailedStatus { status, url } => {
                write!(f, "下载失败（HTTP 状态码错误）\n状态码: {}\nURL: {}", status, url)
            }
        }
    }
}

impl std::error::Error for FetchError {}

/// 校验远程 URL 格式
/// 
/// # 要求
/// - URL 必须是完整 URL（含 http/https scheme）
/// - scheme 只允许 http/https
/// - 禁止包含 "://://" 这种双 scheme
/// - URL 必须能正确解析
fn validate_remote_url(url: &str) -> Result<Url, FetchError> {
    // 快速 pre-check：禁止包含 "://://"
    if url.contains("://://") {
        return Err(FetchError::InvalidRemoteUrl {
            url: url.to_string(),
            reason: "URL 包含双 scheme（如 http://://）".to_string(),
        });
    }
    
    // 使用 url crate 解析 URL
    let parsed_url = Url::parse(url).map_err(|e| FetchError::InvalidRemoteUrl {
        url: url.to_string(),
        reason: format!("URL 解析失败: {}", e),
    })?;
    
    // 检查 scheme 只允许 http/https
    let scheme = parsed_url.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(FetchError::InvalidRemoteUrl {
            url: url.to_string(),
            reason: format!("URL scheme 必须是 http 或 https（当前: {}）", scheme),
        });
    }
    
    // 检查 host 是否存在
    if parsed_url.host().is_none() {
        return Err(FetchError::InvalidRemoteUrl {
            url: url.to_string(),
            reason: "URL 缺少 host".to_string(),
        });
    }
    
    Ok(parsed_url)
}

/// 校验 SHA256 格式
fn validate_sha256(sha256: &str) -> Result<(), FetchError> {
    if sha256.is_empty() {
        return Err(FetchError::InvalidSha256 {
            sha256: sha256.to_string(),
            reason: "SHA256 不能为空".to_string(),
        });
    }
    
    if sha256.len() != 64 {
        return Err(FetchError::InvalidSha256 {
            sha256: sha256.to_string(),
            reason: format!("SHA256 长度必须为 64 字符（当前 {} 字符）", sha256.len()),
        });
    }
    
    if !sha256.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(FetchError::InvalidSha256 {
            sha256: sha256.to_string(),
            reason: "SHA256 只能包含十六进制字符（0-9, a-f, A-F）".to_string(),
        });
    }
    
    Ok(())
}

/// 计算文件的 SHA256 哈希值（流式读取，避免一次性读入内存）
pub fn sha256_file(path: &Path) -> Result<String, FetchError> {
    use sha2::{Sha256, Digest};
    
    let mut file = fs::File::open(path)
        .map_err(|e| FetchError::IoError {
            step: "sha256_verify",
            operation: "打开文件",
            error: format!("无法打开文件 {}: {}", path.display(), e),
        })?;
    
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 8192]; // 8KB 缓冲区
    
    loop {
        let bytes_read = file.read(&mut buffer)
            .map_err(|e| FetchError::IoError {
                step: "sha256_verify",
                operation: "读取文件",
                error: format!("无法读取文件 {}: {}", path.display(), e),
            })?;
        
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
    }
    
    let hash = hasher.finalize();
    Ok(format!("{:x}", hash))
}

/// 确保驱动包 ZIP 文件存在（下载或从缓存获取）
/// 
/// # 参数
/// - `drivers_root`: 驱动根目录
/// - `remote_url`: 远程 ZIP 文件 URL
/// - `expected_sha256`: 期望的 SHA256 哈希值（64 字符十六进制）
/// 
/// # 返回
/// - `Ok(FetchResult)`: 下载/缓存成功
/// - `Err(FetchError)`: 下载/校验失败
/// 
/// # 步骤
/// 1. compute_paths: 计算路径（基于 sha256 前缀生成 driver_uuid）
/// 2. cache_check: 检查缓存
/// 3. download: 下载（如果需要）
/// 4. sha256_verify: 校验 SHA256
/// 5. summary: 输出摘要
pub async fn ensure_payload_zip(
    drivers_root: &Path,
    remote_url: &str,
    expected_sha256: &str,
    app: Option<&tauri::AppHandle>,  // 用于发送进度事件（可选）
    printer_name: Option<&str>,  // 打印机名称（用于进度事件）
    job_id: &str,  // 安装任务 ID
) -> Result<FetchResult, FetchError> {
    eprintln!("[EnsurePayloadZip] start remote_url=\"{}\" expected_sha256=\"{}\" drivers_root=\"{}\"", 
        remote_url, expected_sha256, drivers_root.display());
    
    // ============================================================================
    // Step 0: validate_remote_url - 校验并规范化远程 URL（只做一次，后续统一使用）
    // ============================================================================
    eprintln!("[EnsurePayloadZip] step=validate_remote_url inputs=remote_url=\"{}\"", remote_url);
    
    let canonical_url = validate_remote_url(remote_url)?.to_string();
    
    eprintln!("[EnsurePayloadZip] step=validate_remote_url result=passed canonical_url=\"{}\"", canonical_url);
    
    // ============================================================================
    // Step 0.5: validate_sha256 - 校验 SHA256 格式
    // ============================================================================
    eprintln!("[EnsurePayloadZip] step=validate_sha256 inputs=expected_sha256=\"{}\"", expected_sha256);
    
    validate_sha256(expected_sha256)?;
    
    eprintln!("[EnsurePayloadZip] step=validate_sha256 result=passed expected_sha256=\"{}\"", expected_sha256);
    
    // ============================================================================
    // Step 1: compute_paths - 计算路径
    // ============================================================================
    // driver_uuid = "drv_" + sha256[0..12]
    let driver_uuid = format!("drv_{}", &expected_sha256[..12].to_lowercase());
    let uuid_root = drivers_root.join(&driver_uuid);
    let payload_dir = uuid_root.join("payload");
    let payload_zip = payload_dir.join("payload.zip");
    let payload_tmp = payload_dir.join("payload.zip.part");
    
    eprintln!("[EnsurePayloadZip] step=compute_paths inputs=expected_sha256=\"{}\" drivers_root=\"{}\"", 
        expected_sha256, drivers_root.display());
    eprintln!("[EnsurePayloadZip] step=compute_paths outputs=driver_uuid=\"{}\" uuid_root=\"{}\" payload_dir=\"{}\" payload_zip=\"{}\" payload_tmp=\"{}\"", 
        driver_uuid, uuid_root.display(), payload_dir.display(), payload_zip.display(), payload_tmp.display());
    
    // 创建 payload_dir
    if let Err(e) = fs::create_dir_all(&payload_dir) {
        return Err(FetchError::IoError {
            step: "compute_paths",
            operation: "创建 payload 目录",
            error: format!("无法创建 payload 目录 {}: {}", payload_dir.display(), e),
        });
    }
    
    // ============================================================================
    // Step 2: cache_check - 检查缓存
    // ============================================================================
    eprintln!("[EnsurePayloadZip] step=cache_check inputs=payload_zip=\"{}\"", payload_zip.display());
    
    let source_used = if payload_zip.exists() {
        // 计算现有文件的 SHA256
        match sha256_file(&payload_zip) {
            Ok(sha256_actual) => {
                let sha256_actual_lower = sha256_actual.to_lowercase();
                let expected_lower = expected_sha256.to_lowercase();
                
                if sha256_actual_lower == expected_lower {
                    // 缓存命中
                    let file_size = fs::metadata(&payload_zip)
                        .map_err(|e| FetchError::IoError {
                            step: "cache_check",
                            operation: "获取文件大小",
                            error: format!("无法获取文件大小 {}: {}", payload_zip.display(), e),
                        })?
                        .len();
                    
                    eprintln!("[EnsurePayloadZip] step=cache_check result=cache_hit sha256_actual=\"{}\" file_size={}", 
                        sha256_actual, file_size);
                    
                    return Ok(FetchResult {
                        driver_uuid,
                        uuid_root,
                        payload_zip,
                        source_used: "cache".to_string(),
                        bytes: file_size,
                        sha256_actual,
                    });
                } else {
                    // 缓存损坏（SHA256 不匹配）
                    eprintln!("[EnsurePayloadZip] step=cache_check result=cache_corrupt expected=\"{}\" actual=\"{}\"", 
                        expected_sha256, sha256_actual);
                    
                    if let Err(e) = fs::remove_file(&payload_zip) {
                        eprintln!("[EnsurePayloadZip] step=cache_check warning=remove_corrupt_failed error=\"{}\"", e);
                    }
                    
                    // 继续下载
                    "download".to_string()
                }
            }
            Err(e) => {
                eprintln!("[EnsurePayloadZip] step=cache_check result=verify_failed error=\"{}\"", e);
                // 删除损坏的文件，继续下载
                let _ = fs::remove_file(&payload_zip);
                "download".to_string()
            }
        }
    } else {
        eprintln!("[EnsurePayloadZip] step=cache_check result=cache_miss payload_zip=\"{}\" (不存在)", payload_zip.display());
        "download".to_string()
    };
    
    // ============================================================================
    // Step 3: download - 下载 ZIP 文件（应用内下载，禁止系统下载）
    // ============================================================================
    // 注意：使用 canonical_url，不再使用 remote_url，避免二次拼接
    eprintln!("[DriverFetch] step=download_internal start url=\"{}\" dest_tmp=\"{}\" dest_final=\"{}\"", 
        canonical_url, payload_tmp.display(), payload_zip.display());
    
    // 创建 StepReporter（仅在需要下载时）
    let mut step_reporter_opt: Option<crate::platform::windows::step_reporter::StepReporter> = if source_used == "download" {
        if let (Some(app_handle), Some(printer)) = (app, printer_name) {
            Some(crate::platform::windows::step_reporter::StepReporter::start(
                std::sync::Arc::new(app_handle.clone()),
                job_id.to_string(),
                printer.to_string(),
                "driver.download".to_string(),
                "正在下载驱动包".to_string(),
            ))
        } else {
            None
        }
    } else {
        None
    };
    
    // 删除临时文件（如果存在）
    if payload_tmp.exists() {
        let _ = fs::remove_file(&payload_tmp);
    }
    
    // 下载配置
    const MAX_ATTEMPTS: u32 = 3;
    const TIMEOUT_SECS: u64 = 120;
    
    // 脱敏 URL（用于日志）- 基于 canonical_url，不再拼接 scheme
    let url_display = if let Some(domain_end) = canonical_url.find("://") {
        let after_protocol = &canonical_url[domain_end + 3..];
        if let Some(path_start) = after_protocol.find('/') {
            let domain = &after_protocol[..path_start];
            let path = &after_protocol[path_start..];
            let path_summary = if path.len() > 50 {
                format!("{}...", &path[..50])
            } else {
                path.to_string()
            };
            // 注意：这里不再拼接 scheme，直接使用 canonical_url 的 scheme
            format!("{}://{}{}", &canonical_url[..domain_end + 3], domain, path_summary)
        } else {
            // 注意：这里不再拼接 scheme，直接使用 canonical_url 的 scheme
            format!("{}://{}", &canonical_url[..domain_end + 3], after_protocol)
        }
    } else {
        // 如果 canonical_url 没有 scheme（不应该发生，因为 validate_remote_url 已检查），直接使用
        canonical_url.clone()
    };
    
    let mut last_error: Option<FetchError> = None;
    
    for attempt in 1..=MAX_ATTEMPTS {
        eprintln!("[DriverFetch] step=download_internal attempt={}/{} url=\"{}\"", attempt, MAX_ATTEMPTS, url_display);
        
        let start_time = std::time::Instant::now();
        
        // 使用 reqwest 下载（应用内 HTTP 客户端，不会被 IDM 接管）
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(TIMEOUT_SECS))
            .user_agent("ePrinty/1.4.1") // 设置 User-Agent，避免被下载工具识别
            .build()
            .map_err(|e| FetchError::DownloadFailed {
                step: "download_internal",
                url: canonical_url.clone(),
                attempt,
                http_status: None,
                error: format!("无法创建 HTTP 客户端: {}", e),
            })?;
        
        // 注意：使用 canonical_url，不再使用 remote_url
        match client.get(&canonical_url).send().await {
            Ok(response) => {
                let status = response.status();
                let http_status_code = status.as_u16();
                
                // 获取 Content-Length（如果存在）
                let content_length = response.content_length();
                
                // 检查是否重定向（使用 canonical_url 比较）
                let redirected = response.url().as_str() != canonical_url;
                
                eprintln!("[DriverFetch] step=download_internal response status={} content_length={:?} redirected={} url=\"{}\"", 
                    http_status_code, content_length, redirected, url_display);
                
                // 更新下载开始进度
                if let Some(ref mut reporter) = step_reporter_opt {
                    let percent = content_length.and_then(|total| {
                        if total > 0 {
                            Some(0.0)
                        } else {
                            None
                        }
                    });
                    reporter.update_progress(
                        Some(0),
                        content_length,
                        Some("bytes".to_string()),
                        percent,
                        Some("正在下载驱动包".to_string()),
                    );
                }
                
                // ============================================================================
                // 严格成功判据 1: HTTP 状态码检查
                // ============================================================================
                // 禁止 204 (No Content) 和 205 (Reset Content)
                if http_status_code == 204 || http_status_code == 205 {
                    let error = FetchError::DownloadFailedEmptyBody {
                        status: http_status_code,
                        content_length,
                        bytes: 0,
                        url: canonical_url.clone(),
                    };
                    eprintln!("[DriverFetch] step=download_internal result=failed reason=empty_body_status status={} url=\"{}\"", 
                        http_status_code, url_display);
                    
                    // 发送失败事件
                    if let Some(reporter) = step_reporter_opt.take() {
                        let _ = reporter.failed(
                            "DOWNLOAD_EMPTY_BODY".to_string(),
                            format!("下载失败：HTTP {} (空响应)", http_status_code),
                            None,
                            None,
                            None,
                        );
                    }
                    
                    // 清理临时文件
                    let _ = fs::remove_file(&payload_tmp);
                    // 如果最终文件已存在，也删除（避免 cache_check 命中空包）
                    let _ = fs::remove_file(&payload_zip);
                    
                    last_error = Some(error);
                    break;
                }
                
                // 禁止非 2xx 状态码（除了 204/205 已在上面处理）
                if !status.is_success() {
                    let error = FetchError::DownloadFailedStatus {
                        status: http_status_code,
                        url: canonical_url.clone(),
                    };
                    eprintln!("[DriverFetch] step=download_internal result=failed reason=non_success_status status={} url=\"{}\"", 
                        http_status_code, url_display);
                    
                    // 如果是最后一次尝试，发送失败事件
                    if attempt == MAX_ATTEMPTS {
                        if let Some(reporter) = step_reporter_opt.take() {
                            let _ = reporter.failed(
                                format!("HTTP_{}", http_status_code),
                                format!("下载失败：HTTP {} (尝试 {}/{})", http_status_code, attempt, MAX_ATTEMPTS),
                                None,
                                None,
                                None,
                            );
                        }
                    }
                    
                    last_error = Some(error);
                    
                    // 如果不是最后一次尝试，等待后重试（指数退避）
                    if attempt < MAX_ATTEMPTS {
                        let delay_secs = 2_u64.pow(attempt - 1); // 1s, 2s, 4s
                        eprintln!("[DriverFetch] step=download_internal attempt={} failed status={} retry_after={}s", 
                            attempt, http_status_code, delay_secs);
                        tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                        continue;
                    }
                    break;
                }
                
                // ============================================================================
                // 严格成功判据 2: Content-Length 检查
                // ============================================================================
                if let Some(cl) = content_length {
                    if cl == 0 {
                        let error = FetchError::DownloadFailedEmptyBody {
                            status: http_status_code,
                            content_length: Some(0),
                            bytes: 0,
                            url: canonical_url.clone(),
                        };
                        eprintln!("[DriverFetch] step=download_internal result=failed reason=content_length_zero status={} content_length=0 url=\"{}\"", 
                            http_status_code, url_display);
                        
                        // 清理临时文件
                        let _ = fs::remove_file(&payload_tmp);
                        // 如果最终文件已存在，也删除
                        let _ = fs::remove_file(&payload_zip);
                        
                        last_error = Some(error);
                        break;
                    }
                }
                
                // 确保父目录存在
                if let Some(parent) = payload_tmp.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        return Err(FetchError::IoError {
                            step: "download_internal",
                            operation: "创建父目录",
                            error: format!("无法创建父目录 {}: {}", parent.display(), e),
                        });
                    }
                }
                
                // 下载到临时文件（使用 OpenOptions 确保原子性）
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(&payload_tmp)
                    .map_err(|e| FetchError::IoError {
                        step: "download_internal",
                        operation: "创建临时文件",
                        error: format!("无法创建临时文件 {}: {}", payload_tmp.display(), e),
                    })?;
                
                let mut stream = response.bytes_stream();
                let mut total_bytes = 0u64;
                let mut last_progress_log = std::time::Instant::now();
                
                use futures_util::StreamExt;
                while let Some(chunk_result) = stream.next().await {
                    let chunk = chunk_result.map_err(|e| FetchError::DownloadFailed {
                        step: "download_internal",
                        url: canonical_url.clone(),
                        attempt,
                        http_status: Some(http_status_code),
                        error: format!("读取响应流失败: {}", e),
                    })?;
                    
                    use std::io::Write;
                    file.write_all(&chunk)
                        .map_err(|e| FetchError::IoError {
                            step: "download_internal",
                            operation: "写入文件",
                            error: format!("写入文件失败 {}: {}", payload_tmp.display(), e),
                        })?;
                    
                    total_bytes += chunk.len() as u64;
                    
                    // 每 256KB 或每 300ms 发送一次进度事件
                    let should_emit_progress = last_progress_log.elapsed().as_millis() >= 300 || 
                        (total_bytes % 262144 == 0 && total_bytes > 0);  // 每 256KB
                    
                    if should_emit_progress {
                        if let Some(ref mut reporter) = step_reporter_opt {
                            let percent = content_length.map(|total| {
                                if total > 0 {
                                    ((total_bytes * 100) / total).min(100) as f64
                                } else {
                                    0.0
                                }
                            });
                            reporter.update_progress(
                                Some(total_bytes),
                                content_length,
                                Some("bytes".to_string()),
                                percent,
                                Some(format!("已下载 {} / {} MB", 
                                    total_bytes / 1024 / 1024,
                                    content_length.map(|c| c / 1024 / 1024).unwrap_or(0))),
                            );
                        }
                        last_progress_log = std::time::Instant::now();
                    }
                    
                    // 每 5 秒输出一次日志（可选）
                    if last_progress_log.elapsed().as_secs() >= 5 {
                        eprintln!("[DriverFetch] step=download_internal progress bytes={} content_length={:?}", 
                            total_bytes, content_length);
                        last_progress_log = std::time::Instant::now();
                    }
                }
                
                // 定期 flush（确保数据写入磁盘）
                use std::io::Write;
                file.flush()
                    .map_err(|e| FetchError::IoError {
                        step: "download_internal",
                        operation: "刷新文件",
                        error: format!("刷新文件失败 {}: {}", payload_tmp.display(), e),
                    })?;
                
                // 同步文件（fsync，确保数据持久化）
                file.sync_all()
                    .map_err(|e| FetchError::IoError {
                        step: "download_internal",
                        operation: "同步文件",
                        error: format!("同步文件失败 {}: {}", payload_tmp.display(), e),
                    })?;
                
                let elapsed_ms = start_time.elapsed().as_millis();
                
                // ============================================================================
                // 严格成功判据 3: 实际写入字节数检查
                // ============================================================================
                if total_bytes == 0 {
                    let error = FetchError::DownloadFailedEmptyBody {
                        status: http_status_code,
                        content_length,
                        bytes: 0,
                        url: canonical_url.clone(),
                    };
                    eprintln!("[DriverFetch] step=download_internal result=failed reason=zero_bytes status={} content_length={:?} bytes=0 url=\"{}\"", 
                        http_status_code, content_length, url_display);
                    
                    // 发送失败事件
                    if let Some(reporter) = step_reporter_opt.take() {
                        let _ = reporter.failed(
                            "DOWNLOAD_EMPTY_BODY".to_string(),
                            "下载失败：文件为空".to_string(),
                            None,
                            None,
                            None,
                        );
                    }
                    
                    // 清理临时文件
                    let _ = fs::remove_file(&payload_tmp);
                    // 如果最终文件已存在，也删除
                    let _ = fs::remove_file(&payload_zip);
                    
                    last_error = Some(error);
                    break;
                }
                
                // ============================================================================
                // 所有判据通过，执行原子重命名
                // ============================================================================
                // 原子重命名：dest_tmp -> dest_final（保证原子性）
                fs::rename(&payload_tmp, &payload_zip)
                    .map_err(|e| FetchError::IoError {
                        step: "download_internal",
                        operation: "重命名文件",
                        error: format!("重命名文件失败 {} -> {}: {}", payload_tmp.display(), payload_zip.display(), e),
                    })?;
                
                let evidence = format!(
                    "step=download_internal success status={} bytes={} content_length={:?} elapsed_ms={} redirected={} url=\"{}\"",
                    http_status_code, total_bytes, content_length, elapsed_ms, redirected, url_display
                );
                
                eprintln!("[DriverFetch] step=download_internal result=success evidence=\"{}\"", evidence);
                
                // 发送成功事件（确保 percent=100）
                if let Some(reporter) = step_reporter_opt.take() {
                    // 最后一次进度更新：确保 percent=100
                    reporter.update_progress(
                        Some(total_bytes),
                        Some(total_bytes),
                        Some("bytes".to_string()),
                        Some(100.0),
                        Some(format!("下载完成：{} MB", total_bytes / 1024 / 1024)),
                    );
                    
                    let meta = serde_json::json!({
                        "bytes": total_bytes,
                        "content_length": content_length,
                        "url": url_display,
                    });
                    let _ = reporter.success(
                        format!("下载完成：{} MB", total_bytes / 1024 / 1024),
                        Some(meta),
                    );
                }
                
                // 下载成功，跳出循环
                break;
            }
            Err(e) => {
                last_error = Some(FetchError::DownloadFailed {
                    step: "download_internal",
                    url: canonical_url.clone(),
                    attempt,
                    http_status: None,
                    error: format!("请求失败: {}", e),
                });
                
                // 如果是最后一次尝试，发送失败事件
                if attempt == MAX_ATTEMPTS {
                    if let Some(reporter) = step_reporter_opt.take() {
                        let _ = reporter.failed(
                            "DOWNLOAD_REQUEST_FAILED".to_string(),
                            format!("下载失败：请求失败 (尝试 {}/{})", attempt, MAX_ATTEMPTS),
                            None,
                            Some(format!("{}", e)),
                            None,
                        );
                    }
                }
                
                // 如果不是最后一次尝试，等待后重试（指数退避）
                if attempt < MAX_ATTEMPTS {
                    let delay_secs = 2_u64.pow(attempt - 1);
                    eprintln!("[DriverFetch] step=download_internal attempt={} failed error=\"{}\" retry_after={}s", 
                        attempt, e, delay_secs);
                    tokio::time::sleep(Duration::from_secs(delay_secs)).await;
                    continue;
                }
                break;
            }
        }
    }
    
    // 如果所有尝试都失败，返回错误
    if let Some(error) = last_error {
        // 清理临时文件（避免下次误命中）
        if payload_tmp.exists() {
            let _ = fs::remove_file(&payload_tmp);
        }
        return Err(error);
    }
    
    // ============================================================================
    // Step 4: sha256_verify - 校验 SHA256
    // ============================================================================
    eprintln!("[EnsurePayloadZip] step=sha256_verify inputs=payload_zip=\"{}\" expected_sha256=\"{}\"", 
        payload_zip.display(), expected_sha256);
    
    // 发送 Verify 开始事件
    let mut verify_reporter_opt: Option<crate::platform::windows::step_reporter::StepReporter> = if let (Some(app_handle), Some(printer)) = (app, printer_name) {
        Some(crate::platform::windows::step_reporter::StepReporter::start(
            std::sync::Arc::new(app_handle.clone()),
            job_id.to_string(),
            printer.to_string(),
            "driver.verify".to_string(),
            "正在校验驱动包".to_string(),
        ))
    } else {
        None
    };
    
    let sha256_actual = sha256_file(&payload_zip)?;
    let sha256_actual_lower = sha256_actual.to_lowercase();
    let expected_lower = expected_sha256.to_lowercase();
    
    if sha256_actual_lower != expected_lower {
        // SHA256 不匹配，删除文件
        let _ = fs::remove_file(&payload_zip);
        
        let evidence = format!(
            "step=sha256_verify expected=\"{}\" actual=\"{}\" payload_zip=\"{}\"",
            expected_sha256, sha256_actual, payload_zip.display()
        );
        eprintln!("[EnsurePayloadZip] step=sha256_verify result=failed evidence=\"{}\"", evidence);
        
        // 发送 Verify 失败事件
        if let Some(reporter) = verify_reporter_opt.take() {
            let _ = reporter.failed(
                "SHA256_MISMATCH".to_string(),
                format!("SHA256 校验失败：期望 {}，实际 {}", expected_sha256, sha256_actual),
                None,
                Some(evidence.clone()),
                None,
            );
        }
        
        return Err(FetchError::Sha256Mismatch {
            expected: expected_sha256.to_string(),
            actual: sha256_actual,
            payload_zip: payload_zip.display().to_string(),
        });
    }
    
    eprintln!("[EnsurePayloadZip] step=sha256_verify result=passed sha256_actual=\"{}\"", sha256_actual);
    
    // 发送 Verify 成功事件
    if let Some(reporter) = verify_reporter_opt.take() {
        let meta = serde_json::json!({
            "sha256": sha256_actual,
        });
        let _ = reporter.success(
            "驱动包校验通过".to_string(),
            Some(meta),
        );
    }
    
    // ============================================================================
    // Step 5: summary - 输出摘要
    // ============================================================================
    let file_size = fs::metadata(&payload_zip)
        .map_err(|e| FetchError::IoError {
            step: "summary",
            operation: "获取文件大小",
            error: format!("无法获取文件大小 {}: {}", payload_zip.display(), e),
        })?
        .len();
    
    let evidence = format!(
        "step=summary driver_uuid=\"{}\" uuid_root=\"{}\" payload_zip=\"{}\" source_used=\"{}\" bytes={} sha256_actual=\"{}\"",
        driver_uuid, uuid_root.display(), payload_zip.display(), source_used, file_size, sha256_actual
    );
    
    eprintln!("[EnsurePayloadZip] step=summary result=success evidence=\"{}\"", evidence);
    
    Ok(FetchResult {
        driver_uuid,
        uuid_root,
        payload_zip,
        source_used,
        bytes: file_size,
        sha256_actual,
    })
}
