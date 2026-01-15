use serde_json::Value;
use std::{
    net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs},
    process::Command,
    sync::Once,
    time::{Duration, Instant},
};
use sha2::{Digest, Sha256};
use tauri::AppHandle;
use tokio::time::{sleep, timeout, Duration as TokioDuration};
use url::Url;

use crate::install_event_emitter::StepReporter;

/// Generate a monotonic-ish timestamp in milliseconds
fn now_ts_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

struct CmdOutput {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    success: bool,
}

struct DeviceTarget {
    uri: String,
    host: String,
    port: u16,
}

struct StepError {
    code: &'static str,
    detail: String,
}

struct VerifyError {
    code: &'static str,
    detail: String,
}

enum VerifyStatus {
    Exists(Option<String>),
    NotFound(String),
    Error(VerifyError),
}
const CMD_TIMEOUT_MS: u64 = 12_000;
const LPADMIN_TIMEOUT_MS: u64 = 20_000;
const FINAL_VERIFY_TIMEOUT_SECS: u64 = 10;
const FINAL_VERIFY_RETRY_MS: u64 = 500;

async fn run_cmd(cmd: &str, args: &[&str], timeout_ms: u64) -> Result<CmdOutput, String> {
    let cmd_owned = cmd.to_string();
    let args_owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    let task = tokio::task::spawn_blocking(move || {
        let output = Command::new(&cmd_owned)
            .args(&args_owned)
            .output()
            .map_err(|e| format!("执行命令失败: {} {}", cmd_owned, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let result = CmdOutput {
            stdout,
            stderr,
            exit_code: output.status.code(),
            success: output.status.success(),
        };

        if cfg!(debug_assertions) {
            eprintln!(
                "[macOS.run_cmd] cmd=\"{}\" args={:?} success={} exit_code={:?}",
                cmd_owned, args_owned, result.success, result.exit_code
            );
            if !result.stdout.is_empty() {
                eprintln!(
                    "[macOS.run_cmd] stdout=\"{}\"",
                    truncate_text(&result.stdout, 400)
                );
            }
            if !result.stderr.is_empty() {
                eprintln!(
                    "[macOS.run_cmd] stderr=\"{}\"",
                    truncate_text(&result.stderr, 400)
                );
            }
        }

        Ok(result)
    });

    match timeout(TokioDuration::from_millis(timeout_ms), task).await {
        Ok(join_result) => join_result.map_err(|e| format!("命令执行任务失败: {}", e))?,
        Err(_) => Err(format!("命令执行超时 ({} ms)", timeout_ms)),
    }
}

fn truncate_text(input: &str, max_len: usize) -> String {
    if input.len() <= max_len {
        return input.to_string();
    }
    let mut truncated = input[..max_len].to_string();
    truncated.push_str("...");
    truncated
}

fn is_privilege_error(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    lower.contains("not authorized")
        || lower.contains("not permitted")
        || lower.contains("permission denied")
        || lower.contains("privilege")
        || lower.contains("operation not permitted")
        || lower.contains("authentication")
        || lower.contains("sudo")
}

fn sanitize_queue_name(input: &str) -> String {
    let mut result = String::new();
    let mut prev_dash = false;
    for ch in input.chars() {
        let allowed = ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-';
        if allowed {
            result.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            result.push('-');
            prev_dash = true;
        }
    }

    let mut trimmed = result.trim_matches('-').to_string();
    if trimmed.is_empty() {
        trimmed = format!("queue-{}", now_ts_ms() % 10000);
    }

    if trimmed.len() > 60 {
        trimmed.truncate(60);
        trimmed = trimmed.trim_end_matches('-').to_string();
        if trimmed.is_empty() {
            trimmed = format!("queue-{}", now_ts_ms() % 10000);
        }
    }

    trimmed
}

const QUEUE_HASH_ALGO: &str = "sha256-8";
static QUEUE_HASH_ALGO_LOG: Once = Once::new();

fn log_queue_hash_algo_once() {
    QUEUE_HASH_ALGO_LOG.call_once(|| {
        eprintln!("[InstallPrinterMacOS] queue_hash_algo={}", QUEUE_HASH_ALGO);
    });
}

fn short_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let hex = format!("{:x}", digest);
    hex.chars().take(8).collect()
}

fn normalize_uri_for_compare(input: &str) -> String {
    input.trim().to_lowercase()
}

fn parse_lpstat_device_uri(queue: &str, output: &str) -> Option<String> {
    let prefix = format!("device for {}:", queue);
    for line in output.lines() {
        if let Some(idx) = line.find(&prefix) {
            return Some(line[idx + prefix.len()..].trim().to_string());
        }
    }
    None
}

fn is_queue_not_found(stderr: &str) -> bool {
    let lower = stderr.to_lowercase();
    let en_patterns = [
        "unknown destination",
        "not found",
        "invalid destination",
        "invalid destination name",
    ];
    if en_patterns.iter().any(|p| lower.contains(p)) {
        return true;
    }
    let zh_patterns = [
        "无效目的位置",
        "无效目的地",
        "目的位置名称无效",
        "目的地名称无效",
        "未添加目的位置",
        "未添加目的地",
        "没有打印机",
    ];
    zh_patterns.iter().any(|p| stderr.contains(p))
}

fn default_port_for_scheme(scheme: &str) -> u16 {
    match scheme {
        "ipp" | "ipps" => 631,
        "socket" => 9100,
        "lpd" => 515,
        _ => 631,
    }
}

fn normalize_uri_input(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.starts_with("ipp:") && !trimmed.starts_with("ipp://") {
        return format!("ipp://{}", trimmed.trim_start_matches("ipp:"));
    }
    if trimmed.starts_with("ipps:") && !trimmed.starts_with("ipps://") {
        return format!("ipps://{}", trimmed.trim_start_matches("ipps:"));
    }
    if trimmed.starts_with("socket:") && !trimmed.starts_with("socket://") {
        return format!("socket://{}", trimmed.trim_start_matches("socket:"));
    }
    if trimmed.starts_with("lpd:") && !trimmed.starts_with("lpd://") {
        return format!("lpd://{}", trimmed.trim_start_matches("lpd:"));
    }
    trimmed.to_string()
}

fn parse_host_port(raw: &str) -> Result<(String, Option<u16>), String> {
    let cleaned = raw
        .trim()
        .trim_start_matches('\\')
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("")
        .to_string();

    if cleaned.is_empty() {
        return Err("设备地址为空".to_string());
    }

    if let Some((host, port_str)) = cleaned.rsplit_once(':') {
        if port_str.chars().all(|c| c.is_ascii_digit()) {
            let port = port_str.parse::<u16>().map_err(|_| "端口号无效".to_string())?;
            return Ok((host.to_string(), Some(port)));
        }
    }

    Ok((cleaned, None))
}

fn parse_device_target(path: &str, fallback_name: &str) -> Result<DeviceTarget, String> {
    let raw = if path.trim().is_empty() { fallback_name } else { path };
    let normalized = normalize_uri_input(raw);

    if normalized.contains("://") {
        let url = Url::parse(&normalized).map_err(|e| format!("无法解析设备 URI: {}", e))?;
        let host = url.host_str().ok_or_else(|| "URI 缺少 host".to_string())?.to_string();
        let port = url.port().unwrap_or(default_port_for_scheme(url.scheme()));
        return Ok(DeviceTarget {
            uri: normalized,
            host,
            port,
        });
    }

    let (host, port_opt) = parse_host_port(&normalized)?;
    let port = port_opt.unwrap_or(631);
    let uri = if port == 631 {
        format!("ipp://{}/ipp/print", host)
    } else {
        format!("ipp://{}:{}/ipp/print", host, port)
    };

    Ok(DeviceTarget { uri, host, port })
}

fn build_meta(
    queue_name: &str,
    ip: &str,
    uri: Option<&str>,
    requested_mode: &str,
    effective_mode: &str,
) -> Value {
    let mut meta = serde_json::Map::new();
    meta.insert("platform".into(), Value::String("macos".into()));
    meta.insert("installMode".into(), Value::String(effective_mode.to_string()));
    meta.insert("requestedMode".into(), Value::String(requested_mode.to_string()));
    meta.insert("effectiveMode".into(), Value::String(effective_mode.to_string()));
    meta.insert("queueName".into(), Value::String(queue_name.to_string()));
    meta.insert("ip".into(), Value::String(ip.to_string()));
    if let Some(uri_str) = uri {
        meta.insert("uri".into(), Value::String(uri_str.to_string()));
        meta.insert("deviceUri".into(), Value::String(uri_str.to_string()));
    }
    Value::Object(meta)
}

fn probe_device(host: &str, port: u16) -> Result<(), String> {
    let socket = if let Ok(ip) = host.parse::<IpAddr>() {
        SocketAddr::new(ip, port)
    } else {
        let mut addrs = (host, port)
            .to_socket_addrs()
            .map_err(|_| "无法解析主机地址".to_string())?;
        addrs.next().ok_or_else(|| "无法解析主机地址".to_string())?
    };

    TcpStream::connect_timeout(&socket, Duration::from_secs(3))
        .map(|_| ())
        .map_err(|e| format!("无法连接到 {}:{} ({})", host, port, e))
}

fn build_queue_name(printer_name: &str, device_uri: &str) -> String {
    log_queue_hash_algo_once();
    let base_prefix = "eprinty";
    let mut ascii_part = String::new();
    let mut prev_dash = false;

    for ch in printer_name.chars() {
        if ch.is_ascii_alphanumeric() {
            ascii_part.push(ch);
            prev_dash = false;
        } else if !prev_dash {
            ascii_part.push('-');
            prev_dash = true;
        }
    }

    let ascii_part = ascii_part.trim_matches('-').to_string();
    let ascii_part = if ascii_part.is_empty() {
        "printer".to_string()
    } else {
        ascii_part
    };

    let device_uri_norm = normalize_uri_for_compare(device_uri);
    let hash_part = short_hash(&format!("{}|{}", printer_name, device_uri_norm));
    let max_len = 63usize;
    let reserved = base_prefix.len() + 1 + 1 + hash_part.len();
    let max_ascii_len = max_len.saturating_sub(reserved);

    let mut ascii_trimmed = ascii_part;
    if max_ascii_len > 0 && ascii_trimmed.len() > max_ascii_len {
        ascii_trimmed.truncate(max_ascii_len);
        ascii_trimmed = ascii_trimmed.trim_matches('-').to_string();
        if ascii_trimmed.is_empty() {
            ascii_trimmed = "printer".to_string();
        }
    }

    let queue_name = format!("{}-{}-{}", base_prefix, ascii_trimmed, hash_part);
    let filtered: String = queue_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' { c } else { '-' })
        .collect();

    let filtered = filtered.trim_matches('-').to_string();
    if filtered.is_empty() {
        format!("{}-printer-{}", base_prefix, hash_part)
    } else if filtered.len() > max_len {
        let mut trimmed = filtered;
        trimmed.truncate(max_len);
        trimmed.trim_matches('-').to_string()
    } else {
        filtered
    }
}

async fn ensure_queue(queue: &str, uri: &str, display_name: &str) -> Result<(), StepError> {
    let base_args = ["-p", queue, "-E", "-v", uri, "-D", display_name];
    let attempts = [
        vec!["-m", "everywhere"],
        vec!["-m", "raw"],
        vec![],
    ];
    let mut last_stderr = None;
    let mut last_exit_code = None;

    for extra in attempts.iter() {
        let mut args: Vec<&str> = base_args.to_vec();
        args.extend(extra.iter().copied());

        eprintln!(
            "[macOS.ensure_queue] queue_name=\"{}\" uri=\"{}\" display_name=\"{}\" args={:?}",
            queue,
            uri,
            display_name,
            args
        );
        let output = run_cmd("lpadmin", &args, LPADMIN_TIMEOUT_MS)
            .await
            .map_err(|e| StepError {
                code: "E_QUEUE_CREATE_FAILED",
                detail: e,
            })?;
        if output.success {
            return Ok(());
        }

        last_stderr = Some(output.stderr.clone());
        last_exit_code = output.exit_code;
        if is_privilege_error(&output.stderr) {
            return Err(StepError {
                code: "E_NO_PRIVILEGE",
                detail: format!(
                    "queue_name={} exit_code={:?} stderr={}",
                    queue,
                    output.exit_code,
                    truncate_text(&output.stderr, 300)
                ),
            });
        }
    }

    Err(StepError {
        code: "E_QUEUE_CREATE_FAILED",
        detail: format!(
            "queue_name={} exit_code={:?} stderr={}",
            queue,
            last_exit_code,
            truncate_text(last_stderr.as_deref().unwrap_or("lpadmin 执行失败"), 300)
        ),
    })
}

async fn verify_queue_once(queue: &str, attempt: u32) -> VerifyStatus {
    let cmd = "lpoptions";
    let args = ["-p", queue];
    let result = run_cmd(cmd, &args, CMD_TIMEOUT_MS).await;

    match result {
        Ok(output) => {
            eprintln!(
                "[InstallPrinterMacOS] finalVerify attempt={} cmd=\"{}\" args={:?} exit_code={:?} stdout_len={} stderr_len={} stdout_snip=\"{}\" stderr_snip=\"{}\"",
                attempt,
                cmd,
                args,
                output.exit_code,
                output.stdout.len(),
                output.stderr.len(),
                truncate_text(&output.stdout, 200),
                truncate_text(&output.stderr, 200),
            );

            if output.success {
                let uri_output = run_cmd("lpstat", &["-v", queue], CMD_TIMEOUT_MS).await;
                if let Ok(uri_out) = uri_output {
                    if uri_out.success {
                        let actual_uri = parse_lpstat_device_uri(queue, &uri_out.stdout);
                        return VerifyStatus::Exists(actual_uri);
                    }
                    eprintln!(
                        "[InstallPrinterMacOS] finalVerify warn: lpstat -v failed queue_name={} exit_code={:?} stderr_snip=\"{}\"",
                        queue,
                        uri_out.exit_code,
                        truncate_text(&uri_out.stderr, 200)
                    );
                }
                return VerifyStatus::Exists(None);
            }

            if is_queue_not_found(&output.stderr) {
                return VerifyStatus::NotFound(format!(
                    "cmd={} args={:?} exit_code={:?} stderr_snip=\"{}\"",
                    cmd,
                    args,
                    output.exit_code,
                    truncate_text(&output.stderr, 200)
                ));
            }

            VerifyStatus::Error(VerifyError {
                code: "E_VERIFY_CMD_FAILED",
                detail: format!(
                    "cmd={} args={:?} exit_code={:?} stderr_snip=\"{}\"",
                    cmd,
                    args,
                    output.exit_code,
                    truncate_text(&output.stderr, 200)
                ),
            })
        }
        Err(err) => VerifyStatus::Error(VerifyError {
            code: "E_VERIFY_CMD_FAILED",
            detail: format!("cmd={} args={:?} error={}", cmd, args, err),
        }),
    }
}

#[allow(non_snake_case)]
pub async fn install_printer_macos(
    app: AppHandle,
    name: String,
    path: String,
    installMode: Option<String>,
    dry_run: bool,
) -> Result<crate::InstallResult, String> {
    let job_id = format!("job_{}_{}", now_ts_ms(), now_ts_ms() % 10000);

    let requested_mode = installMode.unwrap_or_else(|| "auto".to_string());
    let requested_mode_lower = requested_mode.to_lowercase();
    let effective_mode = "driverless".to_string();

    let base_queue_name = sanitize_queue_name(&name);
    let target = match parse_device_target(&path, &name) {
        Ok(target) => target,
        Err(err) => {
            let init_meta = build_meta(
                &base_queue_name,
                &path,
                None,
                &requested_mode_lower,
                &effective_mode,
            );
            let mut reporter = StepReporter::new(
                app.clone(),
                job_id.clone(),
                name.clone(),
                Some(effective_mode.clone()),
                Some(init_meta),
            );

            let init_message = if requested_mode_lower != "driverless" {
                format!(
                    "macOS 平台已将 installMode=\"{}\" 降级为 \"{}\"",
                    requested_mode_lower, effective_mode
                )
            } else {
                "开始安装 (macOS driverless)".to_string()
            };
            let _ = reporter.emit_job_init(&init_message);
            let _ = reporter.emit_step_running("device.probe", "检测打印机可用性", None);
            let _ = reporter.emit_step_fail("device.probe", "E_INVALID_URI", "设备地址无效", Some(&err));
            let _ = reporter.emit_job_failed("E_INVALID_URI", "设备探测失败", Some(&err));
            let _ = reporter.emit_job_done(false, Some("设备探测失败"));

            return Ok(crate::InstallResult {
                success: false,
                message: "设备探测失败".into(),
                method: Some("driverless".into()),
                stdout: None,
                stderr: None,
                effective_dry_run: dry_run,
                job_id,
            });
        }
    };

    let device_uri_norm = normalize_uri_for_compare(&target.uri);
    let mut reused_queue_name = None;
    if let Ok(printers) = super::cups_ffi::list_printers_via_cups() {
        for printer in printers {
            if let Some(uri) = printer.device_uri {
                if normalize_uri_for_compare(&uri) == device_uri_norm {
                    reused_queue_name = Some(printer.system_queue_name);
                    break;
                }
            }
        }
    }

    let has_reused = reused_queue_name.is_some();
    let queue_name = reused_queue_name.unwrap_or_else(|| build_queue_name(&name, &target.uri));
    if has_reused {
        eprintln!(
            "[InstallPrinterMacOS] reuse_queue_name=\"{}\" device_uri_norm=\"{}\"",
            queue_name, device_uri_norm
        );
    }
    eprintln!(
        "[InstallPrinterMacOS] queue_name=\"{}\" device_uri=\"{}\"",
        queue_name, target.uri
    );

    let init_meta = build_meta(
        &queue_name,
        &target.host,
        Some(&target.uri),
        &requested_mode_lower,
        &effective_mode,
    );
    let mut reporter = StepReporter::new(
        app.clone(),
        job_id.clone(),
        name.clone(),
        Some(effective_mode.clone()),
        Some(init_meta.clone()),
    );

    let init_message = if requested_mode_lower != "driverless" {
        format!(
            "macOS 平台已将 installMode=\"{}\" 降级为 \"{}\"",
            requested_mode_lower, effective_mode
        )
    } else {
        "开始安装 (macOS driverless)".to_string()
    };
    if let Err(err) = reporter.emit_job_init(&init_message) {
        eprintln!("[InstallPrinterMacOS] job.init emit failed: {}", err);
    }

    if let Err(err) = reporter.emit_step_running("device.probe", "检测打印机可用性", None) {
        eprintln!("[InstallPrinterMacOS] device.probe running emit failed: {}", err);
    }

    let probe_result = if dry_run {
        Ok(())
    } else {
        probe_device(&target.host, target.port)
    };

    if let Err(err) = probe_result {
        let message = format!("无法连接到 {}:{} ({})", target.host, target.port, err);
        let _ = reporter.emit_step_fail("device.probe", "E_PROBE_FAILED", &message, Some(&err));
        let _ = reporter.emit_job_failed("E_PROBE_FAILED", "探测失败", Some(&err));
        let _ = reporter.emit_job_done(false, Some("探测失败"));

        return Ok(crate::InstallResult {
            success: false,
            message: "设备探测失败".into(),
            method: Some("driverless".into()),
            stdout: None,
            stderr: None,
            effective_dry_run: dry_run,
            job_id,
        });
    }

    let probe_success_message = format!("探测成功，使用 URI: {}", target.uri);
    if let Err(err) = reporter.emit_step_success("device.probe", Some(&probe_success_message)) {
        eprintln!("[InstallPrinterMacOS] device.probe success emit failed: {}", err);
    }

    reporter.set_default_meta(Some(build_meta(
        &queue_name,
        &target.host,
        Some(&target.uri),
        &requested_mode_lower,
        &effective_mode,
    )));

    if let Err(err) = reporter.emit_step_running("device.ensureQueue", "正在创建打印队列 (需要管理员权限)", None) {
        eprintln!("[InstallPrinterMacOS] device.ensureQueue running emit failed: {}", err);
    }

    let ensure_result = if dry_run {
        Ok(())
    } else {
        ensure_queue(&queue_name, &target.uri, &name).await
    };

    if let Err(err) = ensure_result {
        let message = match err.code {
            "E_NO_PRIVILEGE" => "创建队列失败：需要管理员权限".to_string(),
            _ => "创建队列失败".to_string(),
        };
        let _ = reporter.emit_step_fail("device.ensureQueue", err.code, &message, Some(&err.detail));
        let _ = reporter.emit_job_failed(err.code, &message, Some(&err.detail));
        let _ = reporter.emit_job_done(false, Some(&message));

        return Ok(crate::InstallResult {
            success: false,
            message,
            method: Some("driverless".into()),
            stdout: None,
            stderr: None,
            effective_dry_run: dry_run,
            job_id,
        });
    }

    let ensure_message = format!("队列已创建/更新：{}", queue_name);
    if let Err(err) = reporter.emit_step_success("device.ensureQueue", Some(&ensure_message)) {
        eprintln!("[InstallPrinterMacOS] device.ensureQueue success emit failed: {}", err);
    }

    if let Err(err) = reporter.emit_step_running("device.finalVerify", "正在验证队列可用性", None) {
        eprintln!("[InstallPrinterMacOS] device.finalVerify running emit failed: {}", err);
    }

    eprintln!(
        "[InstallPrinterMacOS] finalVerify start queue_name=\"{}\" device_uri=\"{}\" cmd=\"lpoptions\" args=[\"-p\",\"{}\"]",
        queue_name,
        target.uri,
        queue_name
    );

    let deadline = Instant::now() + Duration::from_secs(FINAL_VERIFY_TIMEOUT_SECS);
    let mut last_error: Option<StepError> = None;
    let mut attempt: u32 = 0;
    loop {
        attempt += 1;
        if dry_run {
            let _ = reporter.emit_step_success("device.finalVerify", Some("队列验证通过"));
            let _ = reporter.emit_job_done(true, Some("安装完成"));

            return Ok(crate::InstallResult {
                success: true,
                message: "安装完成".into(),
                method: Some("driverless".into()),
                stdout: None,
                stderr: None,
                effective_dry_run: dry_run,
                job_id,
            });
        }

        match verify_queue_once(&queue_name, attempt).await {
            VerifyStatus::Exists(actual_uri) => {
                if let Some(actual_uri) = actual_uri {
                    if normalize_uri_for_compare(&actual_uri) != normalize_uri_for_compare(&target.uri) {
                        eprintln!(
                            "[InstallPrinterMacOS] finalVerify warn: uri mismatch queue_name=\"{}\" expected=\"{}\" actual=\"{}\"",
                            queue_name,
                            target.uri,
                            actual_uri
                        );
                    }
                }

                let _ = reporter.emit_step_success("device.finalVerify", Some("队列验证通过"));
                let _ = reporter.emit_job_done(true, Some("安装完成"));

                return Ok(crate::InstallResult {
                    success: true,
                    message: "安装完成".into(),
                    method: Some("driverless".into()),
                    stdout: None,
                    stderr: None,
                    effective_dry_run: dry_run,
                    job_id,
                });
            }
            VerifyStatus::NotFound(detail) => {
                last_error = Some(StepError {
                    code: "E_VERIFY_NOT_FOUND",
                    detail,
                });
                if Instant::now() >= deadline {
                    break;
                }
                sleep(TokioDuration::from_millis(FINAL_VERIFY_RETRY_MS)).await;
            }
            VerifyStatus::Error(err) => {
                last_error = Some(StepError {
                    code: "E_VERIFY_CMD_FAILED",
                    detail: err.detail,
                });
                break;
            }
        }
    }

    let fallback_error = StepError {
        code: "E_VERIFY_TIMEOUT",
        detail: format!("queue_name={} 队列验证超时", queue_name),
    };
    let final_error = last_error.unwrap_or(fallback_error);
    let message = if final_error.code == "E_VERIFY_TIMEOUT" {
        format!("队列验证超时（{}s）", FINAL_VERIFY_TIMEOUT_SECS)
    } else {
        "队列校验失败".to_string()
    };
    let _ = reporter.emit_step_fail("device.finalVerify", final_error.code, &message, Some(&final_error.detail));
    let _ = reporter.emit_job_failed(final_error.code, &message, Some(&final_error.detail));
    let _ = reporter.emit_job_done(false, Some(&message));

    Ok(crate::InstallResult {
        success: false,
        message,
        method: Some("driverless".into()),
        stdout: None,
        stderr: None,
        effective_dry_run: dry_run,
        job_id,
    })
}
