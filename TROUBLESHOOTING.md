# 故障排除指南

## 常见问题及解决方案

### 问题 1: 终端中无法运行 rustc/npm，但独立 CMD 可以运行

**症状：**
- 在 VS Code 集成终端中运行 `rustc --version` 或 `npm --version` 报错
- 在独立打开的 CMD 或 PowerShell 中可以正常运行
- 错误信息：`无法将"xxx"项识别为 cmdlet、函数、脚本文件或可运行程序的名称`

**原因：**
VS Code 的集成终端在启动时缓存了环境变量，安装新软件后新添加的 PATH 没有生效。

**解决方案：**

#### 方法 1：重启 VS Code（最简单）✅ **强烈推荐**

1. 完全关闭 VS Code（关闭所有窗口）
2. 重新打开 VS Code
3. 打开终端测试：
   ```powershell
   node --version
   npm --version
   rustc --version
   ```

#### 方法 2：刷新 PowerShell 环境变量（一次性解决所有问题）✅

在 VS Code 的 PowerShell 终端中运行：

```powershell
# 刷新所有环境变量（包括 Node.js、npm、Rust 等）
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# 验证 Node.js 和 npm
node --version
npm --version

# 验证 Rust（如果已安装）
rustc --version
cargo --version
```

**这是最快捷的解决方法，一次刷新所有环境变量！**

#### 方法 3：手动添加 PATH（临时）

如果方法 2 不起作用，可以手动添加：

```powershell
# Node.js（通常在这个位置）
$nodePath = "C:\Program Files\nodejs"
if (Test-Path $nodePath) {
    $env:Path += ";$nodePath"
}

# Rust（替换 <你的用户名> 为实际用户名）
$rustPath = "C:\Users\<你的用户名>\.cargo\bin"
if (Test-Path $rustPath) {
    $env:Path += ";$rustPath"
}

# 验证
node --version
npm --version
rustc --version
```

**注意：** 这只是临时生效，关闭终端后失效。

#### 方法 4：使用完整路径测试

确认软件确实已安装：

```powershell
# 测试 Node.js（替换为实际路径）
& "C:\Program Files\nodejs\node.exe" --version

# 测试 npm
& "C:\Program Files\nodejs\npm.cmd" --version

# 测试 Rust（替换 <你的用户名>）
C:\Users\<你的用户名>\.cargo\bin\rustc.exe --version
```

如果能运行，说明问题只是 PATH 未生效。

#### 方法 5：查找 Node.js 安装位置

如果不确定 Node.js 安装在哪里：

```powershell
# 在独立 CMD 中运行
where node
where npm

# 或在 PowerShell 中查找
Get-Command node | Select-Object -ExpandProperty Source
Get-Command npm | Select-Object -ExpandProperty Source
```

然后将找到的路径添加到 PATH。

### 问题 1B: Rust 已安装但 rustc 命令无效

**症状：**
- 在独立 CMD 中 `rustc --version` 也无效
- 或者 VS Code 重启后 `rustc` 仍然无效
- Rust 文件存在：`C:\Users\<用户名>\.cargo\bin\rustc.exe`

**原因：**
Rust 安装后没有正确添加到 PATH 环境变量，或者 PATH 设置失败。

**解决方案：**

#### 方法 1：手动添加到 PATH（推荐）

1. **检查 Rust 是否已安装**：
   ```powershell
   # 替换 <你的用户名> 为实际用户名
   Test-Path "C:\Users\<你的用户名>\.cargo\bin\rustc.exe"
   ```

2. **手动添加到 PATH**：
   
   a. 打开"系统属性" → "高级" → "环境变量"
   
   b. 在"用户变量"中找到 `Path`（如果没有则新建）
   
   c. 点击"编辑"，点击"新建"，添加：
      ```
      C:\Users\<你的用户名>\.cargo\bin
      ```
   
   d. 点击"确定"保存所有对话框
   
   e. **完全关闭并重新打开 VS Code**

3. **验证**：
   ```powershell
   rustc --version
   cargo --version
   ```

#### 方法 2：使用脚本自动修复

运行项目根目录下的修复脚本：

```powershell
.\fix_rust_path.ps1
```

这个脚本会：
- 检查 Rust 安装
- 自动添加到用户 PATH
- 添加到当前会话 PATH（立即生效）

#### 方法 3：重新安装 Rust

如果以上方法都不行，可以重新安装 Rust：

1. 访问 https://rustup.rs/ 下载安装程序
2. 运行 `rustup-init.exe`
3. **重要**：安装完成后选择"默认"选项（选项 1）
4. 安装完成后**完全重启 VS Code**

#### 方法 4：使用 rustup 修复

如果 rustup 可用，可以尝试：

```powershell
# 查找 rustup（可能在 PATH 中）
where.exe rustup

# 如果找到，运行修复
rustup self uninstall
# 然后重新安装 Rust
```

#### 方法 5：临时使用（当前会话）

如果只是临时需要，可以在当前 PowerShell 会话中添加：

```powershell
# 添加到当前会话 PATH（只对当前终端有效）
$env:Path += ";C:\Users\<你的用户名>\.cargo\bin"

# 验证
rustc --version
```

**注意**：这只是临时生效，关闭终端后失效。

#### 方法 5：修改 PowerShell 配置文件（永久）

如果你想每次打开 PowerShell 都自动加载 Rust：

1. 检查是否有配置文件：
   ```powershell
   Test-Path $PROFILE
   ```

2. 如果没有，创建配置文件：
   ```powershell
   New-Item -Path $PROFILE -Type File -Force
   ```

3. 编辑配置文件：
   ```powershell
   notepad $PROFILE
   ```

4. 添加以下内容（替换 `<你的用户名>`）：
   ```powershell
   # 添加 Rust 到 PATH
   $RustPath = "C:\Users\<你的用户名>\.cargo\bin"
   if (Test-Path $RustPath) {
       $env:Path += ";$RustPath"
   }
   ```

5. 保存文件，重新加载配置：
   ```powershell
   . $PROFILE
   ```

### 问题 2: npm run tauri dev 失败

**可能的原因和解决方案：**

#### 原因 A: Rust 未正确安装

```powershell
# 检查 Rust 是否可用
rustc --version
cargo --version

# 如果失败，参考问题 1 的解决方案
```

#### 原因 B: Node.js 依赖未安装

```powershell
# 重新安装依赖
npm install
```

#### 原因 C: Tauri CLI 未正确安装

```powershell
# 重新安装 Tauri CLI
npm install --save-dev @tauri-apps/cli

# 或全局安装
npm install -g @tauri-apps/cli
```

#### 原因 D: 端口被占用

```powershell
# 检查端口占用（默认 1420）
netstat -ano | findstr :1420

# 如果需要，修改 vite.config.js 中的端口号
```

### 问题 3: 构建失败 - 找不到 Visual Studio Build Tools

**症状：**
```
error: linker `link.exe` not found
```

**解决方案：**

1. **自动安装（推荐）**：
   - 重新运行 `rustup-init.exe`
   - 选择安装 Visual Studio Build Tools

2. **手动安装**：
   - 下载并安装 "Build Tools for Visual Studio"
   - 选择 "Desktop development with C++" 工作负载

3. **使用其他工具链**（不推荐）：
   ```powershell
   rustup toolchain install stable-x86_64-pc-windows-gnu
   rustup default stable-x86_64-pc-windows-gnu
   ```

### 问题 4: 网络请求失败

**症状：**
- `load_config()` 返回网络错误
- 无法从服务器加载配置

**解决方案：**

#### 检查服务器 URL

1. 确认 `src-tauri/src/main.rs` 中的 URL 正确
2. 在浏览器中测试 URL 是否可以访问
3. 检查防火墙设置

#### 使用本地测试服务器

参考 `run_local_server.md` 设置本地服务器。

#### 检查 Tauri 权限配置

确保 `src-tauri/tauri.conf.json` 中的 HTTP allowlist 配置正确：

```json
{
  "tauri": {
    "allowlist": {
      "http": {
        "all": false,
        "request": true,
        "scope": ["http://**", "https://**"]
      }
    }
  }
}
```

### 问题 5: 打印机安装失败

**症状：**
- 点击"安装"按钮后显示错误
- 打印机未安装成功

**解决方案：**

#### 原因 A: 权限不足

**解决：** 以管理员身份运行应用

#### 原因 B: 打印机路径错误

**检查：**
1. 确认打印机路径格式正确：`\\\\server\\printer`
2. 测试网络打印机是否可以访问
3. 检查路径中的服务器名称是否正确

#### 原因 C: 网络连接问题

**解决：**
1. 确保网络打印机可访问
2. 检查网络连接
3. 确认打印机共享设置正确

#### 原因 D: rundll32 命令格式问题

**检查：**
1. 确认 Windows 版本支持该命令
2. 手动测试命令：
   ```powershell
   rundll32 printui.dll,PrintUIEntry /in /n "\\server\printer"
   ```

### 问题 6: 获取打印机列表失败

**症状：**
- `list_printers()` 返回错误
- 无法显示已安装的打印机

**解决方案：**

#### 原因 A: PowerShell 不可用

**测试：**
```powershell
powershell -Command "Get-Printer"
```

如果失败，需要安装或修复 PowerShell。

#### 原因 B: 权限不足

**解决：** 以管理员身份运行应用

#### 原因 C: PowerShell 命令格式问题

**检查：** `src-tauri/src/main.rs` 中的 PowerShell 命令格式是否正确。

可以尝试手动运行命令测试：

```powershell
Get-Printer | Select-Object -ExpandProperty Name | ConvertTo-Json -Compress
```

### 问题 7: 构建后的 exe 文件无法运行

**症状：**
- 构建成功，但运行 exe 时出错
- 提示缺少 DLL 文件

**解决方案：**

1. **检查 Visual C++ Redistributable**：
   - 安装最新版本的 Visual C++ Redistributable
   - 下载地址：https://aka.ms/vs/17/release/vc_redist.x64.exe

2. **使用静态链接**（需要修改配置）：
   在 `Cargo.toml` 中添加：
   ```toml
   [target.'cfg(windows)'.dependencies]
   windows = { version = "0.52", features = ["..."] }
   ```

### 问题 8: VS Code 终端类型问题

**症状：**
- 在 PowerShell 中可以运行，在 Bash/CMD 中不行

**解决方案：**

#### 切换到正确的终端类型

在 VS Code 中：
1. 点击终端下拉菜单（右侧的 `+` 旁边）
2. 选择 "PowerShell" 或 "Command Prompt"
3. 或使用快捷键 `Ctrl+Shift+` 切换

#### 配置默认终端

1. 打开设置（`Ctrl+,`）
2. 搜索 "terminal.integrated.defaultProfile.windows"
3. 设置为 "PowerShell" 或 "Command Prompt"

### 问题 9: 环境变量冲突

**症状：**
- 某些命令可以运行，某些不行
- 不同终端表现不同

**解决方案：**

1. **检查 PATH 中的 Rust 路径**：
   ```powershell
   $env:Path -split ';' | Select-String cargo
   ```

2. **确认路径存在**：
   ```powershell
   Test-Path "C:\Users\<你的用户名>\.cargo\bin"
   ```

3. **清理重复路径**：
   检查系统 PATH 和用户 PATH，移除重复项。

### 问题 10: 构建时间过长

**症状：**
- `npm run tauri dev` 首次运行很慢
- 构建过程耗时很长

**解决方案：**

1. **这是正常的**：
   - 首次构建需要编译所有 Rust 依赖
   - 可能需要 5-15 分钟
   - 后续构建会快很多（只编译更改的部分）

2. **使用镜像源**（中国用户）：
   ```powershell
   # 设置 Rust 镜像
   $env:CARGO_NET_GIT_FETCH_WITH_CLI='true'
   # 创建或编辑 cargo 配置
   notepad $env:USERPROFILE\.cargo\config
   ```
   
   添加内容：
   ```toml
   [source.crates-io]
   replace-with = 'rustcc'

   [source.rustcc]
   registry = "https://mirrors.rustcc.cn/git/crates.io-index.git"
   ```

## 获取更多帮助

如果以上方案都无法解决问题：

1. **查看详细错误信息**：
   ```powershell
   npm run tauri dev -- --verbose
   ```

2. **检查 Rust 工具链**：
   ```powershell
   rustup show
   ```

3. **更新工具链**：
   ```powershell
   rustup update
   ```

4. **查看 Tauri 日志**：
   检查终端输出中的详细错误信息

5. **提交 Issue**：
   如果问题持续存在，可以提供错误信息以便进一步诊断

