use std::fs;
use std::io::Write;

/// Windows 平台打印测试页实现
/// 
/// 日志封装入口：本函数内的所有 eprintln! 调用
pub fn print_test_page_windows(printer_name: String) -> Result<String, String> {
    eprintln!("[PrintTestPage] START printer_name=\"{}\"", printer_name);
    
    // 先验证打印机是否存在（仅一次检查）
    // 修复：使用 Where-Object 精确过滤，避免 Get-Printer -Name 的通配符匹配导致误判
    let escaped_printer_name = printer_name.replace("'", "''");
    let check_script = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $q = '{}'; $printer = Get-Printer -Name $q -ErrorAction SilentlyContinue | Where-Object {{ $_.Name -eq $q }} | Select-Object -ExpandProperty Name",
        escaped_printer_name
    );
    
    let printer_exists = match crate::platform::windows::ps::run_powershell(&check_script) {
        Ok(output) => {
            let stdout = crate::platform::windows::encoding::decode_windows_string(&output.stdout);
            let exit_code = output.status.code();
            // 二次确认：验证返回的名称是否完全等于 printer_name
            let exists = exit_code == Some(0) 
                && !stdout.trim().is_empty() 
                && stdout.trim() == printer_name;
            eprintln!("[PrintTestPage] CHECK_EXISTS result exists={} exit_code={:?} stdout=\"{}\" expected=\"{}\"", 
                exists, exit_code, stdout.trim(), printer_name);
            exists
        }
        Err(e) => {
            eprintln!("[PrintTestPage] CHECK_EXISTS result exists=false error=\"{}\"", e);
            false
        }
    };
    
    if !printer_exists {
        // 失败时输出证据：获取打印机详细信息
        let _ = print_printer_evidence(&printer_name, "CHECK_EXISTS");
        return Err(format!("[PrintTestPage] ERROR step=CHECK_EXISTS message=打印机不存在或未连接: {}", printer_name));
    }
    
    // 生成测试页内容
    let now = chrono::Local::now();
    let test_content = format!(
r#"
====================================================
    ePrinty 测试页 
====================================================

别担心，我不是广告，我只是来测试你的打印机 


Hello 小伙伴！

恭喜你，你的打印机已经安装成功啦！


1 文字测试：

我是一行可爱的测试文字。

我也是一行幽默的测试文字。

请放心打印，我不会跑掉的。


2 ASCII 艺术测试：

     (\\_/)
     ( •_•)
     />  ePrinty 向你问好！


3 信息测试：

打印机名称：{printer_name}

测试打印时间：{test_time}


4 字符集测试：

中文：你好世界！

English: Hello World!

数字：0123456789

特殊字符：!@#$%^&*()_+-=[]{{}}|;:',.<>?


====================================================
 小提示：

如果你能看到这个页面，说明打印机工作正常 

打印机越开心，工作效率越高哦!

— ePrinty，让打印这件事，简单一点 —
====================================================
"#,
        printer_name = printer_name,
        test_time = now.format("%Y年%m月%d日 %H:%M:%S")
    );
    
    // 创建临时文件
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join(format!("printer_test_{}.txt", std::process::id()));
    let temp_file_display = super::log::sanitize_path(&temp_file);
    eprintln!("[PrintTestPage] TEMP_FILE_CREATE path=\"{}\"", temp_file_display);
    
    // 写入测试内容（使用 UTF-8 BOM）
    {
        let mut file = fs::File::create(&temp_file)
            .map_err(|e| format!("[PrintTestPage] ERROR step=TEMP_FILE_CREATE message=创建临时文件失败: {}", e))?;
        
        // 写入 UTF-8 BOM
        file.write_all(&[0xEF, 0xBB, 0xBF])
            .map_err(|e| format!("[PrintTestPage] ERROR step=TEMP_FILE_CREATE message=写入 UTF-8 BOM 失败: {}", e))?;
        
        // 写入测试内容
        file.write_all(test_content.as_bytes())
            .map_err(|e| format!("[PrintTestPage] ERROR step=TEMP_FILE_CREATE message=写入测试内容失败: {}", e))?;
        
        file.sync_all()
            .map_err(|e| format!("[PrintTestPage] ERROR step=TEMP_FILE_CREATE message=同步文件失败: {}", e))?;
    }
    
    let file_metadata = fs::metadata(&temp_file)
        .map_err(|e| format!("[PrintTestPage] ERROR step=TEMP_FILE_CREATE message=获取文件元数据失败: {}", e))?;
    let file_size = file_metadata.len();
    eprintln!("[PrintTestPage] TEMP_FILE_CREATE result size={} bytes", file_size);
    
    // 使用 PowerShell 打印文件到指定打印机（主路径，无 fallback）
    let ps_command = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $content = Get-Content '{}' -Encoding UTF8 -Raw; $content | Out-Printer -Name '{}'; if ($LASTEXITCODE -eq 0 -or $?) {{ Write-Output 'Success' }} else {{ Write-Error '打印失败' }}",
        temp_file.to_str().unwrap().replace("'", "''").replace("\\", "\\\\"),
        printer_name.replace("'", "''").replace("\\", "\\\\")
    );
    
    // 检查是否保留临时文件（调试开关）
    let keep_file = std::env::var("EPRINTY_DEBUG_KEEP_TESTPAGE_FILE")
        .map(|v| v == "1")
        .unwrap_or(false);
    
    let output = match crate::platform::windows::ps::run_powershell(&ps_command) {
        Ok(output) => output,
        Err(e) => {
            // PowerShell 执行失败，输出证据并清理临时文件
            let _ = print_printer_evidence(&printer_name, "PS_EXEC");
            if !keep_file {
                let _ = fs::remove_file(&temp_file);
            } else {
                eprintln!("[PrintTestPage] TEMP_FILE_DELETE skipped (debug mode) path=\"{}\"", temp_file_display);
            }
            return Err(format!("[PrintTestPage] ERROR step=PS_EXEC message={}", e));
        }
    };
    
    let stdout = crate::platform::windows::encoding::decode_windows_string(&output.stdout);
    let stderr = crate::platform::windows::encoding::decode_windows_string(&output.stderr);
    
    // 使用统一的截断函数（默认 2000 字符）
    let stdout_display = super::log::truncate(&stdout, 2000);
    let stderr_display = super::log::truncate(&stderr, 2000);
    
    eprintln!("[PrintTestPage] EXEC result exit_code={:?} stdout_len={} stderr_len={}", 
        output.status.code(), output.stdout.len(), output.stderr.len());
    
    // 清理临时文件（除非调试模式）
    if !keep_file {
        match fs::remove_file(&temp_file) {
            Ok(_) => eprintln!("[PrintTestPage] TEMP_FILE_DELETE success path=\"{}\"", temp_file_display),
            Err(e) => eprintln!("[PrintTestPage] TEMP_FILE_DELETE failed path=\"{}\" error=\"{}\"", temp_file_display, e),
        }
    } else {
        eprintln!("[PrintTestPage] TEMP_FILE_DELETE skipped (debug mode) path=\"{}\"", temp_file_display);
        eprintln!("[PrintTestPage] DEBUG: 已保留文件用于排查：path=\"{}\"", temp_file_display);
    }
    
    if output.status.success() || stdout.contains("Success") {
        eprintln!("[PrintTestPage] SUCCESS printer_name=\"{}\"", printer_name);
        Ok(format!("测试页已发送到打印机: {}", printer_name))
    } else {
        // 失败时输出证据和详细错误
        let _ = print_printer_evidence(&printer_name, "PS_RESULT");
        let error_summary = if !stderr_display.trim().is_empty() {
            stderr_display.trim()
        } else if !stdout_display.trim().is_empty() {
            stdout_display.trim()
        } else {
            "未知错误"
        };
        eprintln!("[PrintTestPage] FAILURE printer_name=\"{}\" exit_code={:?} stderr=\"{}\"", 
            printer_name, output.status.code(), error_summary);
        Err(format!("[PrintTestPage] ERROR step=PS_RESULT message=打印测试页失败: {}. 请检查打印机是否在线且名称是否正确。", error_summary))
    }
}

/// 打印失败时的证据信息（仅失败时调用）
/// 
/// # 参数
/// - `printer_name`: 打印机名称
/// - `step`: 失败步骤
fn print_printer_evidence(printer_name: &str, step: &str) {
    // 获取打印机详细信息（DriverName、PortName）
    let info_script = format!(
        "Get-Printer -Name '{}' -ErrorAction SilentlyContinue | Select-Object Name, DriverName, PortName | ConvertTo-Json -Compress",
        printer_name.replace("'", "''")
    );
    
    if let Ok(output) = crate::platform::windows::ps::run_powershell(&info_script) {
        let stdout = crate::platform::windows::encoding::decode_windows_string(&output.stdout);
        if !stdout.trim().is_empty() {
            let info_display = super::log::truncate(&stdout, 500);
            eprintln!("[PrintTestPage] EVIDENCE step={} printer_info=\"{}\"", step, info_display);
        }
    }
    
    // 检查权限问题（Access Denied）
    let perm_script = format!(
        "Get-Printer -Name '{}' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty PermissionSDDL",
        printer_name.replace("'", "''")
    );
    
    if let Ok(output) = crate::platform::windows::ps::run_powershell(&perm_script) {
        let stdout = crate::platform::windows::encoding::decode_windows_string(&output.stdout);
        if !stdout.trim().is_empty() {
            let perm_display = super::log::truncate(&stdout, 4000);
            eprintln!("[PrintTestPage] EVIDENCE step={} permission_sddl=\"{}\"", step, perm_display);
        }
    }
}

