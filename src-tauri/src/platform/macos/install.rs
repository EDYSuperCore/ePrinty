use serde_json::{json, Value};
use std::{net::{IpAddr, SocketAddr, TcpStream}, time::Duration};
use std::process::Command;
use tauri::{AppHandle, Manager};

/// Generate a monotonic-ish timestamp in milliseconds
fn now_ts_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Sanitize queue name: only allow letters, digits, underscore and hyphen; replace others with '_'
fn sanitize_queue_name(input: &str) -> String {
    let mut result = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            result.push(ch);
        } else {
            result.push('_');
        }
    }
    let trimmed = result.trim_matches('_');
    if trimmed.is_empty() {
        // fallback to a deterministic but safe name
        format!("queue_{}", now_ts_ms() % 10000)
    } else {
        trimmed.to_string()
    }
}

/// Basic IP parser that accepts raw IP or path-like inputs such as "\\\\10.0.0.1" or "10.0.0.1/ipp/print"
fn parse_ip(input: &str) -> Result<IpAddr, String> {
    if input.trim().is_empty() {
        return Err("IP 地址为空".to_string());
    }

    // strip leading backslashes or slashes
    let mut cleaned = input.trim().trim_start_matches('\\').trim_start_matches('/').to_string();

    // strip common prefixes
    for prefix in ["ipp://", "http://", "https://", "ipp:", "http:", "https:"] {
        if cleaned.starts_with(prefix) {
            cleaned = cleaned[prefix.len()..].to_string();
            break;
        }
    }

    // take substring before first slash
    if let Some(pos) = cleaned.find('/') {
        cleaned = cleaned[..pos].to_string();
    }
    // take substring before first colon (port)
    if let Some(pos) = cleaned.find(':') {
        cleaned = cleaned[..pos].to_string();
    }

    cleaned
        .parse::<IpAddr>()
        .map_err(|_| format!("无法解析 IP 地址: {}", input))
}

fn emit_step_event(
    app: &AppHandle,
    job_id: &str,
    printer_name: &str,
    step_id: &str,
    state: &str,
    message: &str,
    error: Option<crate::ErrorPayload>,
    meta: Option<Value>,
) {
    let event = crate::InstallProgressEvent {
        job_id: job_id.to_string(),
        printer_name: printer_name.to_string(),
        step_id: step_id.to_string(),
        state: state.to_string(),
        message: message.to_string(),
        ts_ms: now_ts_ms(),
        progress: None,
        error,
        meta,
        legacy_phase: None,
    };
    let _ = app.emit_all("install_progress", &event);
}

fn emit_job_done(
    app: &AppHandle,
    job_id: &str,
    printer_name: &str,
    success: bool,
    message: &str,
    error: Option<crate::ErrorPayload>,
) {
    emit_step_event(
        app,
        job_id,
        printer_name,
        "job.done",
        if success { "success" } else { "failed" },
        message,
        error,
        None,
    );
}

fn build_meta(queue_name: &str, ip: &str, uri: Option<&str>) -> Value {
    let mut meta = serde_json::Map::new();
    meta.insert("platform".into(), Value::String("macos".into()));
    meta.insert("installMode".into(), Value::String("driverless".into()));
    meta.insert("queueName".into(), Value::String(queue_name.to_string()));
    meta.insert("ip".into(), Value::String(ip.to_string()));
    if let Some(uri_str) = uri {
        meta.insert("uri".into(), Value::String(uri_str.to_string()));
    }
    Value::Object(meta)
}

fn probe_device(ip: &IpAddr) -> Result<(), String> {
    let socket = SocketAddr::new(*ip, 631);
    TcpStream::connect_timeout(&socket, Duration::from_secs(3))
        .map(|_| ())
        .map_err(|e| format!("无法连接到 {}:631 ({})", ip, e))
}

fn escape_for_applescript(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn run_lpadmin_with_osascript(queue: &str, uri: &str) -> Result<(), String> {
    let shell_cmd = format!("lpadmin -p '{}' -E -v '{}' -m everywhere", queue, uri);
    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        escape_for_applescript(&shell_cmd)
    );

    let output = Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .output()
        .map_err(|e| format!("无法执行 osascript: {}", e))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    if stderr.to_lowercase().contains("user canceled") || stderr.to_lowercase().contains("用户取消") {
        return Err("USER_CANCELED".to_string());
    }

    Err(format!(
        "lpadmin 执行失败。stdout: {} stderr: {}",
        stdout.trim(),
        stderr.trim()
    ))
}

fn verify_queue(queue: &str, ip: &str) -> Result<(), String> {
    let status_output = Command::new("lpstat")
        .args(["-p", queue])
        .output()
        .map_err(|e| format!("执行 lpstat -p 失败: {}", e))?;

    if !status_output.status.success() {
        let stderr = String::from_utf8_lossy(&status_output.stderr);
        return Err(format!("队列状态不可用: {}", stderr.trim()));
    }

    let status_stdout = String::from_utf8_lossy(&status_output.stdout).to_lowercase();
    if status_stdout.contains("disabled") {
        return Err("队列已创建但处于禁用状态".to_string());
    }

    let uri_output = Command::new("lpstat")
        .args(["-v", queue])
        .output()
        .map_err(|e| format!("执行 lpstat -v 失败: {}", e))?;

    if !uri_output.status.success() {
        let stderr = String::from_utf8_lossy(&uri_output.stderr);
        return Err(format!("无法获取设备 URI: {}", stderr.trim()));
    }

    let uri_stdout = String::from_utf8_lossy(&uri_output.stdout).to_lowercase();
    if !uri_stdout.contains(ip.to_lowercase().as_str()) {
        return Err(format!("队列 URI 与目标 IP 不匹配: {}", uri_stdout.trim()));
    }

    Ok(())
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

    let queue_name = sanitize_queue_name(&name);
    let ip_addr = parse_ip(&path).or_else(|_| parse_ip(&name))?;
    let ip_str = ip_addr.to_string();

    let primary_uri = format!("ipp://{}/ipp/print", ip_str);
    let fallback_uri = format!("ipp://{}:631/ipp/print", ip_str);

    // job.init
    let init_meta = build_meta(&queue_name, &ip_str, Some(&primary_uri));
    emit_step_event(
        &app,
        &job_id,
        &name,
        "job.init",
        "running",
        "开始安装 (macOS driverless)",
        None,
        Some(init_meta.clone()),
    );

    // step: device.probe
    emit_step_event(
        &app,
        &job_id,
        &name,
        "device.probe",
        "running",
        "检测打印机可用性",
        None,
        Some(init_meta.clone()),
    );

    let probe_result = if dry_run {
        Ok(())
    } else {
        probe_device(&ip_addr)
    };

    if let Err(err) = probe_result {
        emit_step_event(
            &app,
            &job_id,
            &name,
            "device.probe",
            "failed",
            &format!("无法连接到 {}: {}", ip_str, err),
            Some(crate::ErrorPayload {
                code: "PROBE_FAILED".into(),
                detail: err.clone(),
                stderr: None,
                stdout: None,
            }),
            Some(init_meta.clone()),
        );
        emit_job_done(
            &app,
            &job_id,
            &name,
            false,
            &format!("探测失败: {}", err),
            Some(crate::ErrorPayload {
                code: "PROBE_FAILED".into(),
                detail: err,
                stderr: None,
                stdout: None,
            }),
        );

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

    emit_step_event(
        &app,
        &job_id,
        &name,
        "device.probe",
        "success",
        "打印机可达，准备创建队列",
        None,
        Some(init_meta.clone()),
    );

    // step: device.ensureQueue
    emit_step_event(
        &app,
        &job_id,
        &name,
        "device.ensureQueue",
        "running",
        "正在创建打印队列 (需要管理员权限)",
        None,
        Some(init_meta.clone()),
    );

    let ensure_result = if dry_run {
        Ok(primary_uri.clone())
    } else {
        match run_lpadmin_with_osascript(&queue_name, &primary_uri) {
            Ok(_) => Ok(primary_uri.clone()),
            Err(e) if e == "USER_CANCELED" => Err(e),
            Err(err_primary) => {
                // fallback to :631
                match run_lpadmin_with_osascript(&queue_name, &fallback_uri) {
                    Ok(_) => Ok(fallback_uri.clone()),
                    Err(err_fallback) => Err(format!("{}; fallback: {}", err_primary, err_fallback)),
                }
            }
        }
    };

    match ensure_result {
        Ok(uri_used) => {
            emit_step_event(
                &app,
                &job_id,
                &name,
                "device.ensureQueue",
                "success",
                "打印队列创建/更新成功",
                None,
                Some(build_meta(&queue_name, &ip_str, Some(&uri_used))),
            );
        }
        Err(err) if err == "USER_CANCELED" => {
            emit_step_event(
                &app,
                &job_id,
                &name,
                "device.ensureQueue",
                "failed",
                "用户取消授权，未创建队列",
                Some(crate::ErrorPayload {
                    code: "USER_CANCELED".into(),
                    detail: "用户取消授权，未执行 lpadmin".into(),
                    stderr: None,
                    stdout: None,
                }),
                Some(init_meta.clone()),
            );
            emit_job_done(
                &app,
                &job_id,
                &name,
                false,
                "用户取消授权",
                Some(crate::ErrorPayload {
                    code: "USER_CANCELED".into(),
                    detail: "用户取消授权，未创建队列".into(),
                    stderr: None,
                    stdout: None,
                }),
            );

            return Ok(crate::InstallResult {
                success: false,
                message: "用户取消授权".into(),
                method: Some("driverless".into()),
                stdout: None,
                stderr: None,
                effective_dry_run: dry_run,
                job_id,
            });
        }
        Err(err) => {
            emit_step_event(
                &app,
                &job_id,
                &name,
                "device.ensureQueue",
                "failed",
                &format!("创建队列失败: {}", err),
                Some(crate::ErrorPayload {
                    code: "QUEUE_CREATE_FAILED".into(),
                    detail: err.clone(),
                    stderr: None,
                    stdout: None,
                }),
                Some(init_meta.clone()),
            );
            emit_job_done(
                &app,
                &job_id,
                &name,
                false,
                &format!("创建队列失败: {}", err),
                Some(crate::ErrorPayload {
                    code: "QUEUE_CREATE_FAILED".into(),
                    detail: err,
                    stderr: None,
                    stdout: None,
                }),
            );

            return Ok(crate::InstallResult {
                success: false,
                message: "创建队列失败".into(),
                method: Some("driverless".into()),
                stdout: None,
                stderr: None,
                effective_dry_run: dry_run,
                job_id,
            });
        }
    }

    // step: device.finalVerify
    emit_step_event(
        &app,
        &job_id,
        &name,
        "device.finalVerify",
        "running",
        "正在验证队列可用性",
        None,
        Some(init_meta.clone()),
    );

    let verify_result = if dry_run {
        Ok(())
    } else {
        verify_queue(&queue_name, &ip_str)
    };

    match verify_result {
        Ok(_) => {
            emit_step_event(
                &app,
                &job_id,
                &name,
                "device.finalVerify",
                "success",
                "队列验证通过",
                None,
                Some(init_meta.clone()),
            );

            emit_job_done(&app, &job_id, &name, true, "安装完成", None);

            Ok(crate::InstallResult {
                success: true,
                message: "安装完成".into(),
                method: Some("driverless".into()),
                stdout: None,
                stderr: None,
                effective_dry_run: dry_run,
                job_id,
            })
        }
        Err(err) => {
            emit_step_event(
                &app,
                &job_id,
                &name,
                "device.finalVerify",
                "failed",
                &format!("队列不可用: {}", err),
                Some(crate::ErrorPayload {
                    code: "FINAL_VERIFY_FAILED".into(),
                    detail: err.clone(),
                    stderr: None,
                    stdout: None,
                }),
                Some(init_meta.clone()),
            );
            emit_job_done(
                &app,
                &job_id,
                &name,
                false,
                &format!("队列校验失败: {}", err),
                Some(crate::ErrorPayload {
                    code: "FINAL_VERIFY_FAILED".into(),
                    detail: err,
                    stderr: None,
                    stdout: None,
                }),
            );

            Ok(crate::InstallResult {
                success: false,
                message: "队列创建失败或不可用".into(),
                method: Some("driverless".into()),
                stdout: None,
                stderr: None,
                effective_dry_run: dry_run,
                job_id,
            })
        }
    }
}
