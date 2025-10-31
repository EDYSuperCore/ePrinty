# 项目结构说明

## 完整目录结构

```
easyPrinter/
│
├── src/                              # Vue 3 前端源码
│   ├── App.vue                      # 主应用组件（包含状态管理和UI布局）
│   ├── main.js                      # Vue 应用入口文件
│   ├── style.css                    # 全局样式（TailwindCSS 指令）
│   └── components/
│       ├── PrinterArea.vue           # 办公区域组件（显示区域卡片）
│       └── PrinterItem.vue          # 打印机项组件（显示单个打印机）
│
├── src-tauri/                       # Tauri Rust 后端
│   ├── src/
│   │   └── main.rs                  # Rust 主程序
│   │                                #    - load_config: 加载服务器配置
│   │                                #    - list_printers: 获取本地打印机
│   │                                #    - install_printer: 安装打印机
│   ├── Cargo.toml                   # Rust 依赖配置
│   ├── build.rs                     # Rust 构建脚本
│   ├── tauri.conf.json              # Tauri 应用配置
│   └── icons/                       # 应用图标目录（可选）
│       ├── 32x32.png
│       ├── 128x128.png
│       ├── 128x128@2x.png
│       ├── icon.ico
│       └── icon.icns
│
├── index.html                       # HTML 入口文件
├── package.json                     # Node.js 依赖配置
├── vite.config.js                   # Vite 构建配置
├── tailwind.config.js               # TailwindCSS 配置
├── postcss.config.js                # PostCSS 配置
│
├── test_config.json                 # 测试配置示例（用于本地测试）
│
├── README.md                        # 项目主文档
├── INSTALL.md                       # 详细安装指南
├── QUICK_START.md                   # 快速开始指南
├── PROJECT_STRUCTURE.md             # 项目结构说明（本文件）
├── run_local_server.md              # 本地测试服务器设置
│
├── .gitignore                       # Git 忽略文件
└── .vscode/
    └── extensions.json              # VS Code 推荐扩展
```

## 文件职责说明

### 前端文件

| 文件 | 职责 |
|------|------|
| `src/App.vue` | 主应用组件，负责：<br>- 数据加载和状态管理<br>- 调用 Tauri 命令<br>- 布局和错误处理<br>- 状态栏显示 |
| `src/components/PrinterArea.vue` | 办公区域组件，负责：<br>- 显示区域名称<br>- 渲染打印机列表<br>- 判断打印机是否已安装 |
| `src/components/PrinterItem.vue` | 打印机项组件，负责：<br>- 显示打印机名称和路径<br>- 显示安装状态<br>- 触发安装事件 |
| `src/main.js` | Vue 应用入口，初始化应用 |
| `src/style.css` | 全局样式，引入 TailwindCSS |

### 后端文件

| 文件 | 职责 |
|------|------|
| `src-tauri/src/main.rs` | Rust 后端主程序：<br>- `load_config()`: 从服务器获取配置<br>- `list_printers()`: 获取本地已安装打印机<br>- `install_printer()`: 安装网络打印机 |
| `src-tauri/Cargo.toml` | Rust 依赖配置，包含：<br>- Tauri 框架<br>- HTTP 客户端 (reqwest)<br>- 序列化库 (serde) |
| `src-tauri/tauri.conf.json` | Tauri 应用配置：<br>- 窗口设置<br>- 权限配置<br>- 打包选项 |

### 配置文件

| 文件 | 职责 |
|------|------|
| `package.json` | Node.js 依赖和脚本配置 |
| `vite.config.js` | Vite 构建配置（端口、别名等） |
| `tailwind.config.js` | TailwindCSS 主题配置 |
| `postcss.config.js` | PostCSS 插件配置 |

## 数据流

### 应用启动流程

```
1. 用户启动应用
   ↓
2. App.vue mounted()
   ↓
3. 调用 loadData()
   ↓
4. 并行执行：
   - invoke('load_config')      → 获取服务器配置
   - invoke('list_printers')    → 获取本地打印机列表
   ↓
5. 更新状态并渲染 UI
```

### 安装打印机流程

```
1. 用户点击"安装"按钮
   ↓
2. PrinterItem.vue 触发 install 事件
   ↓
3. App.vue 调用 handleInstall()
   ↓
4. invoke('install_printer', { path })
   ↓
5. Rust 后端执行 Windows API 命令
   ↓
6. 返回安装结果
   ↓
7. 更新状态栏并刷新打印机列表
```

## 关键技术点

### 前后端通信

- **Tauri Commands**: 使用 `#[tauri::command]` 定义 Rust 函数
- **前端调用**: 使用 `invoke()` 调用后端命令
- **异步处理**: 所有命令都是异步的

### 状态管理

- 使用 Vue 3 的响应式数据（`data()`）
- 状态包括：配置数据、已安装打印机列表、错误信息等

### 样式系统

- **TailwindCSS**: 原子化 CSS 框架
- **响应式设计**: 使用 Tailwind 的响应式类
- **动画效果**: 使用 Tailwind 的动画类

## 开发建议

### 修改配置 URL

编辑 `src-tauri/src/main.rs` 第 34 行。

### 添加新功能

1. **前端**: 在 `App.vue` 或组件中添加
2. **后端**: 在 `src-tauri/src/main.rs` 中添加新的 `#[tauri::command]` 函数
3. **注册**: 在 `main()` 函数中注册新命令

### 调试技巧

- **前端**: 使用浏览器开发者工具（如果启用了调试）
- **后端**: 使用 `println!()` 或 Rust 调试器
- **日志**: 查看终端输出

## 扩展方向

1. **添加搜索功能**: 在前端添加打印机搜索
2. **添加过滤功能**: 按区域或状态过滤打印机
3. **添加批量安装**: 支持同时安装多个打印机
4. **添加日志记录**: 记录安装历史
5. **添加配置缓存**: 缓存配置以减少网络请求

