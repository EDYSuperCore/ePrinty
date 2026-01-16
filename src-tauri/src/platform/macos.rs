use std::collections::HashSet;
use std::process::Command;

pub mod install;
pub mod test_page;
pub mod ipp_printers;
pub mod cups_ffi;
pub mod delete;

/// macOS 平台打开 URL
/// 
/// 使用 `open` 命令打开 URL
pub fn open_url_macos(url: &str) -> Result<(), String> {
    let output = Command::new("open")
        .arg(url)
        .output()
        .map_err(|e| format!("执行命令失败: {}", e))?;
    
    if output.status.success() {
        Ok(())
    } else {
        let error = String::from_utf8_lossy(&output.stderr);
        Err(format!("无法打开 URL: {}", error))
    }
}

#[derive(Debug, Clone)]
pub struct Destination {
    pub name: String,
    pub device_uri: Option<String>,
    pub display_name: Option<String>,
    pub is_accepting_jobs: Option<bool>,
    pub state: Option<i32>,
}

enum LpstatResult {
    Data(String),
    Empty,
    Error(String),
}

fn stderr_snip(stderr: &str) -> String {
    let snip = if stderr.len() > 200 {
        &stderr[..200]
    } else {
        stderr
    };
    snip.replace('\n', " ").trim().to_string()
}

fn is_empty_destinations(text: &str) -> bool {
    let lower = text.trim().to_lowercase();
    let en_patterns = ["no destinations", "no printers"];
    if en_patterns.iter().any(|p| lower.contains(p)) {
        return true;
    }
    if lower.contains("not added") && lower.contains("destination") {
        return true;
    }
    let zh_patterns = [
        "未添加目的位置",
        "未添加目的地",
        "没有添加目的位置",
        "未添加目标位置",
        "未添加打印机",
        "没有打印机",
        "没有可用打印机",
    ];
    zh_patterns.iter().any(|p| text.contains(p))
}

fn classify_lpstat_result(cmd: &str, args: &[&str], exit_code: i32, stdout: &str, stderr: &str) -> LpstatResult {
    if exit_code == 0 {
        return LpstatResult::Data(stdout.to_string());
    }
    if is_empty_destinations(stderr) || is_empty_destinations(stdout) {
        return LpstatResult::Empty;
    }
    LpstatResult::Error(format!(
        "cmd={} args={:?} exit_code={} stderr_snip=\"{}\"",
        cmd,
        args,
        exit_code,
        stderr_snip(stderr)
    ))
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

fn parse_lpstat_description(output: &str) -> Option<String> {
    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Description:") {
            return Some(trimmed.trim_start_matches("Description:").trim().to_string());
        }
    }
    None
}

fn is_valid_queue_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
}

fn parse_queue_names_from_lpstat_v(stdout: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in stdout.lines() {
        let trimmed = line.trim();
        if let Some(pos) = trimmed.find("device for ") {
            let after = &trimmed[pos + "device for ".len()..];
            if let Some(colon_pos) = after.find(':') {
                let name = after[..colon_pos].trim();
                if !name.is_empty() {
                    names.push(name.to_string());
                }
            }
        }
    }
    names
}

pub fn macos_list_queue_names() -> Result<Vec<String>, String> {
    let output = Command::new("/usr/bin/lpstat")
        .arg("-v")
        .output()
        .map_err(|e| format!("cmd=lpstat args=[\"-v\"] error={}", e))?;

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    match classify_lpstat_result("/usr/bin/lpstat", &["-v"], code, &stdout, &stderr) {
        LpstatResult::Data(data) => {
            let mut unique = HashSet::new();
            let mut names = Vec::new();
            for name in parse_queue_names_from_lpstat_v(&data) {
                if !is_valid_queue_name(&name) {
                    eprintln!(
                        "[MacOS][ListPrinters] WARN invalid queue name from lpstat -v: \"{}\"",
                        name
                    );
                    continue;
                }
                if unique.insert(name.clone()) {
                    names.push(name);
                }
            }
            Ok(names)
        }
        LpstatResult::Empty => Ok(Vec::new()),
        LpstatResult::Error(err) => Err(err),
    }
}

pub fn list_destinations() -> Result<Vec<Destination>, String> {
    match cups_ffi::list_printers_via_cups() {
        Ok(printers) => {
            let samples = printers
                .iter()
                .take(2)
                .map(|p| {
                    let display = p.display_name.as_deref().unwrap_or("");
                    let uri = p.device_uri.as_deref().unwrap_or("");
                    format!("{}|{}|{}", p.system_queue_name, display, uri)
                })
                .collect::<Vec<_>>();
            eprintln!(
                "[MacOS][ListPrinters] cups_success count={} sample={:?}",
                printers.len(),
                samples
            );
            return Ok(printers
                .into_iter()
                .map(|p| Destination {
                    name: p.system_queue_name,
                    device_uri: p.device_uri,
                    display_name: p.display_name,
                    is_accepting_jobs: p.is_accepting_jobs,
                    state: p.state,
                })
                .collect());
        }
        Err(err) => {
            eprintln!("[MacOS][ListPrinters] cups_failed fallback=lpstat error=\"{}\"", err);
        }
    }

    let names = macos_list_queue_names()?;
    if names.is_empty() {
        return Ok(Vec::new());
    }

    let mut destinations: Vec<Destination> = names
        .into_iter()
        .map(|name| Destination {
            name,
            device_uri: None,
            display_name: None,
            is_accepting_jobs: None,
            state: None,
        })
        .collect();

    for dest in destinations.iter_mut() {
        let output = Command::new("/usr/bin/lpstat")
            .args(["-v", &dest.name])
            .output();
        match output {
            Ok(out) => {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    dest.device_uri = parse_lpstat_device_uri(&dest.name, &stdout);
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    eprintln!(
                        "[MacOS][ListPrinters] WARN cmd=lpstat args=[\"-v\",\"{}\"] exit_code={} stderr_snip=\"{}\"",
                        dest.name,
                        out.status.code().unwrap_or(-1),
                        stderr_snip(&stderr)
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "[MacOS][ListPrinters] WARN cmd=lpstat args=[\"-v\",\"{}\"] error={}",
                    dest.name, e
                );
            }
        }
    }

    for dest in destinations.iter_mut() {
        let output = Command::new("/usr/bin/lpstat")
            .args(["-l", "-p", &dest.name])
            .output();
        match output {
            Ok(out) => {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                    dest.display_name = parse_lpstat_description(&stdout);
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    eprintln!(
                        "[MacOS][ListPrinters] WARN cmd=lpstat args=[\"-l\",\"-p\",\"{}\"] exit_code={} stderr_snip=\"{}\"",
                        dest.name,
                        out.status.code().unwrap_or(-1),
                        stderr_snip(&stderr)
                    );
                }
            }
            Err(e) => {
                eprintln!(
                    "[MacOS][ListPrinters] WARN cmd=lpstat args=[\"-l\",\"-p\",\"{}\"] error={}",
                    dest.name, e
                );
            }
        }
    }

    Ok(destinations)
}

/// macOS 平台获取打印机列表（稳定实现）
/// 
/// 主方案：lpstat 绝对路径 + 两条命令 (lpstat -p 和 lpstat -v)
/// 兜底方案：system_profiler（仅当主方案 ERROR 时触发，放在后台线程）
/// 
/// 错误分类：
/// - EMPTY: "no destinations" 或 "no printers"（系统无打印机）- 多语言支持
/// - MAC_CUPS_NOT_RUNNING: "scheduler is not running" 等 CUPS 问题
/// - MAC_LPSTAT_EXEC_FAIL: 其他 lpstat 执行失败
pub fn list_printers_macos() -> Result<Vec<String>, String> {
    let destinations = list_destinations()?;
    Ok(destinations.into_iter().map(|d| d.name).collect())
}
