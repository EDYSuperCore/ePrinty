# macOS 适配说明

## 已完成的适配

根据你提供的 Objective-C 代码，我已经实现了 macOS 版本的打印机管理功能：

### 1. **打印机列表功能** (`list_printers`)

使用 macOS 的 `lpstat` 命令获取打印机列表：
- 优先使用 `lpstat -p` 获取所有打印机及其状态
- 如果失败，尝试使用 `lpstat -a` 获取可用打印机列表
- 如果仍然没有，尝试获取默认打印机 (`lpstat -d`)

### 2. **安装打印机功能** (`install_printer`)

参考 Objective-C 代码的实现：

#### a. PPD 文件查找和复制

代码会：
1. 从应用 bundle 的 `Resources` 目录查找 PPD 文件（.txt 或 .ppd）
2. 如果找到 `.txt` 文件（如 `ricoh320.txt`），复制到用户目录并重命名为 `.ppd`（如 `~/ricoh320.ppd`）
3. 支持查找多个可能的文件名：`ricoh320`, `generic`, `PostScript`

#### b. 使用 lpadmin 安装打印机

使用命令：
```bash
lpadmin -p <打印机名称> -E -v lpd://<IP地址> -P <PPD文件路径> -D <描述>
```

参数说明：
- `-p`: 打印机名称
- `-E`: 启用打印机
- `-v`: 打印机地址（使用 lpd:// 协议）
- `-P`: PPD 文件路径（可选，如果未指定会使用 `-m everywhere` 让系统自动选择驱动）
- `-D`: 打印机描述

## 与 Objective-C 代码的差异

### 1. **管理员权限处理**

**Objective-C 代码**：
- 使用 `runProcessAsAdministrator` 方法
- 通过 AppleScript `do shell script` 获取管理员权限

**当前 Rust 实现**：
- 直接使用 `Command::new()` 执行命令
- 如果命令需要管理员权限，系统会提示用户输入密码

**如果需要自动获取管理员权限**，可以考虑：
1. 使用 `osascript` 执行需要管理员权限的命令
2. 或者提示用户在"系统偏好设置"中授予权限

### 2. **PPD 文件复制**

**Objective-C 代码**：
```objective-c
cp stemFile ~/ricoh320.ppd
```

**当前 Rust 实现**：
- 使用 `fs::copy()` 复制文件
- 自动处理多个可能的 PPD 文件名
- 如果复制失败，尝试直接使用原文件路径

## 使用方法

### 1. **准备 PPD 文件**

将 PPD 文件（.txt 或 .ppd）放在以下位置之一：

**打包后的 app**：
```
App.app/Contents/Resources/ricoh320.txt
App.app/Contents/Resources/ricoh320.ppd
```

**开发模式**：
```
src-tauri/scripts/ricoh320.txt
src-tauri/scripts/ricoh320.ppd
```

### 2. **配置打印机**

在 `printer_config.json` 中，打印机路径可以使用：
- `\\192.168.x.x` （Windows 格式，会自动转换）
- `lpd://192.168.x.x` （macOS 格式）
- `192.168.x.x` （会添加 lpd:// 前缀）

### 3. **编译 macOS 版本**

在 macOS 系统上：

```bash
npm run tauri build
```

输出：
- `src-tauri/target/release/bundle/macos/easy-printer.app`

## 注意事项

### 1. **管理员权限**

macOS 上安装打印机可能需要管理员权限。如果命令执行失败：
- 系统会自动提示输入管理员密码
- 或者需要在"系统偏好设置" > "安全性与隐私"中授予权限

### 2. **PPD 文件**

- 如果没有提供 PPD 文件，系统会使用 `-m everywhere` 自动选择驱动
- 建议提供特定打印机的 PPD 文件以获得更好的兼容性

### 3. **网络打印机协议**

macOS 通常使用 `lpd://` 协议连接网络打印机。代码会自动处理路径格式转换。

## 如果遇到问题

### 问题 1: 权限被拒绝

**解决**：
- 确保用户有管理员权限
- 在"系统偏好设置"中授予应用权限

### 问题 2: 找不到 PPD 文件

**解决**：
- 检查 PPD 文件是否在正确的目录
- 确认文件名是否正确（ricoh320.txt, generic.ppd 等）
- 查看应用日志了解搜索路径

### 问题 3: 打印机安装失败

**解决**：
- 检查 IP 地址是否正确
- 确认打印机支持 LPD 协议
- 检查网络连接

## 后续改进建议

如果需要完全匹配 Objective-C 代码的行为，可以考虑：

1. **使用 AppleScript 获取管理员权限**：
   ```rust
   let script = format!("do shell script \"lpadmin ...\" with administrator privileges");
   Command::new("osascript").arg("-e").arg(&script)...
   ```

2. **改进错误处理**：
   - 更详细的错误信息
   - 区分权限错误和其他错误

3. **支持更多打印机协议**：
   - `ipp://` (Internet Printing Protocol)
   - `socket://` (Raw socket)

## 测试

在 macOS 上测试：

1. **开发模式**：
   ```bash
   npm run tauri dev
   ```

2. **编译发布版本**：
   ```bash
   npm run tauri build
   ```

3. **测试打印机列表**：
   - 应用应该能够列出已安装的打印机

4. **测试安装打印机**：
   - 选择办公区和打印机
   - 点击安装
   - 检查是否成功安装

