#[cfg(windows)]
use encoding_rs::GBK;

/// Windows 编码转换辅助函数（解决中文乱码问题）
/// 
/// 实现逻辑：
/// 1. 首先尝试 UTF-8 解码
/// 2. 如果失败，尝试 GBK 解码（中文 Windows 默认编码）
/// 3. 如果 GBK 解码也有错误，使用 UTF-8 lossy 作为后备
#[cfg(windows)]
pub fn decode_windows_string(bytes: &[u8]) -> String {
    // 尝试 UTF-8 解码
    if let Ok(utf8_str) = String::from_utf8(bytes.to_vec()) {
        return utf8_str;
    }
    
    // 如果 UTF-8 失败，尝试 GBK 解码（中文 Windows 默认编码）
    // 对于非 GBK 编码，使用 lossy 解码以避免崩溃
    let (decoded, _, had_errors) = GBK.decode(bytes);
    let result = decoded.to_string();
    
    // 如果 GBK 解码也有错误，使用 UTF-8 lossy 作为后备
    if had_errors {
        String::from_utf8_lossy(bytes).to_string()
    } else {
        result
    }
}

#[cfg(not(windows))]
pub fn decode_windows_string(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
}

