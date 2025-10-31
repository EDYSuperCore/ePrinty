// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::fs;

#[cfg(windows)]
use encoding_rs::GBK;

// Windows 编码转换辅助函数（解决中文乱码问题）
#[cfg(windows)]
fn decode_windows_string(bytes: &[u8]) -> String {
    // 尝试 UTF-8 解码
    if let Ok(utf8_str) = String::from_utf8(bytes.to_vec()) {
        return utf8_str;
    }
    
    // 如果 UTF-8 失败，尝试 GBK 解码（中文 Windows 默认编码）
    // 对于非 GBK 编码，使用 lossy 解码以避免崩溃
    let (decoded, _, had_errors) = GBK.decode(bytes);
    let result = decoded.to_string();
    
    // 如果 GBK 解码也有错误，使用 UTF-8 lossy 作为后备
    if had_errors {
        String::from_utf8_lossy(bytes).to_string()
    } else {
        result
    }
}

#[cfg(not(windows))]
fn decode_windows_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

#[derive(Debug, Serialize, Deserialize)]
struct PrinterConfig {
    areas: Vec<Area>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Area {
    name: String,
    printers: Vec<Printer>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Printer {
    name: String,
    path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct InstallResult {
    success: bool,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct LoadConfigResult {
    config: PrinterConfig,
    source: String, // "local" 或 "remote"
    remote_error: Option<String>,
}

// 加载本地配置文件
fn load_local_config() -> Result<PrinterConfig, String> {
    use std::path::PathBuf;
    
    // 尝试从多个可能的位置加载本地配置
    // 优先级：1. 可执行文件所在目录（应用目录）- 最高优先级
    //         2. 当前工作目录
    //         3. 项目根目录（开发模式）
    
    let mut search_paths: Vec<PathBuf> = vec![];
    
    // 优先：可执行文件所在目录（打包后的应用会在这里查找）
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            search_paths.push(exe_dir.join("printer_config.json"));
        }
    }
    
    // 其次：当前工作目录
    if let Ok(current_dir) = std::env::current_dir() {
        search_paths.push(current_dir.join("printer_config.json"));
        // 也尝试上级目录（开发模式下可能在 src-tauri 目录运行）
        if let Some(parent) = current_dir.parent() {
            search_paths.push(parent.join("printer_config.json"));
        }
    }
    
    // 尝试所有可能的路径
    for config_path in &search_paths {
        if config_path.exists() {
            let content = fs::read_to_string(config_path)
                .map_err(|e| format!("读取本地配置文件失败 ({}): {}", config_path.display(), e))?;
            
            let config: PrinterConfig = serde_json::from_str(&content)
                .map_err(|e| format!("解析本地配置文件失败: {}", e))?;
            
            return Ok(config);
        }
    }
    
    // 如果没有找到，返回详细错误信息
    let mut error_msg = "未找到本地配置文件 printer_config.json。已搜索以下位置：\n".to_string();
    for path in search_paths {
        error_msg.push_str(&format!("  - {}\n", path.display()));
    }
    
    Err(error_msg)
}

// 加载打印机配置（优先本地，远程失败只提示）
#[tauri::command]
async fn load_config() -> Result<LoadConfigResult, String> {
    // 优先加载本地配置
    match load_local_config() {
        Ok(local_config) => {
            // 本地配置加载成功，尝试加载远程配置（但不影响使用）
            // 使用 tokio::time::timeout 确保不会无限等待
            let remote_result = tokio::time::timeout(
                std::time::Duration::from_secs(6), // 6秒总超时（比客户端超时多1秒）
                load_remote_config()
            ).await;
            
            let remote_error = match remote_result {
                Ok(Ok(_remote_config)) => {
                    // 远程配置加载成功（但我们使用本地配置）
                    None
                }
                Ok(Err(e)) => {
                    // 远程加载失败，只记录错误，不影响使用
                    Some(format!("远程配置加载失败: {}（已使用本地配置）", e))
                }
                Err(_) => {
                    // 超时
                    Some("远程配置加载超时（已使用本地配置）".to_string())
                }
            };
            
            Ok(LoadConfigResult {
                config: local_config,
                source: "local".to_string(),
                remote_error,
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
                    Ok(LoadConfigResult {
                        config: remote_config,
                        source: "remote".to_string(),
                        remote_error: None,
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

// 加载远程配置
async fn load_remote_config() -> Result<PrinterConfig, String> {
    // 创建带超时的 HTTP 客户端（5秒超时）
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("创建HTTP客户端失败: {}", e))?;
    
    let url = "https://example.com/printer_config.json";
    
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

// 获取本地已安装的打印机列表
#[tauri::command]
fn list_printers() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        
        // 设置 PowerShell 输出编码为 UTF-8，避免中文乱码
        let output = Command::new("powershell")
            .args([
                "-Command",
                "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-Printer | Select-Object -ExpandProperty Name | ConvertTo-Json -Compress"
            ])
            .env("PYTHONIOENCODING", "utf-8")
            .output()
            .map_err(|e| format!("执行 PowerShell 命令失败: {}", e))?;
        
        if !output.status.success() {
            let error = decode_windows_string(&output.stderr);
            return Err(format!("获取打印机列表失败: {}", error));
        }
        
        let stdout = decode_windows_string(&output.stdout);
        let trimmed = stdout.trim();
        
        // 处理空输出
        if trimmed.is_empty() {
            return Ok(vec![]);
        }
        
        // PowerShell 返回的可能是单个字符串或数组
        if trimmed.starts_with('[') {
            // 是数组格式
            let printers: Vec<String> = serde_json::from_str(trimmed)
                .map_err(|e| format!("解析打印机列表失败: {}", e))?;
            Ok(printers)
        } else if trimmed.starts_with('"') && trimmed.ends_with('"') {
            // 是单个字符串
            let printer = trimmed.trim_matches('"').to_string();
            Ok(vec![printer])
        } else {
            // 可能是空数组或格式不正确
            Ok(vec![])
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // 使用 macOS 的 lpstat 命令获取打印机列表
        // lpstat -p 列出所有打印机及其状态
        let output = Command::new("lpstat")
            .arg("-p")
            .output()
            .map_err(|e| format!("执行 lpstat 命令失败: {}", e))?;
        
        if !output.status.success() {
            // lpstat -p 在没有任何打印机时也会返回错误，所以先尝试 lpstat -d 或直接解析输出
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            
            // 如果没有输出，可能只是没有打印机
            if stdout.trim().is_empty() && stderr.trim().is_empty() {
                return Ok(vec![]);
            }
            
            // 尝试使用 lpstat -a 获取打印机列表（列出所有可用的打印机）
            let output2 = Command::new("lpstat")
                .arg("-a")
                .output();
            
            match output2 {
                Ok(result) => {
                    let stdout2 = String::from_utf8_lossy(&result.stdout);
                    if !stdout2.trim().is_empty() {
                        // lpstat -a 输出格式: printer_name accepting requests since ...
                        let printers: Vec<String> = stdout2
                            .lines()
                            .filter_map(|line| {
                                let parts: Vec<&str> = line.split_whitespace().collect();
                                if !parts.is_empty() {
                                    Some(parts[0].to_string())
                                } else {
                                    None
                                }
                            })
                            .collect();
                        return Ok(printers);
                    }
                }
                Err(_) => {}
            }
            
            return Err(format!("获取打印机列表失败: {}", stderr));
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        
        // lpstat -p 输出格式示例:
        // printer Ricoh_Printer is idle.  enabled since ...
        // printer HP_Printer is idle.  enabled since ...
        let printers: Vec<String> = stdout
            .lines()
            .filter_map(|line| {
                if line.starts_with("printer ") {
                    // 格式: "printer Ricoh_Printer is idle..."
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 && parts[0] == "printer" {
                        Some(parts[1].to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();
        
        // 如果没有找到，尝试使用 lpstat -d 获取默认打印机
        if printers.is_empty() {
            let default_output = Command::new("lpstat")
                .arg("-d")
                .output();
            
            if let Ok(result) = default_output {
                let default_stdout = String::from_utf8_lossy(&result.stdout);
                // lpstat -d 输出格式: system default destination: printer_name
                if let Some(colon_pos) = default_stdout.find(':') {
                    let printer_name = default_stdout[colon_pos + 1..].trim().to_string();
                    if !printer_name.is_empty() {
                        return Ok(vec![printer_name]);
                    }
                }
            }
        }
        
        Ok(printers)
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        // 其他平台的实现
        Err("当前仅支持 Windows 和 macOS 平台".to_string())
    }
}

// 安装打印机（使用 prnport.vbs 脚本 + rundll32 printui.dll）
#[tauri::command]
async fn install_printer(name: String, path: String) -> Result<InstallResult, String> {
    #[cfg(windows)]
    {
        use std::process::Command;
        
        // 查找 prnport.vbs 脚本的位置
        let mut script_paths = vec![];
        
        // 1. 尝试应用目录（可执行文件所在目录）
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                script_paths.push(exe_dir.join("scripts").join("prnport.vbs"));
                script_paths.push(exe_dir.join("prnport.vbs"));
            }
        }
        
        // 2. 尝试当前工作目录
        if let Ok(current_dir) = std::env::current_dir() {
            script_paths.push(current_dir.join("scripts").join("prnport.vbs"));
            script_paths.push(current_dir.join("prnport.vbs"));
            // 开发模式下可能在 src-tauri 目录
            if let Some(parent) = current_dir.parent() {
                script_paths.push(parent.join("scripts").join("prnport.vbs"));
                script_paths.push(parent.join("prnport.vbs"));
            }
        }
        
        // 3. 尝试项目根目录的 scripts 目录
        if let Ok(current_dir) = std::env::current_dir() {
            if let Some(parent) = current_dir.parent() {
                script_paths.push(parent.join("src-tauri").join("scripts").join("prnport.vbs"));
            }
        }
        
        // 查找脚本文件
        let script_path = script_paths.iter()
            .find(|p| p.exists())
            .ok_or_else(|| {
                format!("未找到 prnport.vbs 脚本文件。已搜索以下位置：\n{}", 
                    script_paths.iter()
                        .map(|p| format!("  - {}", p.display()))
                        .collect::<Vec<_>>()
                        .join("\n"))
            })?;
        
        // 从路径中提取 IP 地址（格式：\\192.168.x.x）
        let ip_address = path.trim_start_matches("\\\\").to_string();
        
        // 端口名格式：IP_IP地址（用下划线替换点）
        let port_name = format!("IP_{}", ip_address.replace(".", "_"));
        
        // 步骤1：删除旧打印机（如果存在）- 静默模式，忽略错误
        let _ = Command::new("rundll32")
            .args([
                "printui.dll,PrintUIEntry",
                "/dl",  // 删除本地打印机
                "/n",   // 打印机名称
                &format!("\"{}\"", name),
                "/q"    // 静默模式，不显示确认对话框
            ])
            .output();
        
        // 步骤2：使用 cscript 运行 prnport.vbs 脚本添加端口
        // 参考 C# 代码：cscript prnport.vbs -a -r IP_192.168.x.x -h 192.168.x.x -o raw
        let output = Command::new("cscript")
            .args([
                "//NoLogo",  // 不显示脚本横幅
                "//B",       // 批处理模式，不显示脚本错误和提示
                script_path.to_str().unwrap(),
                "-a",        // 添加端口
                "-r",        // 端口名
                &port_name,  // 端口名称
                "-h",        // IP地址
                &ip_address, // IP地址值
                "-o",        // 输出类型
                "raw"        // raw 类型（参考 C# 代码）
            ])
            .output();
        
        match output {
            Ok(result) => {
                let stdout = decode_windows_string(&result.stdout);
                let stderr = decode_windows_string(&result.stderr);
                
                if result.status.success() {
                    // 端口添加成功，现在使用 PowerShell Add-Printer 安装打印机
                    
                    // 注释掉的 rundll32 方法（参考 C# 代码，但暂时不使用）
                    // 参考 C# 代码：rundll32 printui.dll,PrintUIEntry /if /b "打印机名称" /f "驱动文件" /r "IP_IP" /m "驱动型号" /z
                    // 由于我们没有驱动文件和型号信息，使用 /il 模式（安装本地打印机，使用向导）或自动查找驱动
                    /*
                    // 尝试方法1：使用 rundll32 /il（安装本地打印机，静默模式，使用默认或通用驱动）
                    let add_printer_output = Command::new("rundll32")
                        .args([
                            "printui.dll,PrintUIEntry",
                            "/il",  // 安装本地打印机（交互式，但 /q 可以静默）
                            "/b",   // 打印机名称
                            &format!("\"{}\"", name),
                            "/r",   // 端口名
                            &format!("\"{}\"", port_name),
                            "/q"    // 静默模式
                        ])
                        .output();
                    
                    match add_printer_output {
                        Ok(_printer_result) => {
                            // rundll32 可能返回成功状态码，即使有错误
                            // 检查打印机是否真的安装成功
                            let check_output = Command::new("powershell")
                                .args([
                                    "-Command",
                                    &format!(
                                        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-Printer -Name '{}' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name",
                                        name.replace("'", "''")
                                    )
                                ])
                                .output();
                            
                            match check_output {
                                Ok(check_result) => {
                                    let check_stdout = decode_windows_string(&check_result.stdout);
                                    if check_stdout.contains(&name) {
                                        Ok(InstallResult {
                                            success: true,
                                            message: format!("打印机 {} ({}) 安装成功", name, ip_address),
                                        })
                                    } else {
                                        // rundll32 可能失败，尝试使用 PowerShell Add-Printer 作为后备
                                        // ...
                                    }
                                }
                                Err(e) => {
                                    // ...
                                }
                            }
                        }
                        Err(e) => {
                            // ...
                        }
                    }
                    */
                    
                    // 使用 PowerShell Add-Printer 作为主要方法
                    // 查找通用驱动并安装
                    let ps_output = Command::new("powershell")
                        .args([
                            "-Command",
                            &format!(
                                "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $port = '{}'; $drivers = Get-PrinterDriver -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -like '*Generic*' -or $_.Name -like '*Text*' -or $_.Name -like '*HP*' -or $_.Name -like '*RICOH*' -or $_.Name -like '*PCL*' -or $_.Name -like '*PostScript*' }} | Select-Object -First 1; if ($drivers) {{ try {{ Add-Printer -Name '{}' -PortName $port -DriverName $drivers.Name -ErrorAction Stop; Write-Output 'Success' }} catch {{ Write-Error $_.Exception.Message }} }} else {{ $allDrivers = Get-PrinterDriver -ErrorAction SilentlyContinue; if ($allDrivers) {{ $firstDriver = ($allDrivers | Select-Object -First 1).Name; try {{ Add-Printer -Name '{}' -PortName $port -DriverName $firstDriver -ErrorAction Stop; Write-Output 'Success' }} catch {{ Write-Error $_.Exception.Message }} }} else {{ Write-Error '系统中没有可用的打印机驱动。请先安装打印机驱动。' }} }}",
                                port_name,
                                name.replace("'", "''"),
                                name.replace("'", "''")
                            )
                        ])
                        .output();
                    
                    match ps_output {
                        Ok(ps_result) => {
                            let ps_stdout = decode_windows_string(&ps_result.stdout);
                            let ps_stderr = decode_windows_string(&ps_result.stderr);
                            
                            if ps_result.status.success() || ps_stdout.contains("Success") {
                                Ok(InstallResult {
                                    success: true,
                                    message: format!("打印机 {} ({}) 安装成功", name, ip_address),
                                })
                            } else {
                                Ok(InstallResult {
                                    success: false,
                                    message: format!("端口添加成功，但打印机安装失败。错误信息: {}。请确保系统中已安装打印机驱动，或联系管理员安装驱动。", ps_stderr),
                                })
                            }
                        }
                        Err(e) => {
                            Ok(InstallResult {
                                success: false,
                                message: format!("端口添加成功，但执行 PowerShell 命令失败: {}", e),
                            })
                        }
                    }
                } else {
                    Ok(InstallResult {
                        success: false,
                        message: format!("添加打印机端口失败: {} {}", stderr, stdout),
                    })
                }
            }
            Err(e) => {
                Ok(InstallResult {
                    success: false,
                    message: format!("执行 prnport.vbs 脚本失败: {}\n请确保脚本文件存在且可执行", e),
                })
            }
        }
    }
    
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // macOS 使用 lpadmin 命令安装打印机
        // 参考 Objective-C 代码的逻辑：
        // 1. 复制 PPD 文件（如果有）或使用系统自带的驱动
        // 2. 使用 lpadmin 安装打印机
        
        // 从路径中提取 IP 地址（格式：\\192.168.x.x 或 lpd://192.168.x.x）
        let ip_address = path
            .trim_start_matches("\\\\")
            .trim_start_matches("lpd://")
            .trim_start_matches("http://")
            .trim_start_matches("https://")
            .to_string();
        
        // 构建 lpd:// URL（macOS 通常使用 lpd 协议）
        let printer_url = format!("lpd://{}", ip_address);
        
        // 使用 lpadmin 命令安装打印机
        // lpadmin -p <printer_name> -E -v <printer_url> -P <ppd_file> -D <description>
        // -E: 启用打印机
        // -v: 指定打印机地址
        // -P: 指定 PPD 文件（可选，如果不指定会使用通用驱动）
        // -D: 打印机描述
        
        // 先尝试查找是否有 PPD 文件在资源目录中
        // 参考 Objective-C 代码：从 NSBundle mainBundle 查找 .txt 文件，复制到用户目录为 .ppd
        let mut ppd_path: Option<String> = None;
        
        // 获取用户主目录
        let home_dir = std::env::var("HOME")
            .map_err(|_| "无法获取用户主目录".to_string())?;
        
        if let Ok(exe_path) = std::env::current_exe() {
            // 查找资源目录
            // macOS app bundle 结构: App.app/Contents/Resources/
            // 开发模式下可能在 src-tauri/target/... 目录
            let mut resources_dir: Option<std::path::PathBuf> = None;
            
            if let Some(exe_dir) = exe_path.parent() {
                // 如果是打包的 app，exe 在 Contents/MacOS/，Resources 在 Contents/Resources/
                if exe_dir.ends_with("MacOS") {
                    resources_dir = exe_dir.parent().map(|p| p.join("Resources"));
                } else {
                    // 开发模式或其他情况
                    resources_dir = Some(exe_dir.join("Resources"));
                    if !resources_dir.as_ref().unwrap().exists() {
                        // 也尝试直接使用 exe_dir（开发模式）
                        resources_dir = Some(exe_dir.to_path_buf());
                    }
                }
            }
            
            // 查找可能的 PPD 文件（.txt 或 .ppd）
            let possible_ppd_names = ["ricoh320", "generic", "PostScript"];
            
            if let Some(res_dir) = resources_dir {
                for ppd_base in &possible_ppd_names {
                    // 先查找 .txt 文件（参考原代码：ricoh320.txt）
                    let txt_file = res_dir.join(format!("{}.txt", ppd_base));
                    if txt_file.exists() {
                        // 复制到用户目录并改为 .ppd 扩展名
                        let target_ppd = format!("{}/{}.ppd", home_dir, ppd_base);
                        
                        match fs::copy(&txt_file, &target_ppd) {
                            Ok(_) => {
                                ppd_path = Some(target_ppd);
                                break;
                            }
                            Err(_) => {
                                // 复制失败，尝试直接使用原文件
                                ppd_path = Some(txt_file.to_string_lossy().to_string());
                                break;
                            }
                        }
                    }
                    
                    // 查找 .ppd 文件
                    let ppd_file = res_dir.join(format!("{}.ppd", ppd_base));
                    if ppd_file.exists() {
                        ppd_path = Some(ppd_file.to_string_lossy().to_string());
                        break;
                    }
                }
            }
            
            // 也尝试在当前目录和 scripts 目录查找
            if ppd_path.is_none() {
                if let Some(exe_dir) = exe_path.parent() {
                    let possible_dirs = vec![
                        exe_dir.join("Resources"),
                        exe_dir.join("scripts"),
                        exe_dir.to_path_buf(),
                    ];
                    
                    for dir in possible_dirs {
                        for ppd_base in &possible_ppd_names {
                            let txt_file = dir.join(format!("{}.txt", ppd_base));
                            if txt_file.exists() {
                                let target_ppd = format!("{}/{}.ppd", home_dir, ppd_base);
                                if fs::copy(&txt_file, &target_ppd).is_ok() {
                                    ppd_path = Some(target_ppd);
                                    break;
                                }
                            }
                            
                            let ppd_file = dir.join(format!("{}.ppd", ppd_base));
                            if ppd_file.exists() {
                                ppd_path = Some(ppd_file.to_string_lossy().to_string());
                                break;
                            }
                        }
                        if ppd_path.is_some() {
                            break;
                        }
                    }
                }
            }
        }
        
        // 构建 lpadmin 命令
        let mut lpadmin_cmd = Command::new("lpadmin");
        
        // 基本参数
        lpadmin_cmd
            .arg("-p")
            .arg(&name)
            .arg("-E")  // 启用打印机
            .arg("-v")
            .arg(&printer_url)
            .arg("-D")
            .arg(&name);  // 打印机描述
        
        // 如果找到 PPD 文件，使用它；否则让系统自动选择驱动
        if let Some(ppd) = ppd_path {
            lpadmin_cmd.arg("-P").arg(ppd);
        } else {
            // 不指定 -P，系统会使用通用 PostScript 驱动
            // 或者可以使用 -m everywhere 让系统自动选择
            lpadmin_cmd.arg("-m").arg("everywhere");
        }
        
        // 执行 lpadmin 命令
        let output = lpadmin_cmd
            .output()
            .map_err(|e| format!("执行 lpadmin 命令失败: {}", e))?;
        
        if output.status.success() {
            Ok(InstallResult {
                success: true,
                message: format!("打印机 {} ({}) 安装成功", name, ip_address),
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let error_msg = if !stderr.is_empty() {
                stderr.to_string()
            } else if !stdout.is_empty() {
                stdout.to_string()
            } else {
                "未知错误".to_string()
            };
            
            Ok(InstallResult {
                success: false,
                message: format!("安装打印机失败: {}。请确保已授予管理员权限，或联系管理员。", error_msg),
            })
        }
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("当前仅支持 Windows 和 macOS 平台".to_string())
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            load_config,
            list_printers,
            install_printer
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

