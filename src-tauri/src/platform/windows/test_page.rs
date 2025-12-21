use std::fs;
use std::io::Write;

/// Windows 平台打印测试页实现
pub fn print_test_page_windows(printer_name: String) -> Result<String, String> {
    // 先验证打印机是否存在
    let check_script = format!("Get-Printer -Name '{}' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Name", 
        printer_name.replace("'", "''"));
    
    let printer_exists = match crate::platform::windows::ps::run_powershell(&check_script) {
        Ok(output) => {
            let stdout = crate::platform::windows::encoding::decode_windows_string(&output.stdout);
            output.status.success() && !stdout.trim().is_empty()
        }
        Err(_) => false
    };
    
    if !printer_exists {
        return Err(format!("打印机不存在或未连接: {}", printer_name));
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
    
    // 写入测试内容（使用 UTF-8 BOM）
    {
        let mut file = fs::File::create(&temp_file)
            .map_err(|e| format!("创建临时文件失败: {}", e))?;
        
        // 写入 UTF-8 BOM
        file.write_all(&[0xEF, 0xBB, 0xBF])
            .map_err(|e| format!("写入 UTF-8 BOM 失败: {}", e))?;
        
        // 写入测试内容
        file.write_all(test_content.as_bytes())
            .map_err(|e| format!("写入测试内容失败: {}", e))?;
        
        file.sync_all()
            .map_err(|e| format!("同步文件失败: {}", e))?;
    }
    
    // 使用 PowerShell 打印文件到指定打印机
    let ps_command = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $content = Get-Content '{}' -Encoding UTF8 -Raw; $content | Out-Printer -Name '{}'; if ($LASTEXITCODE -eq 0 -or $?) {{ Write-Output 'Success' }} else {{ Write-Error '打印失败' }}",
        temp_file.to_str().unwrap().replace("'", "''").replace("\\", "\\\\"),
        printer_name.replace("'", "''").replace("\\", "\\\\")
    );
    
    // 清理临时文件
    let _ = fs::remove_file(&temp_file);
    
    let output = crate::platform::windows::ps::run_powershell(&ps_command)?;
    let stdout = crate::platform::windows::encoding::decode_windows_string(&output.stdout);
    let stderr = crate::platform::windows::encoding::decode_windows_string(&output.stderr);
    
    if output.status.success() || stdout.contains("Success") {
        Ok(format!("测试页已发送到打印机: {}", printer_name))
    } else {
        // 如果 Out-Printer 失败，尝试使用 CIM 方法作为备选
        let ps_command2 = format!(
            "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; $printer = Get-CimInstance -ClassName Win32_Printer -Filter \"Name='{}'\" -ErrorAction SilentlyContinue; if ($printer) {{ try {{ Invoke-CimMethod -InputObject $printer -MethodName PrintTestPage -ErrorAction Stop; Write-Output 'Success' }} catch {{ Write-Error $_.Exception.Message }} }} else {{ Write-Error '打印机不存在或未连接' }} }}",
            printer_name.replace("'", "''").replace("\\", "\\\\")
        );
        
        match crate::platform::windows::ps::run_powershell(&ps_command2) {
            Ok(output) if output.status.success() || crate::platform::windows::encoding::decode_windows_string(&output.stdout).contains("Success") => {
                Ok(format!("测试页已发送到打印机: {}", printer_name))
            }
            Ok(output) => {
                let stderr2 = crate::platform::windows::encoding::decode_windows_string(&output.stderr);
                Err(format!("打印测试页失败: {}. 请检查打印机是否在线且名称是否正确。", 
                    if !stderr2.trim().is_empty() { 
                        stderr2.trim() 
                    } else if !stderr.trim().is_empty() { 
                        stderr.trim() 
                    } else { 
                        "未知错误" 
                    }))
            }
            Err(e) => {
                Err(format!("打印测试页失败: {}. 请确保打印机已连接并可以访问。", e))
            }
        }
    }
}

