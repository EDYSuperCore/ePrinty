use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // 先执行 tauri_build
    tauri_build::build();
    
    // 嵌入 Windows manifest（请求管理员权限）
    embed_windows_manifest();

    // macOS: link against libcups for FFI
    link_cups();
    
    // 将配置文件复制到输出目录（release/debug），使其与可执行文件在同一目录
    // 注意：这个操作是可选的，如果失败不会阻止构建
    // 注意：VBS 脚本已经通过 include_str! 嵌入到 exe 中，不需要复制
    copy_config_file();
}

#[cfg(windows)]
fn embed_windows_manifest() {
    // 注意：由于 Tauri 已经在构建时处理了资源，我们不能在 build.rs 中使用 winres
    // 因为它会与 Tauri 的资源产生冲突（重复的 VERSION 资源）
    // 解决方案：在构建后使用 PowerShell 脚本嵌入 manifest
    
    let manifest_path = Path::new("app.manifest");
    
    if manifest_path.exists() {
        println!("cargo:warning=检测到 Windows manifest 文件: {:?}", manifest_path);
        println!("cargo:warning=由于 Tauri 的资源处理，manifest 将在构建后手动嵌入");
        println!("cargo:warning=请使用 PowerShell 脚本 embed_manifest.ps1 嵌入 manifest");
        println!("cargo:warning=或者右键点击 exe，选择'属性' -> '兼容性' -> '以管理员身份运行此程序'");
    } else {
        println!("cargo:warning=Windows manifest 文件未找到: {:?}", manifest_path);
    }
}

#[cfg(not(windows))]
fn embed_windows_manifest() {
    // 非 Windows 平台不需要 manifest
}

#[cfg(target_os = "macos")]
fn link_cups() {
    println!("cargo:rustc-link-lib=cups");
}

#[cfg(not(target_os = "macos"))]
fn link_cups() {}

fn copy_config_file() {
    // 获取项目根目录（src-tauri 的父目录）
    let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => dir,
        Err(_) => {
            // 如果环境变量未设置，静默退出
            return;
        }
    };
    
    let manifest_path = Path::new(&manifest_dir);
    let project_root = match manifest_path.parent() {
        Some(root) => root,
        None => return,
    };
    
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    
    // 构建目标目录路径：target/{profile}/
    let out_dir = match env::var("OUT_DIR") {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let out_path = Path::new(&out_dir);
    
    // 查找 target/{profile}/ 目录
    // 方案1：查找包含 profile 名称的父目录
    let mut target_profile_dir: Option<std::path::PathBuf> = None;
    
    for ancestor in out_path.ancestors() {
        if let Some(dir_name) = ancestor.file_name().and_then(|n| n.to_str()) {
            if dir_name == profile.as_str() {
                target_profile_dir = Some(ancestor.to_path_buf());
                break;
            }
        }
    }
    
    // 方案2：如果方案1失败，查找 target 目录
    if target_profile_dir.is_none() {
        for ancestor in out_path.ancestors() {
            if let Some(dir_name) = ancestor.file_name().and_then(|n| n.to_str()) {
                if dir_name == "target" {
                    target_profile_dir = Some(ancestor.join(&profile));
                    break;
                }
            }
        }
    }
    
    // 方案3：如果都失败，从 manifest_dir 推断
    if target_profile_dir.is_none() {
        // src-tauri/target/{profile}/
        target_profile_dir = Some(manifest_path.join("target").join(&profile));
    }
    
    if let Some(exe_dir) = target_profile_dir {
        // 源配置文件路径：项目根目录/printer_config.json
        let config_source = project_root.join("printer_config.json");
        
        // 目标路径：target/{profile}/printer_config.json
        let config_dest = exe_dir.join("printer_config.json");
        
        // 如果源文件存在，复制到输出目录
        if config_source.exists() {
            // 确保目标目录存在
            if let Some(parent) = config_dest.parent() {
                if let Err(e) = fs::create_dir_all(parent) {
                    println!("cargo:warning=无法创建目标目录 {:?}: {}", parent, e);
                    return;
                }
            }
            
            // 在 dev 模式下（debug profile），每次都强制复制
            // 这样可以确保开发时修改配置文件后，下次编译会自动更新
            let should_copy = if profile == "debug" {
                // debug 模式：总是复制（确保开发时配置文件总是最新的）
                true
            } else {
                // release 模式：检查源文件是否比目标文件新，或者目标文件不存在
                match (config_source.metadata(), config_dest.metadata()) {
                    (Ok(source_meta), Ok(dest_meta)) => {
                        // 如果源文件比目标文件新，则复制
                        source_meta.modified().unwrap_or(std::time::UNIX_EPOCH) 
                            > dest_meta.modified().unwrap_or(std::time::UNIX_EPOCH)
                    }
                    _ => true, // 如果无法获取元数据或目标文件不存在，总是复制
                }
            };
            
            if should_copy {
                if let Err(e) = fs::copy(&config_source, &config_dest) {
                    println!("cargo:warning=无法复制 printer_config.json 到输出目录: {}", e);
                } else {
                    println!("cargo:warning=已复制 printer_config.json 到 {:?} (profile: {})", config_dest, profile);
                }
            } else {
                println!("cargo:warning=配置文件已是最新，跳过复制 (profile: {})", profile);
            }
        }
    }
}
