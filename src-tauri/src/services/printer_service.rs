/**
 * 打印机相关命令
 * 职责：
 * - list_printers
 * - list_printers_detailed
 */

pub fn list_printers() -> Result<Vec<crate::platform::PrinterDetectEntry>, String> {
    crate::platform::list_printers()
}

pub fn list_printers_detailed() -> Result<Vec<crate::platform::DetailedPrinterInfo>, String> {
    crate::platform::list_printers_detailed()
}
