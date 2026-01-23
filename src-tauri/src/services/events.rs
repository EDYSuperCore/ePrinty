/**
 * 事件发送服务
 * 职责：
 * - 封装事件发送逻辑
 * - config_updated
 * - config_refresh_failed
 */

use crate::*;

pub fn emit_config_updated(
    app: &tauri::AppHandle,
    config: &PrinterConfig,
    version: Option<String>,
) {
    let payload = serde_json::json!({
        "version": version,
        "config": config,
        "updated": true,
    });

    if let Err(e) = app.emit_all("config_updated", payload) {
        eprintln!("[WARN] 发送 config_updated 事件失败: {}", e);
    }
}

pub fn emit_config_refresh_failed(app: &tauri::AppHandle, error: &str) {
    let payload = serde_json::json!({
        "error": error,
    });

    if let Err(e) = app.emit_all("config_refresh_failed", payload) {
        eprintln!("[WARN] 发送 config_refresh_failed 事件失败: {}", e);
    }
}
