use std::fs;
use std::io::Write;
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::json;
use tauri::{AppHandle, Manager};

use crate::platform::test_page_content::build_test_page_content;

fn stderr_snip(stderr: &str) -> String {
    let snip = if stderr.len() > 200 {
        &stderr[..200]
    } else {
        stderr
    };
    snip.replace('\n', " ").trim().to_string()
}

fn parse_job_id(stdout: &str) -> Option<String> {
    for line in stdout.lines() {
        let lower = line.to_lowercase();
        if let Some(pos) = lower.find("request id is") {
            let after = &line[pos + "request id is".len()..].trim();
            if let Some(job_id) = after.split_whitespace().next() {
                if !job_id.is_empty() {
                    return Some(job_id.to_string());
                }
            }
        }
    }
    None
}

fn emit_print_progress(
    app: &AppHandle,
    job_id: &str,
    printer_name: &str,
    step_id: &str,
    state: &str,
    message: &str,
) {
    let payload = json!({
        "jobId": job_id,
        "printerName": printer_name,
        "stepId": step_id,
        "state": state,
        "message": message,
        "tsMs": chrono::Utc::now().timestamp_millis(),
        "platform": "macos",
    });
    let _ = app.emit_all("print_progress", payload);
}

pub fn print_test_page_macos(app: AppHandle, printer_name: String) -> Result<String, String> {
    eprintln!("[PrintTestPage] START printer_name=\"{}\"", printer_name);
    let job_id = format!("print_{}_{}", chrono::Utc::now().timestamp_millis(), std::process::id());

    let now = chrono::Local::now();
    emit_print_progress(&app, &job_id, &printer_name, "print.prepare", "running", "准备测试页内容");
    let content = build_test_page_content(
        &printer_name,
        &now.format("%Y年%m月%d日 %H:%M:%S").to_string(),
    );
    emit_print_progress(&app, &job_id, &printer_name, "print.prepare", "success", "测试页内容已生成");

    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("eprinty_testpage_{}_{}.txt", std::process::id(), now.timestamp()));
    let temp_file_display = temp_file.to_string_lossy().to_string();

    {
        let mut file = fs::File::create(&temp_file)
            .map_err(|e| format!("[PrintTestPage] ERROR step=TEMP_FILE_CREATE message=创建临时文件失败: {}", e))?;
        file.write_all(content.as_bytes())
            .map_err(|e| format!("[PrintTestPage] ERROR step=TEMP_FILE_CREATE message=写入测试内容失败: {}", e))?;
        file.sync_all()
            .map_err(|e| format!("[PrintTestPage] ERROR step=TEMP_FILE_CREATE message=同步文件失败: {}", e))?;
    }

    eprintln!("[PrintTestPage] TEMP_FILE_CREATE path=\"{}\"", temp_file_display);

    emit_print_progress(&app, &job_id, &printer_name, "print.submit", "running", "提交打印任务");
    let output = Command::new("/usr/bin/lp")
        .args(["-d", &printer_name, temp_file.to_str().unwrap_or_default()])
        .output()
        .map_err(|e| format!("[PrintTestPage] ERROR step=LP_EXEC message=执行 lp 失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    eprintln!(
        "[PrintTestPage] EXEC result exit_code={:?} stdout_len={} stderr_len={}",
        output.status.code(),
        stdout.len(),
        stderr.len()
    );

    let _ = fs::remove_file(&temp_file);

    if output.status.success() {
        emit_print_progress(&app, &job_id, &printer_name, "print.submit", "success", "打印任务已提交");
        let submitted_job_id = parse_job_id(&stdout);
        emit_print_progress(&app, &job_id, &printer_name, "print.monitor", "running", "等待打印完成");

        let deadline = Instant::now() + Duration::from_secs(20);
        let mut completed = false;
        if let Some(submitted_job_id) = &submitted_job_id {
            while Instant::now() < deadline {
                let output = Command::new("/usr/bin/lpstat")
                    .args(["-W", "not-completed", "-o"])
                    .output();
                if let Ok(out) = output {
                    if out.status.success() {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        if !stdout.contains(submitted_job_id) {
                            completed = true;
                            break;
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        }

        emit_print_progress(&app, &job_id, &printer_name, "print.monitor", "success", "打印任务已完成");
        emit_print_progress(&app, &job_id, &printer_name, "print.done", "success", "测试页打印完成");

        if let Some(submitted_job_id) = submitted_job_id {
            eprintln!(
                "[PrintTestPage] SUCCESS printer_name=\"{}\" job_id=\"{}\" completed={}",
                printer_name, submitted_job_id, completed
            );
            return Ok(format!("测试页已提交: {} ({})", printer_name, submitted_job_id));
        }
        eprintln!("[PrintTestPage] SUCCESS printer_name=\"{}\"", printer_name);
        return Ok(format!("测试页已发送到打印机: {}", printer_name));
    }

    emit_print_progress(&app, &job_id, &printer_name, "print.failed", "failed", "打印任务提交失败");
    Err(format!(
        "[PrintTestPage] ERROR step=LP_RESULT message=打印测试页失败 exit_code={:?} stderr={}",
        output.status.code(),
        stderr_snip(&stderr)
    ))
}
