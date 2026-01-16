#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

pub mod test_page_content;

use serde::{Deserialize, Serialize};

// 导入 InstallResult 类型（需要在 main.rs 中定义或从 windows 模块导入）
// 注意：由于 InstallResult 在 main.rs 中定义，这里使用 crate::InstallResult

/// 详细的打印机信息结构体（包含 comment 和 location）
/// 在所有平台上定义以避免条件编译问题
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedPrinterInfo {
    pub name: String,
    pub port_name: Option<String>,
    pub driver_name: Option<String>,
    pub comment: Option<String>,
    pub location: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrinterDetectEntry {
    pub installed_key: String,
    pub system_queue_name: String,
    pub display_name: Option<String>,
    pub device_uri: Option<String>,
    pub is_accepting_jobs: Option<bool>,
    pub state: Option<i32>,
    pub platform: String,
}

/// 平台统一的打印机列表获取入口
/// 
/// 根据当前平台调用相应的实现：
/// - Windows: 调用 Windows 实现
/// - macOS: 调用 macOS 实现
pub fn list_printers() -> Result<Vec<PrinterDetectEntry>, String> {
    #[cfg(windows)]
    {
        // Windows 平台：调用 Windows 实现
        let names = crate::platform::windows::list::list_printers_windows()?;
        Ok(names
            .into_iter()
            .map(|name| PrinterDetectEntry {
                installed_key: name.clone(),
                system_queue_name: name.clone(),
                display_name: Some(name),
                device_uri: None,
                is_accepting_jobs: None,
                state: None,
                platform: "windows".to_string(),
            })
            .collect())
    }
    
    #[cfg(target_os = "macos")]
    {
        let destinations = crate::platform::macos::list_destinations()?;
        Ok(destinations
            .into_iter()
            .map(|dest| PrinterDetectEntry {
                installed_key: dest.name.clone(),
                system_queue_name: dest.name,
                display_name: dest.display_name,
                device_uri: dest.device_uri,
                is_accepting_jobs: dest.is_accepting_jobs,
                state: dest.state,
                platform: "macos".to_string(),
            })
            .collect())
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("当前仅支持 Windows 和 macOS 平台".to_string())
    }
}

/// 平台统一的详细打印机列表获取入口（包含 comment 和 location）
/// 
/// 根据当前平台调用相应的实现：
/// - Windows: 调用 Windows 实现
/// - macOS: 调用 macOS 实现（暂未实现）
pub fn list_printers_detailed() -> Result<Vec<DetailedPrinterInfo>, String> {
    #[cfg(windows)]
    {
        // Windows 平台：调用 Windows 实现
        crate::platform::windows::list::list_printers_detailed()
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS 平台：暂不支持
        Err("macOS 平台暂不支持该功能".to_string())
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("当前仅支持 Windows 和 macOS 平台".to_string())
    }
}

/// 平台统一的打印机安装入口
/// 
/// 根据当前平台调用相应的实现：
/// - Windows: 调用 Windows 实现
/// - macOS: 调用 macOS 实现
#[allow(non_snake_case)]
pub async fn install_printer(
    app: tauri::AppHandle,  // 用于发送进度事件
    name: String,
    path: String,
    driverPath: Option<String>,
    model: Option<String>,
    driverInstallPolicy: Option<String>,  // 驱动安装策略："always" | "reuse_if_installed"
    driverKey: Option<String>,  // v2.0.0+：驱动键（用于 meta 记录）
    installMode: Option<String>,  // 安装方式："auto" | "package" | "installer" | "ipp" | "legacy_inf"（使用 camelCase 匹配前端）
    dry_run: bool,  // 测试模式：true 表示仅模拟，不执行真实安装
) -> Result<crate::InstallResult, String> {
    #[cfg(windows)]
    {
        // Windows 平台：调用 Windows 实现
        let result = crate::platform::windows::install::install_printer_windows(app, name, path, driverPath, model, driverInstallPolicy, driverKey, installMode, dry_run).await?;
        // 转换 InstallResult 类型（从 platform/windows/install::InstallResult 到 crate::InstallResult）
        Ok(crate::InstallResult {
            success: result.success,
            message: result.message,
            method: result.method,
            stdout: result.stdout,
            stderr: result.stderr,
            effective_dry_run: result.effective_dry_run, // 从平台结果中获取
            job_id: result.job_id, // 传递 jobId 给前端
        })
    }
    
    #[cfg(target_os = "macos")]
    {
        crate::platform::macos::install::install_printer_macos(
            app,
            name,
            path,
            installMode,
            dry_run,
        )
        .await
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("当前仅支持 Windows 和 macOS 平台".to_string())
    }
}

/// 平台统一的打印测试页入口
/// 
/// 根据当前平台调用相应的实现：
/// - Windows: 调用 Windows 实现
/// - macOS: 调用 macOS 实现（暂未实现）
pub fn print_test_page(app: tauri::AppHandle, printer_name: String) -> Result<String, String> {
    #[cfg(windows)]
    {
        // Windows 平台：调用 Windows 实现
        crate::platform::windows::test_page::print_test_page_windows(printer_name)
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS 平台：调用 macOS 实现
        crate::platform::macos::test_page::print_test_page_macos(app, printer_name)
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("当前仅支持 Windows 和 macOS 平台".to_string())
    }
}

/// 平台统一的重装打印机入口
/// 
/// 根据当前平台调用相应的实现：
/// - Windows: 调用 Windows 实现
/// - macOS: 调用 macOS 实现（暂未实现）
#[allow(non_snake_case)]
pub async fn reinstall_printer(
    app: tauri::AppHandle,  // 用于发送进度事件
    config_printer_key: String,
    config_printer_path: String,
    config_printer_name: String,
    driverPath: Option<String>,
    model: Option<String>,
    remove_port: bool,
    remove_driver: bool,
    driverInstallStrategy: Option<String>,
) -> Result<crate::InstallResult, String> {
    #[cfg(windows)]
    {
        // Windows 平台：调用 Windows 实现
        crate::platform::windows::remove::reinstall_printer_windows(
            app,
            config_printer_key,
            config_printer_path,
            config_printer_name,
            driverPath,
            model,
            remove_port,
            remove_driver,
            driverInstallStrategy,
        ).await
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS 平台：暂不支持
        Err("macOS 平台暂不支持该功能".to_string())
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("当前仅支持 Windows 和 macOS 平台".to_string())
    }
}

/// 平台统一的打开 URL 入口
/// 
/// 根据当前平台调用相应的实现：
/// - Windows: 调用 Windows 实现
/// - macOS: 调用 macOS 实现
pub fn open_url(url: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        // Windows 平台：调用 Windows 实现
        crate::platform::windows::open::open_url_windows(url)
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS 平台：调用 macOS 实现
        crate::platform::macos::open_url_macos(url)
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("当前仅支持 Windows 和 macOS 平台".to_string())
    }
}

/// 删除打印机结果（统一结构）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeletePrinterResult {
    pub success: bool,
    pub printer_name: String,
    pub removed_queue: bool,
    pub removed_port: bool,
    pub removed_driver: bool,
    pub driver_name: Option<String>,
    pub port_name: Option<String>,
    pub message: String,
    pub evidence: Option<String>,
}

/// 平台统一的删除打印机入口
/// 
/// 根据当前平台调用相应的实现：
/// - Windows: 调用 Windows 实现
/// - macOS: 调用 macOS 实现
pub fn delete_printer(printer_name: &str, remove_port: bool, remove_driver: bool) -> Result<DeletePrinterResult, String> {
    #[cfg(windows)]
    {
        // Windows 平台：调用 Windows 实现
        let result = crate::platform::windows::delete::delete_printer_windows(printer_name, remove_port, remove_driver)?;
        Ok(DeletePrinterResult {
            success: result.success,
            printer_name: printer_name.to_string(),
            removed_queue: result.removed_queue,
            removed_port: result.removed_port,
            removed_driver: result.removed_driver,
            driver_name: result.driver_name,
            port_name: result.port_name,
            message: result.message,
            evidence: result.evidence,
        })
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS 平台：调用 macOS 实现（不支持 remove_driver）
        let result = crate::platform::macos::delete::delete_printer_macos(printer_name, remove_port)?;
        Ok(DeletePrinterResult {
            success: result.success,
            printer_name: printer_name.to_string(),
            removed_queue: result.removed_queue,
            removed_port: result.removed_port,
            removed_driver: false, // macOS 不支持驱动删除
            driver_name: None,
            port_name: None,
            message: result.message,
            evidence: result.evidence,
        })
    }
    
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        Err("当前仅支持 Windows 和 macOS 平台".to_string())
    }
}
