/**
 * 配置相关命令
 * 职责：
 * - get_cached_config
 * - refresh_remote_config
 * - load_config
 * - confirm_update_config
 */

use crate::*;
use std::fs;

pub fn get_cached_config(app: &tauri::AppHandle) -> Result<CachedConfigResult, String> {
    eprintln!("[CACHE_LOADED] 开始读取缓存配置");

    let config_path = get_config_path(&app)?;

    // 步骤 1: 如果本地配置存在，直接读取并返回
    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .map_err(|e| format!("读取本地配置文件失败: {}", e))?;

        let config: PrinterConfig = serde_json::from_str(&content)
            .map_err(|e| format!("解析本地配置文件失败: {}", e))?;

        validate_printer_config_v2(&config)?;

        let timestamp = config_path
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        let config_version = config.version.clone();

        eprintln!(
            "[CONFIG_LOADED] source=local path={} version={:?}",
            config_path.display(),
            config_version
        );

        return Ok(CachedConfigResult {
            config,
            source: "local".to_string(),
            timestamp,
            version: config_version,
        });
    }

    // 步骤 2: 本地不存在，尝试从 seed 复制
    eprintln!("[CACHE_LOADED] 本地配置不存在，尝试从 seed 复制");
    match seed_config_if_needed(&app) {
        Ok(_) => {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("读取 seed 复制的配置文件失败: {}", e))?;

            let config: PrinterConfig = serde_json::from_str(&content)
                .map_err(|e| format!("解析 seed 配置文件失败: {}", e))?;

            validate_printer_config_v2(&config)?;

            let timestamp = config_path
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());

            let config_version = config.version.clone();

            eprintln!(
                "[CONFIG_LOADED] source=seed path={} version={:?}",
                config_path.display(),
                config_version
            );

            return Ok(CachedConfigResult {
                config,
                source: "seed".to_string(),
                timestamp,
                version: config_version,
            });
        }
        Err(seed_err) => {
            eprintln!("[CACHE_LOADED] Seed 配置失败: {}", seed_err);
        }
    }

    // 步骤 3: seed 也不可用，必须同步拉取远程配置（首次启动兜底）
    eprintln!("[CACHE_LOADED] 本地和 seed 都不可用，同步拉取远程配置");

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| format!("创建异步运行时失败: {}", e))?;

    let remote_config = rt.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(10),
            load_remote_config(),
        )
        .await
    });

    match remote_config {
        Ok(Ok(config)) => {
            save_config_to_local(&config, &config_path)
                .map_err(|e| format!("保存远程配置到本地失败: {}", e))?;

            let config_version = config.version.clone();

            eprintln!(
                "[CONFIG_LOADED] source=remote_bootstrap path={} version={:?}",
                config_path.display(),
                config_version
            );

            Ok(CachedConfigResult {
                config,
                source: "remote_bootstrap".to_string(),
                timestamp: None,
                version: config_version,
            })
        }
        Ok(Err(e)) => {
            Err(format!(
                "首次启动必须远程获取配置，但远程获取失败: {}",
                e
            ))
        }
        Err(_) => Err("首次启动必须远程获取配置，但请求超时".to_string()),
    }
}

pub async fn refresh_remote_config(app: &tauri::AppHandle) -> Result<RefreshConfigResult, String> {
    eprintln!("[REMOTE_REFRESH_START] 开始刷新远程配置");

    let config_path = get_config_path(&app)?;

    let local_version = if config_path.exists() {
        match fs::read_to_string(&config_path) {
            Ok(content) => match serde_json::from_str::<PrinterConfig>(&content) {
                Ok(config) => config.version.clone(),
                Err(_) => {
                    eprintln!("[REMOTE_REFRESH] 本地配置文件格式错误，将尝试更新");
                    None
                }
            },
            Err(_) => None,
        }
    } else {
        None
    };

    let remote_result = tokio::time::timeout(
        std::time::Duration::from_millis(3000),
        load_remote_config(),
    )
    .await;

    match remote_result {
        Ok(Ok(remote_config)) => {
            let remote_version = remote_config.version.clone();

            let should_update = match (&local_version, &remote_version) {
                (Some(local_v), Some(remote_v)) => remote_v > local_v,
                (None, _) => {
                    eprintln!(
                        "[REMOTE_REFRESH] 本地配置缺少版本字段，将更新以引入版本"
                    );
                    true
                }
                (_, None) => {
                    eprintln!("[REMOTE_REFRESH] 远程配置缺少版本字段，跳过更新");
                    false
                }
            };

            if should_update {
                eprintln!(
                    "[REMOTE_REFRESH] 远程版本更高，将更新本地配置 local={:?} remote={:?}",
                    local_version, remote_version
                );

                match save_config_to_local(&remote_config, &config_path) {
                    Ok(_) => {
                        eprintln!(
                            "[REMOTE_REFRESH_OK] 远程配置已更新到本地 version={:?} path={}",
                            remote_version,
                            config_path.display()
                        );

                        let payload = serde_json::json!({
                            "version": remote_version,
                            "config": remote_config,
                            "updated": true,
                        });

                        if let Err(e) = app.emit_all("config_updated", payload) {
                            eprintln!("[WARN] 发送 config_updated 事件失败: {}", e);
                        }

                        Ok(RefreshConfigResult {
                            success: true,
                            error: None,
                            version: remote_version,
                        })
                    }
                    Err(save_err) => {
                        let error_msg = format!("保存配置失败: {}", save_err);
                        eprintln!("[REMOTE_REFRESH_FAIL] {}", error_msg);

                        let payload = serde_json::json!({
                            "error": error_msg.clone(),
                        });

                        if let Err(emit_err) = app.emit_all("config_refresh_failed", payload) {
                            eprintln!(
                                "[WARN] 发送 config_refresh_failed 事件失败: {}",
                                emit_err
                            );
                        }

                        Err(error_msg)
                    }
                }
            } else {
                eprintln!(
                    "[REMOTE_REFRESH] 远程版本未提升，跳过更新 local={:?} remote={:?}",
                    local_version, remote_version
                );

                Ok(RefreshConfigResult {
                    success: true,
                    error: None,
                    version: remote_version,
                })
            }
        }
        Ok(Err(e)) => {
            let error_msg = format!("远程配置加载失败: {}", e);
            eprintln!("[REMOTE_REFRESH_FAIL] {}", error_msg);

            let payload = serde_json::json!({
                "error": error_msg.clone(),
            });

            if let Err(emit_err) = app.emit_all("config_refresh_failed", payload) {
                eprintln!("[WARN] 发送 config_refresh_failed 事件失败: {}", emit_err);
            }

            Err(error_msg)
        }
        Err(_) => {
            let error_msg = "远程配置加载超时".to_string();
            eprintln!("[REMOTE_REFRESH_FAIL] {}", error_msg);

            let payload = serde_json::json!({
                "error": error_msg.clone(),
            });

            if let Err(emit_err) = app.emit_all("config_refresh_failed", payload) {
                eprintln!("[WARN] 发送 config_refresh_failed 事件失败: {}", emit_err);
            }

            Err(error_msg)
        }
    }
}
