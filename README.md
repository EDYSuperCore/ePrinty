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

- ✅ **智能配置加载**：优先加载本地配置，远程配置作为备用
- ✅ **办公区导航**：左侧导航栏选择办公区，右侧显示对应打印机列表
- ✅ **打印机信息**：显示打印机名称、型号和 IP 地址
- ✅ **一键安装**：支持 Windows 10+ 和 Windows 7/8 两种安装方式
- ✅ **自动检测**：根据 Windows 版本自动选择最佳安装方式
  - Windows 10+：使用 `Add-PrinterPort` + `Add-Printer`（现代方式）
  - Windows 7/8：使用 VBS 脚本方式（兼容方式）
- ✅ **安装状态**：实时显示安装进度和使用的安装方式（VBS/Add-Printer）
- ✅ **打印机枚举**：自动检测已安装的打印机并标记
- ✅ **IT 热线**：右上角帮助按钮，一键打开钉钉 IT 支持
- ✅ **跨平台支持**：支持 Windows 和 macOS 平台
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
- Windows: `src-tauri/target/release/易点云打印机安装小精灵.exe`
- macOS: `src-tauri/target/release/bundle/macos/easy-printer.app`
- 可执行文件位于：`src-tauri/target/release/`
- 配置文件会自动复制到输出目录：`src-tauri/target/release/printer_config.json`

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
async fn load_config() -> Result<LoadConfigResult, String>
```

- **功能**：加载打印机配置（优先本地，远程作为备用）
- **加载顺序**：
  1. 可执行文件所在目录的 `printer_config.json`
  2. 当前工作目录的 `printer_config.json`
  3. 远程服务器配置（如果配置了 URL）
- **前端调用**：
  ```javascript
  const result = await invoke('load_config')
  // result.config: 配置数据
  // result.source: "local" 或 "remote"
  // result.remote_error: 远程加载错误（如果有）
  ```
- **返回**：`LoadConfigResult` 结构（包含配置、来源和错误信息）

#### 2. `list_printers()` - 列出打印机

```rust
// Rust 后端
#[tauri::command]
fn list_printers() -> Result<Vec<String>, String>
```

- **功能**：获取本地已安装的打印机名称列表
- **Windows 实现**：通过 PowerShell 执行 `Get-Printer` 命令
- **macOS 实现**：通过 `lpstat` 命令获取打印机列表
- **前端调用**：
  ```javascript
  const printers = await invoke('list_printers')
  ```

#### 3. `install_printer(name, path)` - 安装打印机

```rust
// Rust 后端
#[tauri::command]
async fn install_printer(name: String, path: String) -> Result<InstallResult, String>
```

- **功能**：安装指定的网络打印机（根据 Windows 版本自动选择安装方式）
- **Windows 10+ 实现**：
  1. 使用 `Add-PrinterPort` 添加打印机端口
  2. 使用 `Add-Printer` 安装打印机
  3. 自动查找合适的打印机驱动
- **Windows 7/8 实现**：
  1. 使用 VBS 脚本添加打印机端口
  2. 使用 `Add-Printer` 安装打印机
- **macOS 实现**：使用 `lpadmin` 命令安装打印机
- **前端调用**：
  ```javascript
  const result = await invoke('install_printer', { 
    name: '打印机名称',
    path: '\\\\192.168.1.100' 
  })
  // result.success: 是否成功
  // result.message: 安装结果消息
  // result.method: "VBS" 或 "Add-Printer" 或 "macOS"
  ```

### UI 使用流程

1. **应用启动**
   - 应用加载后自动加载配置文件
   - 左侧显示办公区列表（自动选择第一个）
   - 右侧显示选中办公区的打印机列表
   - 已安装的打印机会显示绿色"已安装"标签

2. **选择办公区**
   - 点击左侧办公区名称切换办公区
   - 右侧自动更新显示该办公区的打印机
   - 每个打印机显示：名称、型号（蓝色）、IP 地址

3. **安装打印机**
   - 点击未安装打印机的"安装"按钮
   - 状态栏显示安装进度："正在安装 打印机名称..."
   - 安装完成后显示结果和安装方式：[方式: VBS] 或 [方式: Add-Printer]
   - 自动刷新打印机列表，已安装的打印机会显示"已安装"标签

4. **IT 热线支持**
   - 点击右上角"IT热线"按钮（钉钉图标）
   - 自动打开钉钉聊天对话框
   - 如果钉钉未安装或无法打开，会显示提示信息

### 数据流

1. **应用启动**
   - `App.vue` mounted → 调用 `loadData()`
   - 并行加载配置和已安装打印机列表
   - 自动选择第一个办公区并显示打印机

2. **安装打印机**
   - 用户点击"安装"按钮
   - `PrinterItem.vue` 触发 `install` 事件（传递打印机对象）
   - `App.vue` 调用 `install_printer(name, path)` 命令
   - 状态栏显示安装方式和结果
   - 安装成功后自动刷新打印机列表

3. **状态同步**
   - 通过 `installedPrinters` 数组判断打印机是否已安装
   - 使用 `v-if` 条件渲染显示安装状态
   - 状态栏实时显示当前操作和安装方式

## 配置说明

### 配置加载策略

应用采用**优先本地配置**的策略：

1. **优先加载本地配置**
   - 从可执行文件所在目录加载 `printer_config.json`
   - 如果不存在，从当前工作目录加载

2. **远程配置作为备用**
   - 如果本地配置不存在，尝试从远程服务器加载
   - 远程配置 URL 在 `src-tauri/src/main.rs` 中配置
   - 如果远程加载失败但本地存在配置，仍可使用本地配置（仅提示警告）

3. **配置文件位置**
   - **开发模式**：项目根目录 `printer_config.json`
   - **生产模式**：与可执行文件同目录 `printer_config.json`
   - 构建时会自动复制配置文件到输出目录

### 配置文件格式

配置文件 `printer_config.json` 格式：

```json
{
  "areas": [
    {
      "name": "北京易点云大厦A座一楼",
      "printers": [
        {
          "name": "大厦A座一楼 彩色打印机",
          "path": "\\\\192.168.20.65",
          "model": "RICOH SP 325SNw PCL 6"
        },
        {
          "name": "大厦A座一楼 前台",
          "path": "\\\\192.168.20.11",
          "model": "RICOH SP 325SNw PCL 6"
        }
      ]
    },
    {
      "name": "北京易点云大厦A座二楼",
      "printers": [
        {
          "name": "大厦A座二楼 打印机",
          "path": "\\\\192.168.63.7",
          "model": "RICOH SP 325SNw PCL 6"
        }
      ]
    }
  ]
}
```

**字段说明**：
- `areas`: 办公区数组
  - `name`: 办公区名称
  - `printers`: 该办公区的打印机列表
    - `name`: 打印机名称
    - `path`: 打印机网络路径（Windows 格式：`\\IP地址`）
    - `model`: 打印机型号（可选，会在 UI 中显示）

**配置文件位置**：
- 开发模式：项目根目录 `printer_config.json`
- 生产模式：可执行文件所在目录 `printer_config.json`
- 自动复制：构建时 `build.rs` 会自动复制配置文件到输出目录

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
   - 检查：打印机路径是否正确（Windows 格式：`\\192.168.1.100`）
   - 检查：网络打印机是否可访问
   - 检查：系统中是否已安装打印机驱动
   - 查看状态栏显示的安装方式和错误信息

5. **获取打印机列表失败**
   - Windows：检查 PowerShell 是否可用
   - macOS：检查 `lpstat` 命令是否可用
   - 检查：是否有权限执行系统命令

6. **打包后显示命令行窗口**
   - 确保使用 Release 模式编译：`npm run tauri build`
   - 已为所有命令添加窗口隐藏标志，正常情况下不应显示窗口
   - 如果仍有窗口，检查是否是启动时的短暂闪现（这是正常的）

7. **端口已存在错误**
   - 这是正常情况，应用会自动跳过端口创建步骤
   - 如果端口确实不存在但报错，可能需要管理员权限

### 开发调试

```bash
# 查看详细构建日志
npm run tauri dev -- --verbose

# 查看 Rust 编译错误
cd src-tauri
cargo build --verbose
```

## 部署说明

### Windows 平台部署

打包后需要分发的文件：

1. **主可执行文件**（必需）
   ```
   易点云打印机安装小精灵.exe
   位置：src-tauri/target/release/易点云打印机安装小精灵.exe
   ```

2. **配置文件**（必需）
   ```
   printer_config.json
   位置：src-tauri/target/release/printer_config.json
   说明：与 exe 同目录，可手动修改
   ```

**已嵌入到 exe 中的资源**：
- ✅ `prnport.vbs` - VBS 脚本已嵌入，无需单独分发
- ✅ 所有依赖库已静态链接

详细部署指南请查看：**[DEPLOYMENT.md](./DEPLOYMENT.md)**

## 技术栈

### 前端
- **框架**：Vue 3 (Composition API)
- **构建工具**：Vite
- **样式框架**：TailwindCSS
- **UI 设计**：现代化响应式布局，左侧导航 + 右侧内容区域

### 后端
- **框架**：Tauri 1.5
- **语言**：Rust
- **HTTP 客户端**：reqwest (Rust)
- **编码处理**：encoding_rs (用于 Windows GBK 编码转换)
- **Windows API**：winapi (用于版本检测和窗口控制)

### 平台特性

#### Windows
- **安装方式**：
  - Windows 10+：使用 PowerShell `Add-PrinterPort` + `Add-Printer`
  - Windows 7/8：使用 VBS 脚本 `prnport.vbs` + `Add-Printer`
- **版本检测**：通过 PowerShell `Get-CimInstance` 获取真实构建号
- **窗口隐藏**：所有命令使用 `CREATE_NO_WINDOW` 标志

#### macOS
- **安装方式**：使用 `lpadmin` 命令安装打印机
- **PPD 文件**：支持从资源目录加载 PPD 文件
- **打印机列表**：使用 `lpstat` 命令获取

## 安装方式说明

应用会根据 Windows 版本自动选择最合适的安装方式：

- **Windows 10+（构建号 >= 10240）**：
  ```
  Add-PrinterPort -Name "IP_192_168_20_65" -PrinterHostAddress "192.168.20.65"
  Add-Printer -Name "打印机名称" -DriverName "驱动名称" -PortName "IP_192_168_20_65"
  ```
  - 优点：更现代、更可靠、支持端口验证
  - 显示：[方式: Add-Printer]

- **Windows 7/8（构建号 < 10240）**：
  ```
  cscript prnport.vbs -a -r IP_192.168.20.65 -h 192.168.20.65 -o raw
  Add-Printer -Name "打印机名称" -DriverName "驱动名称" -PortName "IP_192_168_20_65"
  ```
  - 优点：兼容旧版本 Windows
  - 显示：[方式: VBS]

所有安装方式都会在状态栏显示当前使用的安装方法，便于调试和排查问题。

## 许可证

MIT License

## 作者

Easy Printer Team

