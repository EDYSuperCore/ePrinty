#[cfg(windows)]
pub mod windows;

#[cfg(target_os = "macos")]
pub mod macos;

// 导入 InstallResult 类型（需要在 main.rs 中定义或从 windows 模块导入）
// 注意：由于 InstallResult 在 main.rs 中定义，这里使用 crate::InstallResult

/// 平台统一的打印机列表获取入口
/// 
/// 根据当前平台调用相应的实现：
/// - Windows: 调用 Windows 实现
/// - macOS: 调用 macOS 实现
pub fn list_printers() -> Result<Vec<String>, String> {
    #[cfg(windows)]
    {
        // Windows 平台：调用 Windows 实现
        crate::platform::windows::list::list_printers_windows()
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS 平台：调用 macOS 实现
        crate::platform::macos::list_printers_macos()
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
pub fn list_printers_detailed() -> Result<Vec<crate::platform::windows::list::DetailedPrinterInfo>, String> {
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
    name: String,
    path: String,
    driverPath: Option<String>,
    model: Option<String>,
    driverInstallPolicy: Option<String>,  // 驱动安装策略："always" | "reuse_if_installed"
    installMode: Option<String>,  // 安装方式："auto" | "package" | "installer" | "ipp" | "legacy_inf"（使用 camelCase 匹配前端）
    dry_run: bool,  // 测试模式：true 表示仅模拟，不执行真实安装
) -> Result<crate::InstallResult, String> {
    #[cfg(windows)]
    {
        // Windows 平台：调用 Windows 实现
        let result = crate::platform::windows::install::install_printer_windows(name, path, driverPath, model, driverInstallPolicy, installMode, dry_run).await?;
        // 转换 InstallResult 类型（从 platform/windows/install::InstallResult 到 crate::InstallResult）
        Ok(crate::InstallResult {
            success: result.success,
            message: result.message,
            method: result.method,
            stdout: result.stdout,
            stderr: result.stderr,
        })
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS 平台：调用 macOS 实现
        // 注意：macOS 安装实现尚未迁移到 platform/macos，暂时返回错误
        // TODO: 当 macos::install_printer_macos 实现后，替换为：
        // crate::platform::macos::install_printer_macos(name, path, driverPath, model).await
        Err("macOS 平台暂不支持该功能".to_string())
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
pub fn print_test_page(printer_name: String) -> Result<String, String> {
    #[cfg(windows)]
    {
        // Windows 平台：调用 Windows 实现
        crate::platform::windows::test_page::print_test_page_windows(printer_name)
    }
    
    #[cfg(target_os = "macos")]
    {
        // macOS 平台：调用 macOS 实现
        // 注意：macOS 打印测试页实现尚未迁移到 platform/macos，暂时返回错误
        Err("macOS 平台暂不支持该功能".to_string())
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

