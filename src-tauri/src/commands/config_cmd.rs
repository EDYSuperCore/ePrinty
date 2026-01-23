/**
 * 配置命令处理
 */

use crate::*;

#[tauri::command]
pub fn get_cached_config(app: tauri::AppHandle) -> Result<CachedConfigResult, String> {
    crate::services::config_service::get_cached_config(&app)
}

#[tauri::command]
pub async fn refresh_remote_config(app: tauri::AppHandle) -> Result<RefreshConfigResult, String> {
    crate::services::config_service::refresh_remote_config(&app).await
}
