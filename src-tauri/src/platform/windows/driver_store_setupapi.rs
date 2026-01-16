#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::path::{Path, PathBuf};
#[cfg(windows)]
use windows::{
    core::{Error as WindowsError, HSTRING},
    Win32::Devices::DeviceAndDriverInstallation::{
        SetupCopyOEMInfW, OEM_SOURCE_MEDIA_TYPE, SP_COPY_STYLE, SPOST_PATH,
    },
};

/// Stage 驱动包的结果
#[derive(Debug, Clone)]
pub struct StageResult {
    /// Published name，例如 "oem170.inf"
    pub published_name: String,
    /// Published INF 文件的完整路径，例如 "C:\Windows\INF\oem170.inf"
    pub published_inf_path: PathBuf,
    /// 证据信息，用于调试和日志
    pub evidence: String,
    /// 是否使用了 fallback（pnputil）
    pub used_fallback: bool,
}

/// Stage 驱动包的错误
#[derive(Debug, Clone)]
pub struct StageError {
    /// 错误消息
    pub message: String,
    /// Win32 错误码（GetLastError）
    pub win32_error: u32,
    /// 证据信息，用于调试和日志
    pub evidence: String,
}

impl std::fmt::Display for StageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (Win32 error: 0x{:08X})\n\n{}",
            self.message, self.win32_error, self.evidence
        )
    }
}

impl std::error::Error for StageError {}

/// 将 Rust 路径转换为 Windows UTF-16 字符串（以 null 结尾）
fn path_to_wide_string(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// 使用 SetupCopyOEMInfW 将驱动包导入 DriverStore 并获取 published name
///
/// # 参数
/// - `inf_path`: INF 文件的完整路径
///
/// # 返回
/// - `Ok(StageResult)`: 成功，包含 published_name 和 published_inf_path
/// - `Err(StageError)`: 失败，包含 Win32 错误码和详细信息
///
/// # 实现说明
/// 使用 Win32 SetupAPI 的 SetupCopyOEMInfW 函数：
/// - SourceInfFileName: inf_path（UTF-16）
/// - OEMSourceMediaLocation: inf_path 的父目录（UTF-16）
/// - OEMSourceMediaType: 0 (SPOST_PATH)
/// - CopyStyle: 0（让系统决定，如果已存在可能返回已存在的 INF）
/// - DestinationInfFileName: 缓冲区接收返回的 "oemXXX.inf"
pub fn stage_driver_and_get_published_name(inf_path: &Path) -> Result<StageResult, StageError> {
    // 检查 INF 文件是否存在
    if !inf_path.exists() {
        let evidence = format!(
            "method=SetupCopyOEMInfW inf_path=\"{}\" error=file_not_found",
            inf_path.display()
        );
        return Err(StageError {
            message: format!("INF 文件不存在: {}", inf_path.display()),
            win32_error: 0,
            evidence,
        });
    }

    // 将 inf_path 转换为绝对路径
    let inf_path_abs = match inf_path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            let evidence = format!(
                "method=SetupCopyOEMInfW inf_path=\"{}\" error=cannot_canonicalize err=\"{}\"",
                inf_path.display(),
                e
            );
            return Err(StageError {
                message: format!("无法解析 INF 文件路径: {}", e),
                win32_error: 0,
                evidence,
            });
        }
    };

    // 获取 inf_path 的父目录作为 OEMSourceMediaLocation
    let inf_dir = match inf_path_abs.parent() {
        Some(dir) => dir.to_path_buf(),
        None => {
            let evidence = format!(
                "method=SetupCopyOEMInfW inf_path=\"{}\" error=no_parent_directory",
                inf_path_abs.display()
            );
            return Err(StageError {
                message: format!("无法获取 INF 文件所在目录: {}", inf_path_abs.display()),
                win32_error: 0,
                evidence,
            });
        }
    };

    // 转换为 HSTRING（windows crate 推荐的方式）
    // HSTRING 可以从 &Path 创建，需要将 PathBuf 转换为 &Path
    let inf_path_hstring = HSTRING::from(inf_path_abs.as_path());
    let inf_dir_hstring = HSTRING::from(inf_dir.as_path());

    // 准备接收 published name 的缓冲区（足够大，例如 1024 字符）
    const BUFFER_SIZE: usize = 1024;
    let mut destination_buffer = vec![0u16; BUFFER_SIZE];
    let mut required_size: u32 = 0;
    // component_ptr 是指向 PWSTR 的指针，即 *mut *mut u16
    // 由于我们只需要 published name，不需要 component，可以传递 None
    // 如果需要 component，可以传递 Some(&mut component_ptr)，其中 component_ptr: *mut *mut u16

    // 调用 SetupCopyOEMInfW
    // windows crate 0.52 的 SetupCopyOEMInfW 使用 HSTRING 和正确的枚举类型
    let result = unsafe {
        SetupCopyOEMInfW(
            &inf_path_hstring,
            &inf_dir_hstring,
            SPOST_PATH,
            SP_COPY_STYLE(0),
            Some(&mut destination_buffer[..]),
            Some(std::ptr::addr_of_mut!(required_size)),
            None, // DestinationInfFileNameComponent 是可选的，我们只需要 published name
        )
    };

    // 检查结果 - SetupCopyOEMInfW 返回 Result<(), Error>
    let success = result.is_ok();

    if !success {
        let win32_error = unsafe { WindowsError::from_win32().code().0 as u32 };
        
        // 如果缓冲区太小，可以尝试使用更大的缓冲区
        if win32_error == 122 { // ERROR_INSUFFICIENT_BUFFER
            let mut larger_buffer = vec![0u16; required_size as usize];
            let mut new_required_size: u32 = 0;
            // component_ptr 是指向 PWSTR 的指针，即 *mut *mut u16
            let retry_result = unsafe {
                SetupCopyOEMInfW(
                    &inf_path_hstring,
                    &inf_dir_hstring,
                    SPOST_PATH,
                    SP_COPY_STYLE(0),
                    Some(&mut larger_buffer[..]),
                    Some(std::ptr::addr_of_mut!(new_required_size)),
                    None, // DestinationInfFileNameComponent 是可选的
                )
            };
            
            let retry_success = retry_result.is_ok();
            
            if retry_success {
                // 从缓冲区提取 published name
                let published_name = extract_wide_string(&larger_buffer);
                return validate_and_create_result(
                    &published_name,
                    false,
                    &inf_path_abs,
                    &inf_dir,
                );
            }
        }
        
        let evidence = format!(
            "method=SetupCopyOEMInfW inf_path=\"{}\" source_media_location=\"{}\" win32_error=0x{:08X}",
            inf_path_abs.display(),
            inf_dir.display(),
            win32_error
        );
        
        return Err(StageError {
            message: format!("SetupCopyOEMInfW 失败 (Win32 error: 0x{:08X})", win32_error),
            win32_error,
            evidence,
        });
    }

    // 从缓冲区提取 published name
    let published_name = extract_wide_string(&destination_buffer);
    
    // 验证并创建结果
    validate_and_create_result(&published_name, false, &inf_path_abs, &inf_dir)
}

/// 从 UTF-16 缓冲区提取字符串（到 null 终止符）
fn extract_wide_string(buffer: &[u16]) -> String {
    // 找到 null 终止符的位置
    let null_pos = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
    
    // 提取到 null 终止符的部分
    let wide_str = &buffer[..null_pos];
    
    // 转换为 String
    String::from_utf16_lossy(wide_str)
}

/// 验证 published_inf_path 存在并创建 StageResult
fn validate_and_create_result(
    published_name: &str,
    used_fallback: bool,
    inf_path: &Path,
    inf_dir: &Path,
) -> Result<StageResult, StageError> {
    // 检查 published_name 是否为空
    if published_name.is_empty() {
        let win32_error = unsafe { WindowsError::from_win32().code().0 as u32 };
        let evidence = format!(
            "method=SetupCopyOEMInfW inf_path=\"{}\" error=empty_published_name win32_error=0x{:08X}",
            inf_path.display(),
            win32_error
        );
        return Err(StageError {
            message: "SetupCopyOEMInfW 返回空的 published name".to_string(),
            win32_error,
            evidence,
        });
    }

    // 构建 published_inf_path
    let published_inf_path = PathBuf::from(r"C:\Windows\INF").join(published_name);

    // 验证文件是否存在
    if !published_inf_path.exists() {
        let win32_error = unsafe { WindowsError::from_win32().code().0 as u32 };
        let evidence = format!(
            "method=SetupCopyOEMInfW inf_path=\"{}\" source_media_location=\"{}\" published_name=\"{}\" published_inf_path=\"{}\" error=file_not_found_after_copy win32_error=0x{:08X}",
            inf_path.display(),
            inf_dir.display(),
            published_name,
            published_inf_path.display(),
            win32_error
        );
        return Err(StageError {
            message: format!(
                "SetupCopyOEMInfW 成功但 published INF 文件不存在: {}",
                published_inf_path.display()
            ),
            win32_error,
            evidence,
        });
    }

    // 创建证据信息
    let evidence = format!(
        "method=SetupCopyOEMInfW inf_path=\"{}\" source_media_location=\"{}\" published_name=\"{}\" published_inf_path=\"{}\" win32_error=0",
        inf_path.display(),
        inf_dir.display(),
        published_name,
        published_inf_path.display()
    );

    Ok(StageResult {
        published_name: published_name.to_string(),
        published_inf_path,
        evidence,
        used_fallback,
    })
}
