// Windows 平台打印机查找和重装模块
// 提供查找已安装打印机和重新安装打印机的功能（不提供删除能力）

use super::log;
use super::cmd;

/// 已安装的打印机信息（包含完整信息用于匹配）
#[derive(Debug, Clone)]
pub struct InstalledPrinter {
    pub name: String,
    pub port_name: Option<String>,
    pub driver_name: Option<String>,
    pub comment: Option<String>,
    pub location: Option<String>,
}

/// 匹配方式
#[derive(Debug, Clone)]
pub enum MatchMethod {
    EPrintyTag,  // 通过 ePrinty tag 匹配
    PortName,    // 通过端口名匹配
    NameExact,   // 通过名称精确匹配
}

/// 生成 stable_id（与 install.rs 中的逻辑保持一致）
fn generate_stable_id_for_config(printer_name: &str, printer_path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    // 从配置中查找匹配的 printer，获取 area.name
    match crate::load_local_config() {
        Ok((config, _)) => {
            // 在所有 areas 中查找匹配的 printer
            for city in &config.cities {
                for area in &city.areas {
                    for printer in &area.printers {
                        if printer.name == printer_name || printer.path == printer_path {
                            // 生成 hash
                            let ip = printer_path.trim_start_matches("\\\\").trim_start_matches("\\").to_string();
                            let hash_input = format!("{}|{}|{}", area.area_name, printer.name, ip);
                            
                            let mut hasher = DefaultHasher::new();
                            hash_input.hash(&mut hasher);
                            let hash = hasher.finish();
                            
                            return format!("{:x}", hash);
                        }
                    }
                }
            }
        }
        Err(_) => {
            // 配置加载失败，使用默认方式生成
        }
    }
    
    // 如果找不到配置，使用 name + path 生成
    let ip = printer_path.trim_start_matches("\\\\").trim_start_matches("\\").to_string();
    let hash_input = format!("{}|{}", printer_name, ip);
    let mut hasher = DefaultHasher::new();
    hash_input.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{:x}", hash)
}

/// 查找已安装的打印机（根据配置打印机信息）
/// 
/// 匹配优先级：
/// 1. ePrinty tag 匹配（comment/location 包含 "ePrinty:{stable_id}" 或包含 "ip={ip}"）
/// 2. port_name 或 comment/location 中包含 ip={ip}（次匹配）
/// 3. name 精确匹配（仅当系统中只有 1 个同名队列时允许）
pub fn find_installed_printer_for_config(
    config_printer_name: &str,
    config_printer_path: &str,
) -> Result<Option<(InstalledPrinter, MatchMethod)>, String> {
    log::write_log(&format!("[FindPrinter] 开始查找配置打印机: name={}, path={}", config_printer_name, config_printer_path));

    // 枚举所有已安装的打印机（使用 Level 2 获取完整信息）
    let installed_printers = match enum_printers_level_2() {
        Ok(printers) => printers,
        Err(e) => {
            log::write_log(&format!("[FindPrinter] 枚举打印机失败: {}", e));
            return Err(format!("枚举打印机失败: {}", e));
        }
    };

    log::write_log(&format!("[FindPrinter] 枚举到 {} 台打印机", installed_printers.len()));

    // 提取 IP 地址（从 path 中，例如 "\\\\192.168.1.100" 或 "192.168.1.100"）
    let ip = extract_ip_from_path(config_printer_path);
    
    // 生成 stable_id
    let stable_id = generate_stable_id_for_config(config_printer_name, config_printer_path);
    log::write_log(&format!("[FindPrinter] 生成的 stable_id: {}", stable_id));

    // 优先级 1: 通过 ePrinty tag 匹配（强匹配）
    // 检查 comment 或 location 包含 ePrinty:{stable_id}
    let tag_pattern = format!("ePrinty:{}", stable_id);
    for printer in &installed_printers {
        if let Some(tag) = extract_eprinty_tag(&printer.comment, &printer.location) {
            if tag.contains(&tag_pattern) {
                log::write_log(&format!("[FindPrinter] 通过 ePrinty tag 匹配（stable_id）: name={}, tag={}", printer.name, tag));
                return Ok(Some((printer.clone(), MatchMethod::EPrintyTag)));
            }
        }
    }
    
    // 优先级 1.5: 通过 ePrinty tag 中的 ip=xxx 匹配（如果 stable_id 匹配失败）
    if let Some(ip) = &ip {
        let ip_pattern = format!("ip={}", ip);
        for printer in &installed_printers {
            if let Some(tag) = extract_eprinty_tag(&printer.comment, &printer.location) {
                if tag.contains(&ip_pattern) {
                    log::write_log(&format!("[FindPrinter] 通过 ePrinty tag 匹配（ip）: name={}, tag={}", printer.name, tag));
                    return Ok(Some((printer.clone(), MatchMethod::EPrintyTag)));
                }
            }
        }
    }

    // 优先级 2: 通过端口名/IP 匹配或 comment/location 中包含 ip={ip}
    if let Some(ip) = &ip {
        let ip_pattern = format!("ip={}", ip);
        for printer in &installed_printers {
            // 检查端口名
            if let Some(port) = &printer.port_name {
                if port.contains(ip) {
                    log::write_log(&format!("[FindPrinter] 通过端口名匹配: name={}, port={}", printer.name, port));
                    return Ok(Some((printer.clone(), MatchMethod::PortName)));
                }
            }
            
            // 检查 comment 或 location 中包含 ip=
            if let Some(comment) = &printer.comment {
                if comment.contains(&ip_pattern) {
                    log::write_log(&format!("[FindPrinter] 通过 comment 中的 ip 匹配: name={}, comment={}", printer.name, comment));
                    return Ok(Some((printer.clone(), MatchMethod::PortName)));
                }
            }
            
            if let Some(location) = &printer.location {
                if location.contains(&ip_pattern) {
                    log::write_log(&format!("[FindPrinter] 通过 location 中的 ip 匹配: name={}, location={}", printer.name, location));
                    return Ok(Some((printer.clone(), MatchMethod::PortName)));
                }
            }
        }
    }

    // 优先级 3: 通过名称精确匹配（仅当系统中只有 1 个同名队列时允许）
    let name_matches: Vec<_> = installed_printers.iter()
        .filter(|p| p.name == config_printer_name)
        .collect();
    
    if name_matches.len() == 1 {
        let printer = name_matches[0].clone();
        log::write_log(&format!("[FindPrinter] 通过名称精确匹配（系统中只有 1 个同名队列）: name={}", printer.name));
        return Ok(Some((printer, MatchMethod::NameExact)));
    } else if name_matches.len() > 1 {
        // 有多个同名队列，拒绝匹配
        log::write_log(&format!("[FindPrinter] 名称匹配失败：系统中有 {} 个同名队列，为避免误删已拒绝", name_matches.len()));
        return Err(format!("系统中有 {} 个名为 '{}' 的打印机队列，无法确定目标。请先安装打印机以添加 ePrinty tag。", name_matches.len(), config_printer_name));
    }

    // 统计信息（用于错误报告）
    let name_count = installed_printers.iter().filter(|p| p.name == config_printer_name).count();
    let ip_count = if let Some(ip) = &ip {
        installed_printers.iter().filter(|p| {
            p.port_name.as_ref().map(|port| port.contains(ip)).unwrap_or(false) ||
            p.comment.as_ref().map(|c| c.contains(&format!("ip={}", ip))).unwrap_or(false) ||
            p.location.as_ref().map(|l| l.contains(&format!("ip={}", ip))).unwrap_or(false)
        }).count()
    } else {
        0
    };
    let tag_count = installed_printers.iter().filter(|p| {
        extract_eprinty_tag(&p.comment, &p.location).is_some()
    }).count();
    
    log::write_log(&format!("[FindPrinter] 未找到匹配的打印机: name={}, path={}, stable_id={}", config_printer_name, config_printer_path, stable_id));
    log::write_log(&format!("[FindPrinter] 统计: 同名={}, 同IP={}, 有tag={}", name_count, ip_count, tag_count));
    
    Ok(None)
}

/// 使用 PowerShell Get-Printer 获取打印机完整信息（用于匹配）
/// 
/// 注意：此函数仅用于查找匹配，不用于删除操作
/// 由于 windows-rs 0.52 中 PRINTER_INFO_2W 可能不可用，使用 PowerShell 作为替代方案
fn enum_printers_level_2() -> Result<Vec<InstalledPrinter>, String> {
    log::write_log(&format!("[EnumPrinters] 使用 PowerShell Get-Printer 获取完整信息"));

    // 使用 PowerShell Get-Printer 命令获取所有打印机的完整信息
    let ps_command = r#"
        Get-Printer | ForEach-Object {
            $printer = $_
            [PSCustomObject]@{
                Name = $printer.Name
                PortName = $printer.PortName
                DriverName = $printer.DriverName
                Comment = if ($printer.Comment) { $printer.Comment } else { $null }
                Location = if ($printer.Location) { $printer.Location } else { $null }
            }
        } | ConvertTo-Json -Compress
    "#;

    match cmd::run_command("powershell.exe", &[
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        ps_command
    ]) {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("PowerShell Get-Printer 执行失败: {}", stderr));
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            
            // 解析 JSON 输出
            match serde_json::from_str::<Vec<serde_json::Value>>(&stdout) {
                Ok(json_array) => {
                    let mut printers = Vec::new();
                    for item in json_array {
                        let name = item["Name"].as_str().unwrap_or("").to_string();
                        let port_name = item["PortName"].as_str().map(|s| s.to_string());
                        let driver_name = item["DriverName"].as_str().map(|s| s.to_string());
                        let comment = item["Comment"].as_str().map(|s| s.to_string());
                        let location = item["Location"].as_str().map(|s| s.to_string());

                        printers.push(InstalledPrinter {
                            name,
                            port_name,
                            driver_name,
                            comment,
                            location,
                        });
                    }
                    Ok(printers)
                }
                Err(e) => {
                    // 如果解析失败，尝试解析单个对象（只有一个打印机的情况）
                    match serde_json::from_str::<serde_json::Value>(&stdout) {
                        Ok(json_obj) => {
                            let name = json_obj["Name"].as_str().unwrap_or("").to_string();
                            let port_name = json_obj["PortName"].as_str().map(|s| s.to_string());
                            let driver_name = json_obj["DriverName"].as_str().map(|s| s.to_string());
                            let comment = json_obj["Comment"].as_str().map(|s| s.to_string());
                            let location = json_obj["Location"].as_str().map(|s| s.to_string());

                            Ok(vec![InstalledPrinter {
                                name,
                                port_name,
                                driver_name,
                                comment,
                                location,
                            }])
                        }
                        Err(_) => {
                            Err(format!("解析 PowerShell 输出失败: {}, 原始输出: {}", e, stdout))
                        }
                    }
                }
            }
        }
        Err(e) => {
            Err(format!("执行 PowerShell Get-Printer 失败: {}", e))
        }
    }
}

/// 从路径中提取 IP 地址
fn extract_ip_from_path(path: &str) -> Option<String> {
    // 移除 "\\\\" 前缀（如果有）
    let path = path.trim_start_matches("\\\\");
    // 移除 "\\" 前缀（如果有）
    let path = path.trim_start_matches("\\");
    
    // 简单的 IP 地址匹配（IPv4）
    if let Some(ip) = path.split('/').next() {
        if ip.contains('.') && ip.split('.').count() == 4 {
            return Some(ip.to_string());
        }
    }
    
    None
}

/// 将 IP 地址转换为下划线格式（用于端口名匹配）
/// 例如：192.168.1.100 -> 192_168_1_100
fn ip_to_underscore(ip: &str) -> String {
    ip.replace('.', "_")
}

/// 解析目标打印机（简化版：按名称精确匹配，可选 IP/端口名过滤）
/// 
/// # 参数
/// - `config_name`: 配置打印机名称
/// - `ip`: 可选的 IP 地址（用于过滤）
/// 
/// # 返回
/// - `Ok(String)`: 找到目标打印机名称
/// - `Err(String)`: 未找到或匹配失败
/// 
/// # 匹配规则
/// 1. candidates = installed where name == config_name
/// 2. if ip provided:
///    - prefer candidates where port_name == "IP_<ip转下划线>" OR port_name contains ip
/// 3. if still multiple:
///    - choose first deterministically（保持排序稳定：按 name/port_name 字典序）
/// 4. if none: return Err("未找到同名打印机，无法删除/重装")
pub fn resolve_target_printer(config_name: String, ip: Option<String>) -> Result<String, String> {
    log::write_log(&format!("[ResolveTarget] START config_name={} ip={:?}", config_name, ip));
    
    // 使用 list_printers_detailed 获取已安装的打印机列表
    let installed_printers = match super::list::list_printers_detailed() {
        Ok(printers) => printers,
        Err(e) => {
            log::write_log(&format!("[ResolveTarget] 枚举打印机失败: {}", e));
            return Err(format!("枚举打印机失败: {}", e));
        }
    };
    
    // 第一步：按名称精确匹配
    let mut candidates: Vec<&super::list::DetailedPrinterInfo> = installed_printers.iter()
        .filter(|p| p.name == config_name)
        .collect();
    
    log::write_log(&format!("[ResolveTarget] 名称匹配候选数: {}", candidates.len()));
    
    // 输出所有候选者信息
    for (i, candidate) in candidates.iter().enumerate() {
        log::write_log(&format!(
            "[ResolveTarget] Candidate[{}]: name={} port={:?}",
            i, candidate.name, candidate.port_name
        ));
    }
    
    if candidates.is_empty() {
        log::write_log(&format!("[ResolveTarget] 未找到同名打印机: {}", config_name));
        return Err(format!("未找到同名打印机，无法删除/重装"));
    }
    
    // 第二步：如果提供了 IP，优先匹配端口名
    if let Some(ip) = &ip {
        let ip_underscore = format!("IP_{}", ip_to_underscore(ip));
        
        // 优先匹配：port_name == "IP_<ip转下划线>" 或 port_name contains ip
        let ip_matches: Vec<&super::list::DetailedPrinterInfo> = candidates.iter()
            .filter(|p| {
                if let Some(port) = &p.port_name {
                    port == &ip_underscore || port.contains(ip)
                } else {
                    false
                }
            })
            .copied()
            .collect();
        
        if !ip_matches.is_empty() {
            log::write_log(&format!("[ResolveTarget] IP 匹配候选数: {}", ip_matches.len()));
            candidates = ip_matches;
        } else {
            log::write_log(&format!("[ResolveTarget] IP 匹配无结果，使用所有名称匹配候选"));
        }
    }
    
    // 第三步：如果仍有多个候选，按确定性规则选择（按 name/port_name 字典序）
    if candidates.len() > 1 {
        candidates.sort_by(|a, b| {
            // 先按 name 排序
            match a.name.cmp(&b.name) {
                std::cmp::Ordering::Equal => {
                    // name 相同，按 port_name 排序（None 排在最后）
                    match (&a.port_name, &b.port_name) {
                        (Some(pa), Some(pb)) => pa.cmp(pb),
                        (Some(_), None) => std::cmp::Ordering::Less,
                        (None, Some(_)) => std::cmp::Ordering::Greater,
                        (None, None) => std::cmp::Ordering::Equal,
                    }
                }
                other => other,
            }
        });
        
        log::write_log(&format!("[ResolveTarget] 多个候选，排序后选择第一条"));
    }
    
    // 选择第一条（确定性）
    let selected = candidates[0];
    log::write_log(&format!(
        "[ResolveTarget] SELECTED: name={} port={:?}",
        selected.name, selected.port_name
    ));
    
    Ok(selected.name.clone())
}

/// 从 comment 或 location 中提取 ePrinty tag
fn extract_eprinty_tag(comment: &Option<String>, location: &Option<String>) -> Option<String> {
    // 检查 comment
    if let Some(comment) = comment {
        if comment.contains("ePrinty:") {
            return Some(comment.clone());
        }
    }
    
    // 检查 location
    if let Some(location) = location {
        if location.contains("ePrinty:") {
            return Some(location.clone());
        }
    }
    
    None
}

/// 将 Rust String 转换为 UTF-16 宽字符串（以 null 结尾）
/// 
/// # 参数
/// - `s`: 要转换的字符串
/// 
/// # 返回
/// - `Vec<u16>`: UTF-16 编码的宽字符串（包含 null 终止符）
fn string_to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

// 声明 GetLastError、LocalAlloc 和 LocalFree 外部函数（使用正确的链接方式）
#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetLastError() -> winapi::shared::minwindef::DWORD;
    fn LocalAlloc(uFlags: winapi::shared::minwindef::UINT, uBytes: winapi::shared::basetsd::SIZE_T) -> *mut winapi::ctypes::c_void;
    fn LocalFree(hMem: *mut winapi::ctypes::c_void) -> *mut winapi::ctypes::c_void;
}

/// Windows 平台重装打印机入口（简化版：按名称精确匹配）
/// 
/// # 参数
/// - `config_printer_key`: 配置打印机名称
/// - `config_printer_path`: 配置打印机路径（用于提取 IP）
/// - `config_printer_name`: 配置打印机名称（用于重新安装）
/// - `driverPath`: 驱动路径
/// - `model`: 打印机型号
/// - `remove_port`: 是否删除端口
/// - `remove_driver`: 是否删除驱动
/// - `driverInstallStrategy`: 驱动安装策略
#[allow(non_snake_case)]
pub async fn reinstall_printer_windows(
    app: tauri::AppHandle,  // 用于发送进度事件
    config_printer_key: String,
    config_printer_path: String,
    config_printer_name: String,
    driverPath: Option<String>,
    model: Option<String>,
    _remove_port: bool,
    _remove_driver: bool,
    driverInstallStrategy: Option<String>,
) -> Result<crate::InstallResult, String> {
    let start_time = std::time::Instant::now();
    let call_id = format!("reinstall_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis());

    log::write_log(&format!("[ReinstallPrinter][#{}] CALL_START config_key={} name={} path={}", 
        call_id, config_printer_key, config_printer_name, config_printer_path));

    // 步骤 1: 检查已安装的打印机是否存在
    // 使用 resolve_target_printer 解析目标打印机
    let ip = extract_ip_from_path(&config_printer_path);
    match resolve_target_printer(config_printer_key.clone(), ip.clone()) {
        Ok(target_name) => {
            log::write_log(&format!("[ReinstallPrinter][#{}] CHECK_PHASE_START target_name={}", call_id, target_name));
            // 检测到同名打印机已存在，返回明确提示
            log::write_log(&format!("[ReinstallPrinter][#{}] CHECK_PHASE_FOUND target_name={}", call_id, target_name));
            let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
            let job_id = format!("reinstall_{}_{}", timestamp, (timestamp % 10000) as u32);
            return Ok(crate::InstallResult {
                success: false,
                message: format!("同名打印机 \"{}\" 已存在。为保证确定性，ePrinty 不提供应用内删除，请在系统设置中删除后重试。", target_name),
                method: Some("Windows".to_string()),
                stdout: None,
                stderr: Some(format!("同名打印机已存在: {}", target_name)),
                effective_dry_run: false, // 重装是真实操作
                job_id,
            });
        }
        Err(e) => {
            // 未找到打印机，视为未安装，继续执行安装流程
            log::write_log(&format!("[ReinstallPrinter][#{}] CHECK_PHASE_SKIP reason=not_found error={}", call_id, e));
        }
    }

    // 步骤 2: 重新安装打印机
    log::write_log(&format!("[ReinstallPrinter][#{}] INSTALL_PHASE_START", call_id));
    let install_result = crate::platform::windows::install::install_printer_windows(
        app,  // 传入 app_handle（用于发送进度事件）
        config_printer_name.clone(),
        config_printer_path.clone(),
        driverPath.clone(),
        model.clone(),
        driverInstallStrategy.clone(),
        None,  // driverKey: 重装时无 driverKey
        None,  // install_mode: 重装时使用默认值
        false  // dry_run: 重装时不使用 dryRun 模式
    ).await;

    match install_result {
        Ok(result) => {
            if result.success {
                log::write_log(&format!("[ReinstallPrinter][#{}] INSTALL_PHASE_OK", call_id));
                
                // 步骤 3: 写入 ePrinty tag
                log::write_log(&format!("[ReinstallPrinter][#{}] TAG_WRITE_START", call_id));
                match write_eprinty_tag(&config_printer_name, &config_printer_key, &config_printer_path) {
                    Ok(_) => {
                        log::write_log(&format!("[ReinstallPrinter][#{}] TAG_WRITE_OK", call_id));
                    }
                    Err(e) => {
                        log::write_log(&format!("[ReinstallPrinter][#{}] TAG_WRITE_FAIL error={}", call_id, e));
                        // tag 写入失败不影响安装成功，只记录日志
                    }
                }

                let elapsed_ms = start_time.elapsed().as_millis();
                log::write_log(&format!("[ReinstallPrinter][#{}] CALL_SUCCESS elapsed_ms={}", call_id, elapsed_ms));
                
                Ok(crate::InstallResult {
                    success: true,
                    message: format!("已成功重装打印机: {}", config_printer_name),
                    method: result.method,
                    stdout: result.stdout,
                    stderr: result.stderr,
                    effective_dry_run: result.effective_dry_run, // 从安装结果中获取
                    job_id: result.job_id, // 从安装结果中获取
                })
            } else {
                let elapsed_ms = start_time.elapsed().as_millis();
                log::write_log(&format!("[ReinstallPrinter][#{}] INSTALL_PHASE_FAIL elapsed_ms={}", call_id, elapsed_ms));
                Ok(crate::InstallResult {
                    success: false,
                    message: format!("重装失败: {}", result.message),
                    method: result.method,
                    stdout: result.stdout,
                    stderr: result.stderr,
                    effective_dry_run: result.effective_dry_run, // 从安装结果中获取
                    job_id: result.job_id, // 从安装结果中获取
                })
            }
        }
        Err(e) => {
            let elapsed_ms = start_time.elapsed().as_millis();
            log::write_log(&format!("[ReinstallPrinter][#{}] INSTALL_PHASE_FAIL elapsed_ms={} error={}", call_id, elapsed_ms, e));
            let timestamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis();
            let job_id = format!("reinstall_{}_{}", timestamp, (timestamp % 10000) as u32);
            Ok(crate::InstallResult {
                success: false,
                message: format!("重装失败: {}", e),
                method: Some("Windows".to_string()),
                stdout: None,
                stderr: Some(e),
                effective_dry_run: false, // 重装是真实操作
                job_id,
            })
        }
    }
}

/// 写入 ePrinty tag 到打印机属性
/// 
/// 格式：comment 或 location 写入 "ePrinty:{printerId}|ip={ip}"
/// 注意：此功能将在安装时自动完成，这里仅作为占位符
/// 写入 ePrinty tag 到打印机属性（使用 Win32 API SetPrinterW）
/// 
/// # 参数
/// - `printer_name`: 系统打印机名称
/// - `printer_id`: 打印机唯一标识（printer.id 或生成的 stable_id）
/// - `printer_path`: 打印机路径（用于提取 IP）
/// 
/// # 返回
/// - `Ok(())`: 写入成功
/// - `Err(String)`: 写入失败，包含错误信息
pub fn write_eprinty_tag(
    printer_name: &str,
    printer_id: &str,
    printer_path: &str,
) -> Result<(), String> {
    log::write_log(&format!("[WriteTag] TAG_WRITE_START name={} id={} path={}", printer_name, printer_id, printer_path));

    // 构建 tag 字符串
    let ip = extract_ip_from_path(printer_path);
    let tag_content = if let Some(ip) = ip {
        format!("ePrinty:{}|ip={}", printer_id, ip)
    } else {
        format!("ePrinty:{}", printer_id)
    };

    // 使用 Win32 API SetPrinterW 写入 tag
    use winapi::um::winspool::{OpenPrinterW, GetPrinterW, SetPrinterW, ClosePrinter, PRINTER_DEFAULTSW, PRINTER_INFO_2W};
    use winapi::um::winnt::LPWSTR;
    use winapi::ctypes::c_void;
    use std::ptr;

    unsafe {
        // 将打印机名称转换为 UTF-16 宽字符串
        let printer_name_wide = string_to_wide(printer_name);

        // 打开打印机
        let mut printer_handle: *mut c_void = ptr::null_mut();
        let mut defaults: PRINTER_DEFAULTSW = PRINTER_DEFAULTSW {
            pDataType: ptr::null_mut(),
            pDevMode: ptr::null_mut(),
            DesiredAccess: 0x00000008, // PRINTER_ACCESS_USE
        };

        let open_result = OpenPrinterW(
            printer_name_wide.as_ptr() as LPWSTR,
            &mut printer_handle,
            &mut defaults,
        );

        if open_result == 0 {
            let error_code = GetLastError();
            log::write_log(&format!("[WriteTag] TAG_WRITE_FAIL name={} step=OpenPrinterW err={}", printer_name, error_code));
            return Err(format!("打开打印机失败: error_code={}", error_code));
        }

        // 获取当前打印机信息（Level 2）
        let mut needed: u32 = 0;
        let level: u32 = 2;
        
        // 第一次调用：获取所需 buffer 大小
        let _ = GetPrinterW(
            printer_handle,
            level,
            ptr::null_mut(),
            0,
            &mut needed,
        );

        if needed == 0 {
            let _ = ClosePrinter(printer_handle);
            log::write_log(&format!("[WriteTag] TAG_WRITE_FAIL name={} step=GetPrinterW needed=0", printer_name));
            return Err("获取打印机信息失败: needed=0".to_string());
        }

        // 分配 buffer
        let mut buffer: Vec<u8> = vec![0; needed as usize];
        let mut returned: u32 = 0;

        // 第二次调用：获取打印机信息
        let get_result = GetPrinterW(
            printer_handle,
            level,
            buffer.as_mut_ptr() as *mut _,
            needed,
            &mut returned,
        );

        if get_result == 0 {
            let error_code = GetLastError();
            let _ = ClosePrinter(printer_handle);
            log::write_log(&format!("[WriteTag] TAG_WRITE_FAIL name={} step=GetPrinterW err={}", printer_name, error_code));
            return Err(format!("获取打印机信息失败: error_code={}", error_code));
        }

        // 解析 PRINTER_INFO_2W
        let info_ptr = buffer.as_mut_ptr() as *mut PRINTER_INFO_2W;
        let info = &mut *info_ptr;

        // 将 tag 转换为 UTF-16 宽字符串
        let tag_wide = string_to_wide(&tag_content);
        
        // 分配内存并复制 tag 字符串（SetPrinterW 需要保持有效）
        // LMEM_FIXED = 0x0000
        let tag_ptr_raw = LocalAlloc(
            0x0000, // LMEM_FIXED
            tag_wide.len() * std::mem::size_of::<u16>(),
        );
        if tag_ptr_raw.is_null() {
            let _ = ClosePrinter(printer_handle);
            log::write_log(&format!("[WriteTag] TAG_WRITE_FAIL name={} step=LocalAlloc", printer_name));
            return Err("分配内存失败".to_string());
        }
        let tag_ptr = tag_ptr_raw as *mut u16;

        if tag_ptr.is_null() {
            let _ = ClosePrinter(printer_handle);
            log::write_log(&format!("[WriteTag] TAG_WRITE_FAIL name={} step=LocalAlloc", printer_name));
            return Err("分配内存失败".to_string());
        }

        std::ptr::copy_nonoverlapping(tag_wide.as_ptr(), tag_ptr, tag_wide.len());

        // 修改 pComment 字段（优先使用 Comment，如果失败则使用 Location）
        // 注意：这里我们修改 Comment，如果系统不允许修改 Comment，可以尝试 Location
        info.pComment = tag_ptr as LPWSTR;

        // 调用 SetPrinterW 更新打印机信息
        let set_result = SetPrinterW(
            printer_handle,
            level,
            buffer.as_mut_ptr() as *mut _,
            winapi::um::winspool::PRINTER_CONTROL_SET_STATUS, // 0x00000001
        );

        // 释放分配的内存
        LocalFree(tag_ptr_raw);

        // 关闭打印机句柄
        let _ = ClosePrinter(printer_handle);

        if set_result == 0 {
            let error_code = GetLastError();
            log::write_log(&format!("[WriteTag] TAG_WRITE_FAIL name={} step=SetPrinterW err={}", printer_name, error_code));
            return Err(format!("设置打印机 tag 失败: error_code={}", error_code));
        }

        log::write_log(&format!("[WriteTag] TAG_WRITE_OK name={} tag={}", printer_name, tag_content));
        Ok(())
    }
}

