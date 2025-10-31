# 安装和使用指南

## 快速开始

### 1. 环境准备

#### Windows 系统

1. **安装 Node.js**
   - 访问 https://nodejs.org/ 下载并安装 LTS 版本
   - 验证安装：
     ```bash
     node --version
     npm --version
     ```

2. **安装 Rust**
   - 访问 https://rustup.rs/ 下载 `rustup-init.exe`
   - 运行安装程序（选择默认选项）
   - ⚠️ **重要**：安装完成后必须重新打开 PowerShell/终端
   - 验证安装：
     ```bash
     rustc --version
     cargo --version
     ```
   - 如果遇到问题，请查看 `RUST_INSTALL.md` 详细指南

3. **安装 Visual Studio Build Tools**（如果未安装）
   - 访问 https://visualstudio.microsoft.com/downloads/
   - 下载 "Build Tools for Visual Studio"
   - 安装时选择 "Desktop development with C++" 工作负载

### 2. 安装项目依赖

```bash
# 在项目根目录执行
npm install
```

这将安装：
- Vue 3 及其依赖
- Tauri CLI 工具
- Vite 构建工具
- TailwindCSS 及相关工具

### 3. 运行项目

#### 开发模式

```bash
npm run tauri dev
```

首次运行会自动：
1. 下载 Rust 依赖（可能需要几分钟）
2. 编译 Rust 后端代码
3. 启动 Vite 开发服务器
4. 打开 Tauri 应用窗口

#### 生产构建

```bash
npm run tauri build
```

构建输出：
- **Windows**: `src-tauri/target/release/easy-printer.exe`
- **文件大小**: 约 10-20 MB（单文件）

## 配置说明

### 修改服务器 URL

默认配置服务器地址为 `https://example.com/printer_config.json`

修改方法：编辑 `src-tauri/src/main.rs` 第 34 行：

```rust
let url = "https://example.com/printer_config.json"; // 改为你的服务器地址
```

### 添加应用图标

1. 准备图标文件：
   - `32x32.png`
   - `128x128.png`
   - `128x128@2x.png` (256x256)
   - `icon.icns` (macOS)
   - `icon.ico` (Windows)

2. 将图标文件放入 `src-tauri/icons/` 目录

3. 重新构建应用

### 测试配置 JSON

创建测试配置文件 `test_config.json`：

```json
{
  "areas": [
    {
      "name": "测试区域",
      "printers": [
        {
          "name": "测试打印机",
          "path": "\\\\test-server\\test-printer"
        }
      ]
    }
  ]
}
```

可以使用本地服务器测试：
1. 使用 Python 启动本地服务器：
   ```bash
   python -m http.server 8000
   ```
2. 将 `test_config.json` 放在服务器目录
3. 修改代码中的 URL 为 `http://localhost:8000/test_config.json`

## 常见问题

### Q: 构建时出现 "找不到 Rust 工具链"
**A**: 运行 `rustup update` 更新 Rust 工具链

### Q: 网络请求失败
**A**: 
1. 检查服务器 URL 是否正确
2. 检查防火墙设置
3. 检查 Tauri 的 HTTP allowlist 配置（`tauri.conf.json`）

### Q: 安装打印机需要管理员权限
**A**: 
1. 以管理员身份运行应用
2. 或使用任务计划程序配置自动提升权限

### Q: 无法获取打印机列表
**A**: 
1. 检查 PowerShell 是否可用
2. 检查系统权限
3. 尝试手动运行 PowerShell 命令测试

## 开发提示

### 查看日志

开发模式下，可以在浏览器控制台（如果启用了调试）或终端查看日志。

### 调试 Rust 代码

1. 安装 Rust 调试工具
2. 在 VS Code 中安装 "rust-analyzer" 扩展
3. 设置断点进行调试

### 前端热重载

Vue 组件修改后会自动刷新，无需重启应用。

### 后端代码修改

Rust 代码修改后需要重新编译，Tauri 开发模式会自动检测并重新编译。

## 打包分发

### 单文件打包

默认配置已经启用了单文件打包，构建出的 `.exe` 文件可以独立运行。

### 安装程序打包

可以在 `tauri.conf.json` 中配置创建安装程序：

```json
{
  "tauri": {
    "bundle": {
      "targets": ["msi", "nsis"]
    }
  }
}
```

需要安装额外的工具：
- **MSI**: Windows SDK
- **NSIS**: 需要安装 NSIS 工具

## 性能优化

### 减小文件大小

1. 启用代码压缩（已在构建配置中）
2. 移除未使用的依赖
3. 使用 `cargo build --release` 优化 Rust 代码

### 启动速度优化

1. 延迟加载非关键资源
2. 使用代码分割
3. 优化首次渲染时间

