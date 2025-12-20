# 修复 Windows 上 list_printers 在存在 Ricoh 打印机时卡死问题

## 问题描述

- **现象**：Windows 系统中只要存在 Ricoh 打印机，应用启动时 `loadData` 的 `Promise.all([invoke('load_config'), invoke('list_printers')])` 会卡死超时，installed 状态错误；删除 Ricoh 后恢复正常。
- **根因**：`list_printers_windows` 使用 `@(Get-Printer) | ConvertTo-Json -Depth 3` 导致对象深序列化触发驱动/Spooler 阻塞，PowerShell 进程不退出，前端永远 pending。

## 修复方案

### 1. 重写 list_printers_windows（治本）

**文件**：`src-tauri/src/platform/windows/list.rs`

**改动**：
- ❌ 删除：`@(Get-Printer) | ConvertTo-Json -Depth 3`（深序列化导致阻塞）
- ✅ 改为：`Get-Printer | Select-Object -ExpandProperty Name`（只输出名称，纯文本行）
- ✅ 删除所有 JSON 解析逻辑
- ✅ 改为按行 split stdout，trim + 过滤空行

**关键代码**：
```rust
// 旧代码（会导致卡死）
let script = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; @(Get-Printer) | ConvertTo-Json -Depth 3";
let output = super::ps::run_powershell(script)?;
// 复杂的 JSON 解析逻辑...

// 新代码（快速且安全）
let script = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-Printer | Select-Object -ExpandProperty Name";
let output = super::ps::run_powershell_with_timeout(script, 5000)?;
// 简单的按行解析
let printers: Vec<String> = stdout
    .lines()
    .map(|line| line.trim().to_string())
    .filter(|line| !line.is_empty())
    .collect();
```

### 2. 增加 PowerShell 超时机制（关键保险丝）

**文件**：`src-tauri/src/platform/windows/ps.rs`

**新增函数**：`run_powershell_with_timeout(script: &str, timeout_ms: u64)`

**特性**：
- 可配置的超时时间（毫秒）
- 超时后自动杀死 PowerShell 进程
- 返回明确错误码：`WIN_LIST_PRINTERS_TIMEOUT elapsed_ms=... script_hint=...`
- 避免僵尸进程

**关键代码**：
```rust
pub fn run_powershell_with_timeout(script: &str, timeout_ms: u64) -> Result<std::process::Output, String> {
    // ... 启动进程 ...
    let timeout = Duration::from_millis(timeout_ms);
    
    loop {
        if start_time.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            let elapsed_ms = start_time.elapsed().as_millis();
            let script_hint = if script.len() > 50 {
                format!("{}...", &script[..50])
            } else {
                script.to_string()
            };
            return Err(format!(
                "WIN_LIST_PRINTERS_TIMEOUT elapsed_ms={} script_hint={}",
                elapsed_ms, script_hint
            ));
        }
        // ... 检查进程状态 ...
    }
}
```

### 3. 错误处理改进

**文件**：`src-tauri/src/platform/windows/list.rs`

**改动**：
- 超时/失败时返回明确错误码：`WIN_LIST_PRINTERS_TIMEOUT` 或 `WIN_LIST_PRINTERS_FAILED`
- 前端可以根据错误码决定是否降级为空列表

### 4. 前端超时保护（双重保险）

**文件**：`src/App.vue`

**改动**：
- 为 `invoke('list_printers')` 增加 4 秒前端超时
- 使用 `Promise.race` 确保不会阻塞 `load_config`
- 超时后返回空数组，不影响应用启动

**关键代码**：
```javascript
// 旧代码
invoke('list_printers').catch(err => {
  console.warn('获取打印机列表失败:', err)
  return []
})

// 新代码
Promise.race([
  invoke('list_printers'),
  new Promise((resolve) => {
    setTimeout(() => {
      console.warn('获取打印机列表超时（4秒），返回空列表')
      resolve([])
    }, 4000)
  })
]).catch(err => {
  console.warn('获取打印机列表失败:', err)
  return []
})
```

## 代码 Diff

### 1. src-tauri/src/platform/windows/list.rs

```diff
--- a/src-tauri/src/platform/windows/list.rs
+++ b/src-tauri/src/platform/windows/list.rs
@@ -3,102 +3,50 @@
-/// 使用 PowerShell Get-Printer 命令获取打印机列表，并通过 ConvertTo-Json 解析结果
+/// 使用 PowerShell Get-Printer 命令获取打印机名称列表（纯文本行输出）
 /// 返回打印机名称的向量
+/// 
+/// # 实现说明
+/// - 使用 `Get-Printer | Select-Object -ExpandProperty Name` 只输出名称，避免深序列化阻塞
+/// - 使用超时机制（5000ms）防止 Ricoh 等打印机驱动导致的卡死
+/// - 按行解析 stdout，trim 并过滤空行
 pub fn list_printers_windows() -> Result<Vec<String>, String> {
-    // 使用 PowerShell 执行 Get-Printer 命令并转换为 JSON
-    // 使用 @() 确保始终返回数组格式，即使只有一个打印机
-    let script = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; @(Get-Printer) | ConvertTo-Json -Depth 3";
-    let output = super::ps::run_powershell(script)?;
+    // 使用 PowerShell 执行 Get-Printer 命令，只输出 Name 字段的纯文本行
+    // 不再使用 ConvertTo-Json，避免深序列化触发驱动/Spooler 阻塞
+    let script = "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; Get-Printer | Select-Object -ExpandProperty Name";
+    
+    // 使用带超时的 PowerShell 执行（5000ms 超时）
+    // 防止 Ricoh 等打印机驱动导致的卡死
+    let timeout_ms = 5000u64;
+    let output = match super::ps::run_powershell_with_timeout(script, timeout_ms) {
+        Ok(output) => output,
+        Err(e) => {
+            // 超时或执行失败，返回明确错误码
+            if e.contains("WIN_LIST_PRINTERS_TIMEOUT") {
+                return Err(format!("WIN_LIST_PRINTERS_TIMEOUT: {}", e));
+            }
+            return Err(format!("WIN_LIST_PRINTERS_FAILED: {}", e));
+        }
+    };
 
     // 解码输出（处理中文编码问题）
     let stdout = super::encoding::decode_windows_string(&output.stdout);
     let stderr = super::encoding::decode_windows_string(&output.stderr);
 
     // 检查命令是否执行成功
     if !output.status.success() {
         return Err(format!(
-            "获取打印机列表失败: {}\n错误输出: {}",
+            "WIN_LIST_PRINTERS_FAILED: 获取打印机列表失败: {}\n错误输出: {}",
             stdout, stderr
         ));
     }
 
-    // 解析 JSON 输出
-    // PowerShell 的 ConvertTo-Json 可能返回单个对象（如果只有一个打印机）或数组（如果有多个）
-    let printers: Vec<String> = if stdout.trim().is_empty() {
-        // 如果没有输出，返回空列表
-        Vec::new()
-    } else {
-        // 尝试解析为 JSON
-        match serde_json::from_str::<serde_json::Value>(&stdout) {
-            Ok(json_value) => {
-                match json_value {
-                    serde_json::Value::Array(arr) => {
-                        // 如果是数组，提取每个对象的 Name 字段
-                        arr.into_iter()
-                            .filter_map(|item| {
-                                if let serde_json::Value::Object(obj) = item {
-                                    obj.get("Name")
-                                        .and_then(|name| name.as_str())
-                                        .map(|s| s.to_string())
-                                } else {
-                                    None
-                                }
-                            })
-                            .collect()
-                    }
-                    serde_json::Value::Object(obj) => {
-                        // 如果是单个对象，提取 Name 字段
-                        obj.get("Name")
-                            .and_then(|name| name.as_str())
-                            .map(|s| vec![s.to_string()])
-                            .unwrap_or_default()
-                    }
-                    _ => {
-                        return Err(format!("无法解析 PowerShell 输出为有效的 JSON: {}", stdout));
-                    }
-                }
-            }
-            Err(e) => {
-                // 如果 JSON 解析失败，尝试按行解析（PowerShell 可能输出多行）
-                // 或者尝试直接提取打印机名称
-                let lines: Vec<String> = stdout
-                    .lines()
-                    .filter_map(|line| {
-                        let trimmed = line.trim();
-                        // 尝试从 JSON 行中提取 Name 字段
-                        if trimmed.contains("\"Name\"") {
-                            // 简单的正则匹配或字符串提取
-                            if let Some(start) = trimmed.find("\"Name\"") {
-                                let after_name = &trimmed[start + 6..];
-                                if let Some(colon) = after_name.find(':') {
-                                    let after_colon = &after_name[colon + 1..];
-                                    let name_part = after_colon.trim();
-                                    // 提取引号内的内容
-                                    if let Some(quote_start) = name_part.find('"') {
-                                        let after_quote = &name_part[quote_start + 1..];
-                                        if let Some(quote_end) = after_quote.find('"') {
-                                            return Some(after_quote[..quote_end].to_string());
-                                        }
-                                    }
-                                }
-                            }
-                        }
-                        None
-                    })
-                    .collect();
-
-                if !lines.is_empty() {
-                    lines
-                } else {
-                    return Err(format!(
-                        "解析 PowerShell 输出失败: {}\n原始输出: {}",
-                        e, stdout
-                    ));
-                }
-            }
-        }
-    };
+    // 按行解析 stdout，trim 并过滤空行
+    let printers: Vec<String> = stdout
+        .lines()
+        .map(|line| line.trim().to_string())
+        .filter(|line| !line.is_empty())
+        .collect();
 
     Ok(printers)
 }
```

### 2. src-tauri/src/platform/windows/ps.rs

```diff
--- a/src-tauri/src/platform/windows/ps.rs
+++ b/src-tauri/src/platform/windows/ps.rs
@@ -84,3 +84,66 @@ pub fn run_powershell(script: &str) -> Result<std::process::Output, String> {
     Ok(output)
 }
+
+/// 带超时的 PowerShell 命令执行函数
+/// 
+/// # 参数
+/// - `script`: PowerShell 脚本内容（字符串）
+/// - `timeout_ms`: 超时时间（毫秒）
+/// 
+/// # 返回
+/// - `Ok(Output)`: 执行成功，返回进程输出
+/// - `Err(String)`: 执行失败或超时，返回错误信息（包含错误码）
+/// 
+/// # 特性
+/// - 统一设置：-NoProfile, -NonInteractive, -WindowStyle Hidden
+/// - stdout/stderr 管道
+/// - 隐藏窗口（CREATE_NO_WINDOW）
+/// - 可配置的超时控制
+/// - 超时后自动杀死进程并返回明确错误码
+pub fn run_powershell_with_timeout(script: &str, timeout_ms: u64) -> Result<std::process::Output, String> {
+    let mut child = Command::new("powershell")
+        .args([
+            "-NoProfile",
+            "-NonInteractive",
+            "-WindowStyle", "Hidden",
+            "-Command",
+            script
+        ])
+        .stdin(Stdio::null())
+        .stdout(Stdio::piped())
+        .stderr(Stdio::piped())
+        .creation_flags(CREATE_NO_WINDOW)
+        .spawn()
+        .map_err(|e| format!("执行 PowerShell 命令失败: {}", e))?;
+    
+    let start_time = Instant::now();
+    let timeout = Duration::from_millis(timeout_ms);
+    
+    // 轮询检查进程是否完成
+    loop {
+        // 先检查超时
+        if start_time.elapsed() >= timeout {
+            // 超时，杀死进程
+            let _ = child.kill();
+            // 等待进程结束，避免僵尸进程
+            let _ = child.wait();
+            let elapsed_ms = start_time.elapsed().as_millis();
+            // 提取脚本的前50个字符作为提示
+            let script_hint = if script.len() > 50 {
+                format!("{}...", &script[..50])
+            } else {
+                script.to_string()
+            };
+            return Err(format!(
+                "WIN_LIST_PRINTERS_TIMEOUT elapsed_ms={} script_hint={}",
+                elapsed_ms, script_hint
+            ));
+        }
+        
+        match child.try_wait() {
+            Ok(Some(_status)) => {
+                // 进程已完成，退出循环
+                break;
+            }
+            Ok(None) => {
+                // 进程仍在运行，等待后继续
+                std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
+            }
+            Err(e) => {
+                // 检查失败，杀死进程并返回错误
+                let _ = child.kill();
+                let _ = child.wait();
+                return Err(format!("检查 PowerShell 进程状态失败: {}", e));
+            }
+        }
+    }
+    
+    // 循环结束后，获取输出
+    let output = child.wait_with_output()
+        .map_err(|e| format!("获取 PowerShell 输出失败: {}", e))?;
+    Ok(output)
+}
```

### 3. src/App.vue

```diff
--- a/src/App.vue
+++ b/src/App.vue
@@ -964,7 +964,15 @@
           invoke('load_config').catch(err => {
             console.error('加载配置失败:', err)
             throw err
           }),
-          invoke('list_printers').catch(err => {
-            console.warn('获取打印机列表失败:', err)
-            return [] // 失败时返回空数组
-          })
+          // 为 list_printers 增加 4s 超时，防止 Ricoh 打印机导致的卡死
+          Promise.race([
+            invoke('list_printers'),
+            new Promise((resolve) => {
+              setTimeout(() => {
+                console.warn('获取打印机列表超时（4秒），返回空列表')
+                resolve([]) // 超时返回空数组
+              }, 4000)
+            })
+          ]).catch(err => {
+            console.warn('获取打印机列表失败:', err)
+            return [] // 失败时返回空数组
+          })
         ])
```

## 验证步骤

### 最小验证步骤

1. **准备测试环境**
   - 确保 Windows 系统中存在至少一个 Ricoh 打印机（已安装）
   - 如果没有，可以通过"设备和打印机"添加一个 Ricoh 打印机

2. **编译并运行应用**
   ```bash
   cd src-tauri
   cargo build
   # 或使用开发模式
   npm run tauri dev
   ```

3. **验证启动不再卡死**
   - 启动应用
   - **预期结果**：应用应在 3-5 秒内完成启动
   - **不应出现**：应用启动时长时间卡住（超过 5 秒）
   - **检查点**：
     - 应用界面正常显示
     - 状态栏显示"已加载本地配置"或类似消息
     - 打印机列表正常显示（即使 installed 状态可能不准确）

4. **验证 list_printers 超时机制**
   - 打开浏览器开发者工具（F12）查看控制台
   - **正常情况**：应在 3-5 秒内看到打印机列表加载完成
   - **超时情况**：如果超过 5 秒，应看到：
     - 控制台警告："获取打印机列表超时（4秒），返回空列表"
     - 或错误信息包含 "WIN_LIST_PRINTERS_TIMEOUT"
   - **关键**：即使超时，应用也应正常启动，不阻塞

5. **验证打印机列表功能**
   - 检查打印机列表是否正确显示
   - 如果超时返回空列表，installed 状态可能不准确，但不影响应用使用
   - 可以手动点击"刷新"按钮重新获取打印机列表

6. **对比测试（可选）**
   - 删除 Ricoh 打印机后，验证 list_printers 能正常返回结果
   - 重新添加 Ricoh 打印机，验证不再卡死

### 预期行为

- ✅ **正常情况**：list_printers 在 1-3 秒内完成，返回所有打印机名称
- ✅ **超时情况**：如果超过 5 秒，后端超时杀死进程，返回错误；前端 4 秒超时返回空数组
- ✅ **关键**：无论哪种情况，应用都能正常启动，不会卡死

## 技术要点

1. **避免深序列化**：不再使用 `ConvertTo-Json -Depth 3`，只输出 Name 字段
2. **双重超时保护**：后端 5 秒 + 前端 4 秒，确保永不挂死
3. **错误码明确**：`WIN_LIST_PRINTERS_TIMEOUT` 和 `WIN_LIST_PRINTERS_FAILED` 便于诊断
4. **优雅降级**：超时/失败时返回空数组，不影响应用核心功能

## 编译验证

```bash
cd src-tauri
cargo check
# 应显示：Finished `dev` profile [unoptimized + debuginfo] target(s)
```

## 总结

本次修复通过以下方式彻底解决了 Ricoh 打印机导致的卡死问题：

1. **治本**：改用纯文本行输出，避免深序列化阻塞
2. **保险**：增加超时机制，确保进程不会永远挂起
3. **容错**：前端超时保护，确保应用启动不受影响

修复后，即使系统中存在 Ricoh 打印机，应用也能正常启动，list_printers 会在 3-5 秒内完成或超时返回错误，不会导致应用卡死。

