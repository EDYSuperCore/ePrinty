pub mod archive;
pub mod cmd;
pub mod driver_bootstrap;
pub mod driver_fetch;
pub mod encoding;
pub mod enum_printers;
pub mod install;
pub mod list;
pub mod log;
pub mod open;
pub mod powershell_install;
pub mod printer_exists;
pub mod ps;
pub mod remove;
pub mod step_reporter;
pub mod test_page;

// 重新导出 DetailedPrinterInfo 以便子模块使用
pub use crate::platform::DetailedPrinterInfo;

// 注意：list 模块的函数通过完整路径访问，不需要在这里导出
// 注意：remove 模块的函数仅在内部使用，不对外导出

// 注意：install 模块的函数和类型通过完整路径访问，不需要在这里导出
// 这样可以避免循环依赖和未使用的导入警告

