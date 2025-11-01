use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // 先执行 tauri_build
    tauri_build::build();
    
    // 将配置文件复制到输出目录（release/debug），使其与可执行文件在同一目录
    // 注意：这个操作是可选的，如果失败不会阻止构建
    // 注意：VBS 脚本已经通过 include_str! 嵌入到 exe 中，不需要复制
    copy_config_file();
    // copy_scripts_files(); // 不再需要，VBS 已嵌入到 exe 中
}

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
            
            if let Err(e) = fs::copy(&config_source, &config_dest) {
                println!("cargo:warning=无法复制 printer_config.json 到输出目录: {}", e);
            } else {
                println!("cargo:warning=已复制 printer_config.json 到 {:?}", config_dest);
            }
        }
    }
}

fn copy_scripts_files() {
    // 获取项目根目录（src-tauri 的父目录）
    let manifest_dir = match env::var("CARGO_MANIFEST_DIR") {
        Ok(dir) => dir,
        Err(_) => {
            return;
        }
    };
    
    let manifest_path = Path::new(&manifest_dir);
    let profile = env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    
    // 查找 target/{profile}/ 目录
    let out_dir = match env::var("OUT_DIR") {
        Ok(dir) => dir,
        Err(_) => return,
    };
    let out_path = Path::new(&out_dir);
    
    let mut target_profile_dir: Option<std::path::PathBuf> = None;
    
    for ancestor in out_path.ancestors() {
        if let Some(dir_name) = ancestor.file_name().and_then(|n| n.to_str()) {
            if dir_name == profile.as_str() {
                target_profile_dir = Some(ancestor.to_path_buf());
                break;
            }
        }
    }
    
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
    
    if target_profile_dir.is_none() {
        target_profile_dir = Some(manifest_path.join("target").join(&profile));
    }
    
    if let Some(exe_dir) = target_profile_dir {
        // 源脚本文件路径：src-tauri/scripts/
        let scripts_source_dir = manifest_path.join("scripts");
        
        // 目标路径：target/{profile}/scripts/
        let scripts_dest_dir = exe_dir.join("scripts");
        
        // 创建 scripts 目录
        if let Err(e) = fs::create_dir_all(&scripts_dest_dir) {
            println!("cargo:warning=无法创建 scripts 目录 {:?}: {}", scripts_dest_dir, e);
            return;
        }
        
        // 复制 prnport.vbs
        let vbs_source = scripts_source_dir.join("prnport.vbs");
        let vbs_dest = scripts_dest_dir.join("prnport.vbs");
        if vbs_source.exists() {
            if let Err(e) = fs::copy(&vbs_source, &vbs_dest) {
                println!("cargo:warning=无法复制 prnport.vbs 到输出目录: {}", e);
            } else {
                println!("cargo:warning=已复制 prnport.vbs 到 {:?}", vbs_dest);
            }
        }
        
        // 复制 ricoh320.pdd（如果有）
        let pdd_source = scripts_source_dir.join("ricoh320.pdd");
        let pdd_dest = scripts_dest_dir.join("ricoh320.pdd");
        if pdd_source.exists() {
            if let Err(e) = fs::copy(&pdd_source, &pdd_dest) {
                println!("cargo:warning=无法复制 ricoh320.pdd 到输出目录: {}", e);
            } else {
                println!("cargo:warning=已复制 ricoh320.pdd 到 {:?}", pdd_dest);
            }
        }
        
        // 同时也在根目录复制一份（作为备用）
        let vbs_dest_root = exe_dir.join("prnport.vbs");
        if vbs_source.exists() && !vbs_dest_root.exists() {
            if let Err(e) = fs::copy(&vbs_source, &vbs_dest_root) {
                println!("cargo:warning=无法复制 prnport.vbs 到根目录: {}", e);
            }
        }
    }
}
