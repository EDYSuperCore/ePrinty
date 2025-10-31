# 易点云打印机安装小精灵

基于 **Tauri + Vue 3 + TailwindCSS** 开发的跨平台桌面应用，用于企业内网打印机的安装与管理。

## 项目结构

```
easyPrinter/
├── src/                          # Vue 3 前端代码
│   ├── App.vue                   # 主应用组件
│   ├── main.js                   # Vue 应用入口
│   ├── style.css                 # 全局样式（TailwindCSS）
│   └── components/
│       ├── PrinterArea.vue       # 办公区域组件
│       └── PrinterItem.vue       # 打印机项组件
├── src-tauri/                    # Tauri 后端代码（Rust）
│   ├── src/
│   │   └── main.rs               # Rust 主程序（后端命令实现）
│   ├── Cargo.toml                # Rust 依赖配置
│   ├── build.rs                  # Rust 构建脚本
│   └── tauri.conf.json           # Tauri 应用配置
├── index.html                    # HTML 入口文件
├── package.json                  # Node.js 依赖配置
├── vite.config.js                # Vite 构建配置
├── tailwind.config.js            # TailwindCSS 配置
└── postcss.config.js             # PostCSS 配置
```

## 功能特性

- ✅ **配置加载**：从服务器加载打印机配置（JSON）
- ✅ **打印机枚举**：获取本地已安装的打印机列表
- ✅ **一键安装**：通过 Windows API 安装网络打印机
- ✅ **状态显示**：实时显示打印机安装状态
- ✅ **美观界面**：使用 TailwindCSS 设计的现代化 UI

## 安装依赖

### 前置要求

1. **Node.js** (v16 或更高版本)
   - 下载地址：https://nodejs.org/
   - 安装后验证：`node --version` 和 `npm --version`

2. **Rust** (最新稳定版)
   
   **Windows 安装步骤：**
   
   1. 访问 https://rustup.rs/ 下载 `rustup-init.exe`
   2. 运行安装程序（默认选项即可）
   3. 安装完成后，**重新打开 PowerShell 或终端**
   4. 验证安装：
      ```bash
      rustc --version
      cargo --version
      ```
   
   ⚠️ **重要提示**：
   - 安装完成后必须重新打开终端窗口才能使用
   - 如果仍提示找不到命令，检查环境变量 PATH 是否包含：
     - `C:\Users\<你的用户名>\.cargo\bin`
   - 安装过程会自动安装 Visual Studio Build Tools（如果未安装）

3. **系统依赖**（Windows）
   - Microsoft Visual C++ Build Tools（Rust 安装时会自动下载）
   - Windows SDK（通常已包含在系统中）

### 安装步骤

```bash
# 1. 安装 Node.js 依赖
npm install

# 2. 安装 Rust（如果未安装）
# 访问 https://rustup.rs/ 下载并运行安装程序

# 3. 验证环境
node --version
npm --version
rustc --version
cargo --version
```

## 本地运行

### 开发模式

```bash
# 启动开发服务器（自动打开 Tauri 窗口）
npm run tauri dev
```

开发模式下：
- 前端代码修改后会自动热重载
- Rust 代码修改后需要重新编译（会自动触发）
- 窗口大小：900x700

### 构建生产版本

```bash
# 构建前端和 Tauri 应用
npm run tauri build
```

构建输出：
- Windows: `src-tauri/target/release/easy-printer.exe`
- 可执行文件位于：`src-tauri/target/release/`

## 打包 .exe 命令

```bash
# 构建 Windows 可执行文件（单文件）
npm run tauri build

# 构建结果位置
# src-tauri/target/release/easy-printer.exe
```

### 构建选项

可以在 `src-tauri/tauri.conf.json` 中配置：

- **单文件打包**：默认配置已启用
- **图标**：需要将图标文件放在 `src-tauri/icons/` 目录
- **应用信息**：在 `tauri.conf.json` 的 `package` 部分修改

## 工作原理

### 前后端通信流程

```
┌─────────────┐          ┌──────────────┐          ┌─────────────┐
│   Vue 前端   │  invoke  │  Tauri API   │  command │  Rust 后端   │
│             │ ────────► │              │ ───────► │             │
│  App.vue    │          │  tauri::      │          │  main.rs    │
│  组件       │          │  generate_    │          │  命令函数    │
└─────────────┘          │  handler      │          └─────────────┘
                          └──────────────┘
```

### 核心命令说明

#### 1. `load_config()` - 加载配置

```rust
// Rust 后端
#[tauri::command]
async fn load_config() -> Result<PrinterConfig, String>
```

- **功能**：从服务器获取打印机配置 JSON
- **前端调用**：
  ```javascript
  const config = await invoke('load_config')
  ```
- **返回**：`PrinterConfig` 结构（包含 areas 和 printers）

#### 2. `list_printers()` - 列出打印机

```rust
// Rust 后端
#[tauri::command]
fn list_printers() -> Result<Vec<String>, String>
```

- **功能**：获取本地已安装的打印机名称列表
- **实现**：通过 PowerShell 执行 `Get-Printer` 命令
- **前端调用**：
  ```javascript
  const printers = await invoke('list_printers')
  ```

#### 3. `install_printer(path)` - 安装打印机

```rust
// Rust 后端
#[tauri::command]
async fn install_printer(path: String) -> Result<InstallResult, String>
```

- **功能**：安装指定的网络打印机
- **实现**：调用 Windows API `rundll32 printui.dll,PrintUIEntry /in /n`
- **前端调用**：
  ```javascript
  const result = await invoke('install_printer', { path: '\\\\server\\printer' })
  ```

### 数据流

1. **应用启动**
   - `App.vue` mounted → 调用 `loadData()`
   - 并行加载配置和已安装打印机列表
   - 更新 UI 显示

2. **安装打印机**
   - 用户点击"安装"按钮
   - `PrinterItem.vue` 触发 `install` 事件
   - `App.vue` 调用 `install_printer` 命令
   - 显示安装结果并刷新打印机列表

3. **状态同步**
   - 通过 `installedPrinters` 数组判断打印机是否已安装
   - 使用 `v-if` 条件渲染显示安装状态

## 配置说明

### 服务器配置 URL

默认配置 URL 在 `src-tauri/src/main.rs` 中：

```rust
let url = "https://example.com/printer_config.json";
```

**修改方法**：
1. 直接修改代码中的 URL
2. 或使用环境变量（需要添加相应代码）

### 配置文件格式

服务器返回的 JSON 格式：

```json
{
  "areas": [
    {
      "name": "总部办公区",
      "printers": [
        {
          "name": "前台_HP_LaserJet",
          "path": "\\\\hq-server\\HP_LaserJet"
        }
      ]
    }
  ]
}
```

## 故障排除

### 常见问题

1. **终端中无法运行 rustc，但独立 CMD 可以运行**
   - **原因**：VS Code 集成终端缓存了环境变量
   - **解决**：重启 VS Code 或刷新环境变量
   - **详细方案**：查看 `TROUBLESHOOTING.md` 中的"问题 1"

2. **构建失败：找不到 Rust 工具链**
   - 解决：安装 Rust（访问 https://rustup.rs/）
   - 运行：`rustup update`

3. **网络请求失败**
   - 检查：服务器 URL 是否正确
   - 检查：网络连接是否正常
   - 检查：Tauri 的 HTTP allowlist 配置

4. **安装打印机失败**
   - 检查：是否以管理员权限运行
   - 检查：打印机路径是否正确（格式：`\\\\server\\printer`）
   - 检查：网络打印机是否可访问

5. **获取打印机列表失败**
   - 检查：PowerShell 是否可用
   - 检查：是否有权限执行系统命令

### 开发调试

```bash
# 查看详细构建日志
npm run tauri dev -- --verbose

# 查看 Rust 编译错误
cd src-tauri
cargo build --verbose
```

### 详细故障排除指南

完整的故障排除指南请查看：**[TROUBLESHOOTING.md](./TROUBLESHOOTING.md)**

## 技术栈

- **前端框架**：Vue 3 (Composition API)
- **构建工具**：Vite
- **样式框架**：TailwindCSS
- **桌面框架**：Tauri 1.5
- **后端语言**：Rust
- **HTTP 客户端**：reqwest (Rust)

## 许可证

MIT License

## 作者

Easy Printer Team

