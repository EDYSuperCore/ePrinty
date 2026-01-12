# ePrinty - 让打印这件事，简单一点

<div align="center">

![Version](https://img.shields.io/badge/version-1.4.1-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-lightgrey.svg)

基于 **Tauri + Vue 3 + TailwindCSS** 开发的跨平台桌面应用，用于企业内网打印机的安装与管理。

[功能特性](#功能特性) • [快速开始](#快速开始) • [使用指南](#使用指南) • [开发指南](#开发指南) • [故障排除](#故障排除)

</div>

---

## 📋 目录

- [功能特性](#功能特性)
- [快速开始](#快速开始)
- [使用指南](#使用指南)
- [配置说明](#配置说明)
- [开发指南](#开发指南)
- [技术栈](#技术栈)
- [故障排除](#故障排除)
- [部署说明](#部署说明)
- [许可证](#许可证)

## ✨ 功能特性

- ✅ **智能配置加载**：优先加载本地配置，远程配置作为备用
- ✅ **办公区导航**：左侧导航栏选择办公区，右侧显示对应打印机列表
- ✅ **打印机信息**：显示打印机名称、型号和 IP 地址
- ✅ **一键安装**：支持 Windows 10+ 和 Windows 7/8 两种安装方式
- ✅ **自动检测**：根据 Windows 版本自动选择最佳安装方式
  - Windows 10+：使用 `Add-PrinterPort` + `Add-Printer`（现代方式）
  - Windows 7/8：使用 VBS 脚本方式（兼容方式）
- ✅ **安装状态**：实时显示安装进度和使用的安装方式（VBS/Add-Printer）
- ✅ **打印机枚举**：自动检测已安装的打印机并标记（支持 Windows 11）
  - 使用 `EnumPrintersW` API 直接枚举，避免 PowerShell 冷启动延迟
  - 支持本地、网络和共享打印机检测
  - 详细的诊断日志记录
- ✅ **IT 热线**：右上角帮助按钮，一键打开钉钉 IT 支持
- ✅ **跨平台支持**：支持 Windows 和 macOS 平台
- ✅ **美观界面**：使用 TailwindCSS 设计的现代化 UI

## 🚀 快速开始

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
# 1. 克隆项目（如果从 Git 仓库获取）
git clone <repository-url>
cd easyPrinter

# 2. 安装 Node.js 依赖
npm install

# 3. 验证环境
node --version
npm --version
rustc --version
cargo --version
```

## 📖 使用指南

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
- **Windows**: `src-tauri/target/release/ePrinty.exe`
- **macOS**: `src-tauri/target/release/bundle/macos/easy-printer.app`
- 可执行文件位于：`src-tauri/target/release/`
- 配置文件会自动复制到输出目录：`src-tauri/target/release/printer_config.json`

### 应用使用流程

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

4. **刷新列表**
   - 点击右上角"刷新"按钮
   - 重新加载配置并检测已安装的打印机状态

5. **IT 热线支持**
   - 点击右上角"IT热线"按钮（钉钉图标）
   - 自动打开钉钉聊天对话框
   - 如果钉钉未安装或无法打开，会显示提示信息

## ⚙️ 配置说明

### 版本信息

从 **v2.0.0** 开始，应用采用新的驱动管理方案：
- **driverCatalog**：集中管理所有驱动信息（避免重复定义）
- **printers**：仅通过 `driverKey` 引用驱动，不再维护驱动细节字段
- **校验机制**：严格校验配置完整性，缺失字段会明确报错

⚠️ **重要**：如使用 v1.x 旧配置，应用将直接报错并阻止启动，需升级为 v2.0.0 格式。

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

### 配置文件格式（v2.0.0+）

#### 📌 完整示例

```json
{
  "version": "2.0.0",
  "driverCatalog": {
    "HP_M227_WIN_X64": {
      "displayName": "HP LaserJet MFP M227-M231 PCL-6 (V4)",
      "vendor": "HP",
      "installMode": "package",
      "local": {
        "infRel": "drivers/HP 227/hpsw642a4_x64.inf",
        "driverNames": ["HP LaserJet MFP M227-M231 PCL-6 (V4)"]
      },
      "notes": "HP M227 series - used by 3 printers in Beijing office"
    },
    "EPSON_WFC5790_WIN_X64": {
      "displayName": "EPSON WF-C5790",
      "vendor": "EPSON",
      "installMode": "package",
      "local": {
        "infRel": "WFC5790_x64_2120W/WINX64/E_WF1SGE.INF",
        "driverNames": ["EPSON WF-C5790 Series"]
      },
      "remote": {
        "url": "http://192.168.2.200:8080/WFC5790_x64_2120W.zip",
        "sha256": "32C9A770396EC36F71DDBEAF2EAB6514C8BD72B82C44E23A72E0B0E81DA4ED1E",
        "version": "2026.01.10",
        "layout": "drivers_root"
      },
      "notes": "EPSON WF-C5790 with remote package support"
    }
  },
  "cities": [
    {
      "cityId": "beijing",
      "cityName": "北京",
      "areas": [
        {
          "areaId": "bj-a-1f",
          "areaName": "易点云大厦A座一楼",
          "name": "北京易点云大厦A座一楼",
          "printers": [
            {
              "name": "大厦A座一楼 前台",
              "path": "\\\\192.168.20.11",
              "model": "HP LaserJet MFP M227fdw",
              "driverKey": "HP_M227_WIN_X64"
            },
            {
              "name": "大厦A座一楼 SSC打印机",
              "path": "\\\\192.168.14.32",
              "model": "HP LaserJet MFP M227fdw",
              "driverKey": "HP_M227_WIN_X64"
            },
            {
              "name": "大厦B座一楼 打印机",
              "path": "\\\\192.168.20.5",
              "model": "EPSON WF-C5790",
              "driverKey": "EPSON_WFC5790_WIN_X64"
            }
          ]
        }
      ]
    },
    {
      "cityId": "shanghai",
      "cityName": "上海",
      "areas": [...]
    }
  ]
}
```

#### 📋 driverCatalog 详细说明

**driverCatalog** 是一个 JSON 对象，以 `driverKey` 为主键，每个驱动条目包含：

```
driverCatalog: {
  "<driverKey>": {
    "displayName": "驱动显示名称（可选）",
    "vendor": "厂商名称（可选）",
    "installMode": "package" | "inf",  // 优先级高于 printer 配置
    
    // 本地驱动规格（INF 文件相对路径）
    "local": {
      "infRel": "drivers/HP 227/hpsw642a4_x64.inf",  // 相对于应用目录
      "driverNames": ["HP LaserJet MFP M227-M231 PCL-6 (V4)"]  // 用于校验安装
    },
    
    // 远程驱动规格（ZIP 包下载）- 可选
    "remote": {
      "url": "http://192.168.2.200:8080/WFC5790_x64_2120W.zip",
      "sha256": "32C9A770396EC36F71DDBEAF2EAB6514C8BD72B82C44E23A72E0B0E81DA4ED1E",
      "version": "2026.01.10",
      "layout": "drivers_root"  // ZIP 内驱动布局
    },
    
    "notes": "可选备注（用于文档和维护）"
  }
}
```

**关键字段说明**：
- `driverKey` ⭐：驱动的唯一标识（驱动主键）
  - 命名建议：`<厂商>_<型号>_<平台>_<架构>` (如 `HP_M227_WIN_X64`)
  - 字母、数字、下划线和连字符，8-64 字符
- `displayName`：用于 UI 显示的驱动名称（可选）
- `vendor`：厂商名称（可选，用于分类和搜索）
- `installMode`：安装方式
  - `package`：使用 ZIP 包（调用 driver_bootstrap）
  - `inf`：直接使用本地 INF 文件
- `local.infRel`：INF 文件相对于应用根目录的路径
- `local.driverNames`：驱动名称列表（用于 Windows 校验已安装驱动）
- `remote`：远程 ZIP 包配置（可选）
  - `url`：下载地址
  - `sha256`：完整性校验值
  - `version`：驱动版本号
  - `layout`：ZIP 内布局说明（如 `drivers_root`）

#### 📌 printers 节点详细说明

v2.0.0+ **printers** 节点仅包含设备本身的属性，所有驱动信息通过 `driverKey` 从 driverCatalog 获取：

```json
"printers": [
  {
    "name": "打印机显示名称 ⭐ 必需",
    "path": "\\\\192.168.20.11 ⭐ 必需（网络路径）",
    "model": "HP LaserJet MFP M227fdw（可选）",
    "driverKey": "HP_M227_WIN_X64 ⭐ 必需（driverCatalog 的引用键）"
  }
]
```

**关键约束**：
- ✅ **允许的字段**：`name`, `path`, `model`, `driverKey`, `areaId`, `areaName`
- ❌ **不允许的字段**：`driver_path`, `driver_names`, `install_mode`, `drivers`, `inf_path`, `driver_url`, `sha256`
  - 这些字段已全部迁移至 driverCatalog，如出现会触发校验错误

#### 🎯 配置校验规则

应用在启动时执行强校验：

| 规则 | 错误提示 | 处理方式 |
|------|--------|--------|
| driverCatalog 必须存在 | 缺少 driverCatalog | **阻止启动** |
| driverCatalog 非空 | driverCatalog 为空 | **阻止启动** |
| printer.driverKey 必须存在 | 打印机缺少 driverKey | **阻止启动** |
| driverKey 必须在 catalog 中 | driverKey 不存在 | **阻止启动** + 提示有效 key |
| 不允许残留旧字段 | driver_path/driver_names 等 | **阻止启动** + 提示需清理 |

**错误示例**：
```
【配置校验失败】打印机 '大厦A座一楼 前台' (路径: \\192.168.20.11) 缺少 driverKey。
请在 printer_config.json 中补齐此字段

【配置校验失败】打印机 '大厦A座一楼 前台' 引用的 driverKey='HP_M227' 在 driverCatalog 中不存在。
已定义的 driverKey：['HP_M227_WIN_X64', 'HP_M154_WIN_X64', 'EPSON_WFC5790_WIN_X64']

【配置校验失败】打印机 '大厦A座一楼 前台' (路径: \\192.168.20.11) 仍包含已废弃的字段: driver_path, driver_names。
这些字段已迁移至 driverCatalog，请从 printer 节点删除
```

## 🛠️ 开发指南

### 项目结构


```
easyPrinter/
├── src/                          # Vue 3 前端代码
│   ├── App.vue                   # 主应用组件
│   ├── main.js                   # Vue 应用入口
│   ├── style.css                 # 全局样式（TailwindCSS）
│   └── components/
│       ├── AppTitleBar.vue       # 标题栏组件
│       ├── PrinterArea.vue       # 办公区域组件
│       └── PrinterItem.vue       # 打印机项组件
├── src-tauri/                    # Tauri 后端代码（Rust）
│   ├── src/
│   │   ├── main.rs               # Rust 主程序（后端命令实现）
│   │   ├── exec.rs               # 命令执行封装
│   │   └── platform/
│   │       ├── mod.rs            # 平台统一接口
│   │       ├── windows/          # Windows 平台实现
│   │       │   ├── enum_printers.rs  # 打印机枚举（EnumPrintersW）
│   │       │   ├── list.rs       # 打印机列表获取
│   │       │   ├── install.rs    # 打印机安装
│   │       │   ├── log.rs        # 日志记录
│   │       │   └── ...
│   │       └── macos.rs          # macOS 平台实现
│   ├── Cargo.toml                # Rust 依赖配置
│   ├── build.rs                   # Rust 构建脚本
│   └── tauri.conf.json           # Tauri 应用配置
├── index.html                    # HTML 入口文件
├── package.json                  # Node.js 依赖配置
├── vite.config.js                # Vite 构建配置
├── tailwind.config.js            # TailwindCSS 配置
└── postcss.config.js             # PostCSS 配置
```

### 核心命令说明

#### 1. `load_config()` - 加载配置

```rust
{
  "version": "1.3.0",
  "cities": [
    {
      "cityId": "beijing",
      "cityName": "北京",
      "areas": [
        {
          "areaId": "bj-a-1f",
          "areaName": "易点云大厦A座一楼",
          "name": "北京易点云大厦A座一楼",
          "printers": [
            {
              "name": "大厦A座一楼 前台",
              "path": "\\\\192.168.20.11",
              "model": "HP LaserJet MFP M227fdw",
              "driver_names": ["HP LaserJet MFP M227-M231 PCL-6 (V4)"],
              "driver_path": "drivers/HP 227/hpsw642a4_x64.inf",
              "install_mode": "package"
            }
          ]
        },
        {
          "areaId": "bj-a-2f",
          "areaName": "易点云大厦A座二楼",
          "name": "北京易点云大厦A座二楼",
          "printers": [...]
        }
      ]
    },
    {
      "cityId": "shanghai",
      "cityName": "上海",
      "areas": [...]
    }
  ]
}
```

**新版字段说明**：
- `cities`: 城市数组
  - `cityId`: 城市唯一标识（必需）
  - `cityName`: 城市显示名称（必需）
  - `areas`: 该城市下的办公区数组
    - `areaId`: 办公区唯一标识（必需）
    - `areaName`: 办公区显示名称（必需，用于UI显示）
    - `name`: 完整名称（可选，用于向后兼容）
    - `printers`: 打印机列表

**UI 效果**：
- 左侧导航显示为二级树形结构
- 一级菜单：城市（可展开/折叠）
- 二级菜单：该城市下的办公区
- 点击办公区后，右侧显示对应的打印机列表

## 🛠️ 开发指南

### 项目结构

```
easyPrinter/
├── src/                          # Vue 3 前端代码
│   ├── App.vue                   # 主应用组件
│   ├── main.js                   # Vue 应用入口
│   ├── style.css                 # 全局样式（TailwindCSS）
│   └── components/
│       ├── AppTitleBar.vue       # 标题栏组件
│       ├── PrinterArea.vue       # 办公区域组件
│       └── PrinterItem.vue       # 打印机项组件
├── src-tauri/                    # Tauri 后端代码（Rust）
│   ├── src/
│   │   ├── main.rs               # Rust 主程序（后端命令实现）
│   │   ├── exec.rs               # 命令执行封装
│   │   └── platform/
│   │       ├── mod.rs            # 平台统一接口
│   │       ├── windows/          # Windows 平台实现
│   │       │   ├── enum_printers.rs  # 打印机枚举（EnumPrintersW）
│   │       │   ├── list.rs       # 打印机列表获取
│   │       │   ├── install.rs    # 打印机安装
│   │       │   ├── log.rs        # 日志记录
│   │       │   └── ...
│   │       └── macos.rs          # macOS 平台实现
│   ├── Cargo.toml                # Rust 依赖配置
│   ├── build.rs                   # Rust 构建脚本
│   └── tauri.conf.json           # Tauri 应用配置
├── index.html                    # HTML 入口文件
├── package.json                  # Node.js 依赖配置
├── vite.config.js                # Vite 构建配置
├── tailwind.config.js            # TailwindCSS 配置
└── postcss.config.js             # PostCSS 配置
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

#### 2. `list_printers()` - 列出打印机

```rust
// Rust 后端
#[tauri::command]
fn list_printers() -> Result<Vec<String>, String>
```

- **功能**：获取本地已安装的打印机名称列表
- **Windows 实现**：
  - 使用 `EnumPrintersW` API 直接枚举（Unicode 版本）
  - 支持 `PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS | PRINTER_ENUM_NETWORK`
  - 使用 `PRINTER_INFO_4W` (Level 4) 获取打印机信息
  - 详细的诊断日志记录到 `%LOCALAPPDATA%\ePrinty\logs\printer-detect.log`
- **macOS 实现**：通过 `lpstat` 命令获取打印机列表
- **前端调用**：
  ```javascript
  const printers = await invoke('list_printers')
  ```

#### 3. `install_printer()` - 安装打印机

```rust
// Rust 后端
#[tauri::command]
async fn install_printer(
    name: String,
    path: String,
    driverPath: Option<String>,
    model: Option<String>,
    driverInstallPolicy: Option<String>
) -> Result<InstallResult, String>
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
    path: '\\\\192.168.1.100',
    driverPath: 'C:\\path\\to\\driver.inf',  // 可选
    model: '打印机型号',  // 可选
    driverInstallPolicy: 'always'  // 可选
  })
  // result.success: 是否成功
  // result.message: 安装结果消息
  // result.method: "VBS" 或 "Add-Printer" 或 "macOS"
  ```

### 调试开发

```bash
# 查看详细构建日志
npm run tauri dev -- --verbose

# 查看 Rust 编译错误
cd src-tauri
cargo build --verbose

# 查看 Windows 打印机枚举日志
# 日志文件位置：%LOCALAPPDATA%\ePrinty\logs\printer-detect.log
```

## 🏗️ 技术栈

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
- **Windows API**：
  - `windows` crate 0.52 (用于 Win32 API 调用)
  - `winapi` (用于版本检测和窗口控制)

### 平台特性

#### Windows
- **打印机枚举**：
  - 使用 `EnumPrintersW` API（Unicode 版本）
  - 支持本地、网络和共享打印机
  - 详细的诊断日志记录
- **安装方式**：
  - Windows 10+：使用 PowerShell `Add-PrinterPort` + `Add-Printer`
  - Windows 7/8：使用 VBS 脚本 `prnport.vbs` + `Add-Printer`
- **版本检测**：通过 PowerShell `Get-CimInstance` 获取真实构建号
- **窗口隐藏**：所有命令使用 `CREATE_NO_WINDOW` 标志

#### macOS
- **安装方式**：使用 `lpadmin` 命令安装打印机
- **PPD 文件**：支持从资源目录加载 PPD 文件
- **打印机列表**：使用 `lpstat` 命令获取

## 🔧 故障排除

### 常见问题

1. **终端中无法运行 rustc，但独立 CMD 可以运行**
   - **原因**：VS Code 集成终端缓存了环境变量
   - **解决**：重启 VS Code 或刷新环境变量

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

5. **获取打印机列表失败或数量过少（Windows 11）**
   - **问题**：`EnumPrintersW` 枚举结果过少
   - **解决**：已修复，使用 Unicode 版本的 `EnumPrintersW` API
   - **诊断**：查看日志文件 `%LOCALAPPDATA%\ePrinty\logs\printer-detect.log`
   - **日志内容**：包含 flags、Level、调用结果、错误信息等详细诊断信息

6. **打包后显示命令行窗口**
   - 确保使用 Release 模式编译：`npm run tauri build`
   - 已为所有命令添加窗口隐藏标志，正常情况下不应显示窗口
   - 如果仍有窗口，检查是否是启动时的短暂闪现（这是正常的）

7. **端口已存在错误**
   - 这是正常情况，应用会自动跳过端口创建步骤
   - 如果端口确实不存在但报错，可能需要管理员权限

## 📦 部署说明

### Windows 平台部署

打包后需要分发的文件：

1. **主可执行文件**（必需）
   ```
   ePrinty.exe
   位置：src-tauri/target/release/ePrinty.exe
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

### 安装方式说明

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

## 📄 许可证

MIT License

## 👥 作者

Easy Printer Team

---

<div align="center">

**让打印这件事，简单一点** ✨

</div>
