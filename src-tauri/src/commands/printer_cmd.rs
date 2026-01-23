/**
 * 打印机命令处理
 */

#[tauri::command]
pub fn list_printers() -> Result<Vec<crate::platform::PrinterDetectEntry>, String> {
    crate::services::printer_service::list_printers()
}

#[tauri::command]
pub fn list_printers_detailed() -> Result<Vec<crate::platform::DetailedPrinterInfo>, String> {
    crate::services::printer_service::list_printers_detailed()
}
