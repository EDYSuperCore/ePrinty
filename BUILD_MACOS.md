# 编译 macOS App 指南

## 概述

理论上可以使用 Tauri 将应用编译为 macOS app，但**当前代码主要是为 Windows 平台设计的**，需要做一些适配。

## 当前代码的兼容性问题

### 1. **打印机管理功能**（Windows 特定）

当前代码使用了 Windows 特定的功能：

- **`list_printers()`**：使用 PowerShell 命令获取打印机列表
- **`install_printer()`**：使用 Windows 的 `prnport.vbs`、`rundll32` 等
- **编码处理**：使用 GBK 编码（Windows 中文环境）

这些功能在 macOS 上需要**完全重写**。

### 2. **系统命令**

代码中使用了：
- PowerShell 命令（Windows 特有）
- Windows API 调用
- Windows 特定的脚本文件（`prnport.vbs`）

## 编译 macOS App 的条件

### 必需环境

1. **macOS 系统**
   - 必须在 macOS 系统上编译
   - 无法在 Windows 上交叉编译 macOS app

2. **Xcode**
   - 需要安装 Xcode（从 App Store）
   - 需要安装 Xcode Command Line Tools：
     ```bash
     xcode-select --install
     ```

3. **Rust**
   - 需要在 macOS 上安装 Rust
   ```bash
     curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
     ```

4. **Node.js**
   - 需要安装 Node.js 和 npm

## 如何编译（如果适配完成）

如果已经适配了 macOS 代码，可以这样编译：

```bash
# 在 macOS 系统上执行
npm run tauri build

# 或者指定平台
npm run tauri build -- --target universal-apple-darwin
```

编译输出：
- `src-tauri/target/release/bundle/macos/easy-printer.app`
- 或 `.dmg` 文件（如果配置了）

## 适配 macOS 需要做的工作

### 1. **打印机列表功能**

需要在 `src-tauri/src/main.rs` 中添加 macOS 实现：

```rust
#[tauri::command]
fn list_printers() -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        
        // 使用 macOS 的 lpstat 命令
        let output = Command::new("lpstat")
            .arg("-p")
            .output()
            .map_err(|e| format!("执行 lpstat 命令失败: {}", e))?;
        
        if !output.status.success() {
            return Err("获取打印机列表失败".to_string());
        }
        
        let stdout = String::from_utf8_lossy(&output.stdout);
        // 解析 lpstat 输出...
        // ...
    }
    #[cfg(windows)]
    {
        // 现有的 Windows 代码
        // ...
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        Err("当前平台不支持".to_string())
    }
}
```

### 2. **安装打印机功能**

macOS 上安装打印机需要使用不同的方法：

- 使用 `lpadmin` 命令
- 或使用 macOS 的 CUPS API
- 或使用 `system_profiler` 获取打印机信息

### 3. **移除 Windows 特定资源**

在 `tauri.conf.json` 中：

```json
"resources": [
  // "scripts/prnport.vbs"  // macOS 不需要这个
]
```

### 4. **图标文件**

确保有 macOS 图标：
- `icons/icon.icns`（必需）
- 其他 PNG 文件（可选）

## 推荐的适配方案

### 方案 1：完整适配（适合多平台使用）

1. 重写所有打印机管理功能，添加 macOS 实现
2. 使用条件编译区分平台
3. 测试在两个平台上都能正常工作

### 方案 2：仅 Windows 版本（当前）

- 保持当前的 Windows 特定实现
- 在 macOS 上编译时，显示"仅支持 Windows"的错误提示
- 适合仅在 Windows 环境使用的场景

### 方案 3：使用抽象层

- 创建一个打印机管理的抽象层
- 为每个平台实现具体的实现
- 更易于维护和扩展

## 当前代码的 macOS 兼容性

### ✅ 兼容的部分

- 前端代码（Vue 3）完全兼容
- Tauri 框架本身跨平台
- 配置文件加载功能（可以复用）
- 远程配置加载功能（可以复用）

### ❌ 需要重写的部分

- `list_printers()` 函数（Windows 特定）
- `install_printer()` 函数（Windows 特定）
- 编码处理（GBK → UTF-8，macOS 主要使用 UTF-8）
- Windows 特定的脚本文件

## 快速测试（不完整版本）

如果想先测试能否编译（功能不完整）：

1. 在 macOS 上安装依赖
2. 修改 `list_printers()` 和 `install_printer()`，添加 macOS 存根实现
3. 尝试编译：
   ```bash
   npm run tauri build
   ```

但这样编译出来的 app 在 macOS 上无法使用打印机管理功能。

## 总结

- ✅ **可以编译**：Tauri 支持 macOS
- ❌ **需要适配**：打印机管理功能需要重写
- ⚠️ **需要环境**：必须在 macOS 系统上编译
- 💡 **建议**：如果只需要 Windows 版本，保持当前实现即可

## 如果确实需要 macOS 版本

1. 在 macOS 系统上安装开发环境
2. 重写打印机管理功能的 macOS 实现
3. 测试并修复兼容性问题
4. 编译和分发

需要我帮你编写 macOS 版本的打印机管理代码吗？

