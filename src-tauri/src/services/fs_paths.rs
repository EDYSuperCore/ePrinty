/**
 * 文件系统路径管理服务
 * 职责：
 * - 集中管理本地路径获取
 * - Windows: exe_dir
 * - macOS: app_config_dir
 */

use crate::*;
use std::fs;
use std::path::PathBuf;

// 获取配置文件路径（统一入口，平台特定策略）
pub fn get_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    #[cfg(target_os = "windows")]
    {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("获取可执行文件路径失败: {}", e))?;
        let exe_dir = exe_path
            .parent()
            .ok_or_else(|| "无法获取可执行文件目录".to_string())?;
        Ok(exe_dir.join(CONFIG_FILE_NAME))
    }

    #[cfg(target_os = "macos")]
    {
        use tauri::api::path::app_config_dir;

        let app_config = app_config_dir(&app.config())
            .ok_or_else(|| "无法获取应用配置目录".to_string())?;

        fs::create_dir_all(&app_config)
            .map_err(|e| format!("创建配置目录失败: {}", e))?;

        Ok(app_config.join(CONFIG_FILE_NAME))
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        use tauri::api::path::app_config_dir;

        let app_config = app_config_dir(&app.config())
            .ok_or_else(|| "无法获取应用配置目录".to_string())?;

        fs::create_dir_all(&app_config)
            .map_err(|e| format!("创建配置目录失败: {}", e))?;

        Ok(app_config.join(CONFIG_FILE_NAME))
    }
}

pub fn get_local_config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    get_config_path(app)
}

pub fn get_seed_config_path(app: &tauri::AppHandle) -> Option<PathBuf> {
    use tauri::api::path::resource_dir;

    if let Some(resource_dir) = resource_dir(app.package_info(), &app.env()) {
        let possible_names = ["printer_config.json", "default_printer_config.json"];
        for name in &possible_names {
            let seed_path = resource_dir.join(name);
            if seed_path.exists() {
                eprintln!(
                    "[CONFIG_SEED] 找到 seed 配置文件: {}",
                    seed_path.display()
                );
                return Some(seed_path);
            }
        }
    }

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            #[cfg(target_os = "macos")]
            {
                if exe_dir.ends_with("MacOS") {
                    if let Some(contents_dir) = exe_dir.parent() {
                        let resources_dir = contents_dir.join("Resources");
                        let possible_names = ["printer_config.json", "default_printer_config.json"];
                        for name in &possible_names {
                            let seed_path = resources_dir.join(name);
                            if seed_path.exists() {
                                eprintln!(
                                    "[CONFIG_SEED] 找到 seed 配置文件 (macOS bundle): {}",
                                    seed_path.display()
                                );
                                return Some(seed_path);
                            }
                        }
                    }
                }
            }

            let possible_names = ["printer_config.json", "default_printer_config.json"];
            for name in &possible_names {
                let seed_path = exe_dir.join(name);
                if seed_path.exists() {
                    eprintln!(
                        "[CONFIG_SEED] 找到 seed 配置文件 (开发模式): {}",
                        seed_path.display()
                    );
                    return Some(seed_path);
                }
            }
        }
    }

    eprintln!("[CONFIG_SEED] 未找到 seed 配置文件");
    None
}
