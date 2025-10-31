# Rust 安装指南 (Windows)

## 🔍 问题诊断

如果你看到以下错误：
```
rustc : 无法将"rustc"项识别为 cmdlet、函数、脚本文件或可运行程序的名称
```

这说明 **Rust 未安装** 或 **未正确添加到 PATH 环境变量**。

## 📥 安装步骤

### 方法 1：使用 rustup（推荐）

#### 步骤 1：下载安装程序

1. 访问 https://rustup.rs/
2. 页面会自动检测你的操作系统
3. 点击 "DOWNLOAD rustup-init.exe" 下载安装程序

#### 步骤 2：运行安装程序

1. 双击运行 `rustup-init.exe`
2. 会看到以下提示：
   ```
   1) Proceed with installation (default)
   2) Customize installation
   3) Cancel installation
   ```
3. 直接按 **回车键** 选择默认安装（选项 1）

#### 步骤 3：等待安装完成

安装程序会：
- 下载 Rust 工具链
- 安装 Visual Studio Build Tools（如果未安装，这是必需的）
- 配置环境变量

**注意**：首次安装可能需要 10-30 分钟，取决于网络速度。

#### 步骤 4：重新打开终端

⚠️ **非常重要**：安装完成后，必须：
1. **关闭当前 PowerShell 或终端窗口**
2. **重新打开一个新的终端窗口**

这样环境变量才会生效。

#### 步骤 5：验证安装

在新的终端窗口中运行：

```powershell
rustc --version
```

应该看到类似输出：
```
rustc 1.75.0 (xxx) (default)
```

再运行：

```powershell
cargo --version
```

应该看到类似输出：
```
cargo 1.75.0 (xxx)
```

### 方法 2：使用 Chocolatey（可选）

如果你已经安装了 Chocolatey：

```powershell
choco install rust
```

然后重新打开终端验证。

## 🔧 故障排除

### 问题 1：安装后仍然找不到命令

**解决方案：**

1. **确认 Rust 已安装**：
   - 检查目录 `C:\Users\<你的用户名>\.cargo\bin` 是否存在
   - 如果存在，说明已安装成功

2. **手动添加 PATH**：
   
   a. 打开"系统属性" → "高级" → "环境变量"
   
   b. 在"用户变量"或"系统变量"中找到 `Path`
   
   c. 点击"编辑"，添加以下路径：
      ```
      C:\Users\<你的用户名>\.cargo\bin
      ```
   
   d. 点击"确定"保存
   
   e. **重新打开终端**测试

3. **使用完整路径测试**：
   ```powershell
   C:\Users\<你的用户名>\.cargo\bin\rustc.exe --version
   ```
   
   如果这个命令能运行，说明问题在于 PATH 配置。

### 问题 2：安装过程中提示需要 Visual Studio Build Tools

**解决方案：**

1. **自动安装**：rustup 会自动下载并安装 Visual Studio Build Tools
   - 这需要额外的时间和磁盘空间（约 3-5 GB）
   - 安装过程中不要中断

2. **手动安装**（如果自动安装失败）：
   - 访问 https://visualstudio.microsoft.com/downloads/
   - 下载 "Build Tools for Visual Studio"
   - 安装时选择 "Desktop development with C++" 工作负载
   - 安装完成后重新运行 `rustup-init.exe`

### 问题 3：网络问题导致下载失败

**解决方案：**

1. **使用镜像源**（中国用户）：
   
   在 PowerShell 中设置环境变量：
   ```powershell
   $env:RUSTUP_DIST_SERVER='https://mirrors.rustcc.cn'
   $env:RUSTUP_UPDATE_ROOT='https://mirrors.rustcc.cn/rustup'
   ```
   
   然后重新运行 `rustup-init.exe`

2. **使用代理**：
   - 如果使用代理，确保终端可以访问代理
   - 或在企业网络环境下，联系 IT 管理员

### 问题 4：权限不足

**解决方案：**

- 右键点击 `rustup-init.exe`
- 选择"以管理员身份运行"

## ✅ 验证完整安装

运行以下命令确保所有工具都正常工作：

```powershell
# 检查 Rust 编译器
rustc --version

# 检查 Cargo 包管理器
cargo --version

# 检查 Rust 工具链
rustup show

# 更新到最新版本
rustup update
```

## 🎯 安装完成后

安装成功后，你就可以运行项目了：

```powershell
# 进入项目目录
cd easyPrinter

# 安装 Node.js 依赖
npm install

# 运行开发模式
npm run tauri dev
```

## 📚 更多资源

- **Rust 官方文档**：https://www.rust-lang.org/learn
- **Rust Book（中文）**：https://rustwiki.org/
- **Rustup 文档**：https://rust-lang.github.io/rustup/
- **问题反馈**：https://github.com/rust-lang/rustup/issues

## 💡 提示

- Rust 安装是一次性的，之后会自动更新
- 使用 `rustup update` 可以更新到最新版本
- 使用 `rustup self uninstall` 可以完全卸载 Rust



