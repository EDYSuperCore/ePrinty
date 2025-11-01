# 发布文件清单

## Windows 平台发布文件

### 必须分发的文件

#### 1. **主可执行文件**（必需）
```
易点云打印机安装小精灵.exe
位置：src-tauri/target/release/易点云打印机安装小精灵.exe
说明：这是应用程序的主可执行文件，必须分发
```

#### 2. **配置文件**（必需）
```
printer_config.json
位置：src-tauri/target/release/printer_config.json
说明：打印机配置文件，应用程序会自动从可执行文件所在目录加载
```

#### 3. **资源文件**（已嵌入 exe）
- **`prnport.vbs`** - 已通过 `include_str!` 嵌入到 exe 中，**不需要单独分发**
- **`ricoh320.pdd`** - macOS 打印机 PPD 文件（仅在 macOS 需要，通过 Tauri resources 打包）

### 分发包结构建议

#### 方案 1：单文件夹分发（推荐）
```
易点云打印机安装小精灵/
├── 易点云打印机安装小精灵.exe    # 主程序（包含 prnport.vbs）
└── printer_config.json            # 配置文件
```

用户只需：
1. 将整个文件夹复制到目标位置
2. 双击运行 `易点云打印机安装小精灵.exe`
3. 可根据需要修改 `printer_config.json` 配置文件

#### 方案 2：压缩包分发
将上述文件打包成 ZIP 文件：
```
易点云打印机安装小精灵_v1.0.0.zip
├── 易点云打印机安装小精灵.exe
└── printer_config.json
```

### 文件大小参考

- **易点云打印机安装小精灵.exe**：约 10-20 MB（包含所有依赖）
- **printer_config.json**：通常几 KB 到几十 KB（取决于打印机数量）

### 不需要分发的文件

以下文件**不需要**随应用分发：
- ❌ `*.pdb` - 调试符号文件（仅在调试时需要）
- ❌ `target/release/build/` - 构建中间文件
- ❌ `target/release/deps/` - 依赖库文件（已静态链接）
- ❌ `target/release/incremental/` - 增量编译缓存
- ❌ `node_modules/` - Node.js 依赖（仅开发时需要）
- ❌ `src/` - 源代码（仅在开发时需要）
- ❌ `src-tauri/src/` - Rust 源代码（仅在开发时需要）

### 构建命令

```bash
# 构建发布版本
npm run tauri build

# 构建后的文件位置
src-tauri/target/release/易点云打印机安装小精灵.exe
src-tauri/target/release/printer_config.json
```

### 部署步骤

1. **构建应用**
   ```bash
   npm run tauri build
   ```

2. **检查文件**
   确认以下文件存在：
   - `src-tauri/target/release/易点云打印机安装小精灵.exe`
   - `src-tauri/target/release/printer_config.json`

3. **创建分发包**
   - 创建一个新文件夹（如 `易点云打印机安装小精灵_v1.0.0`）
   - 复制 `易点云打印机安装小精灵.exe` 到该文件夹
   - 复制 `printer_config.json` 到该文件夹
   - （可选）压缩成 ZIP 文件

4. **分发**
   - 可以直接分发文件夹
   - 或分发 ZIP 压缩包
   - 或分发到文件服务器/内网共享

### 注意事项

1. **配置文件位置**
   - 应用程序会按以下顺序查找 `printer_config.json`：
     1. 可执行文件所在目录（优先）
     2. 当前工作目录
     3. 父目录（开发模式）
   
2. **用户权限**
   - 安装打印机需要管理员权限
   - 建议用户以管理员身份运行程序

3. **文件权限**
   - 确保 `printer_config.json` 文件可读可写（用户可能需要修改配置）

4. **路径问题**
   - 可执行文件和配置文件应在同一目录
   - 避免路径中包含特殊字符（如果有问题）

### macOS 平台（如果将来需要）

如果需要发布 macOS 版本：
```
易点云打印机安装小精灵.app/          # macOS 应用程序包（文件夹）
└── Contents/
    ├── MacOS/
    │   └── 易点云打印机安装小精灵      # 可执行文件（已打包）
    └── Resources/
        ├── printer_config.json       # 配置文件
        └── ricoh320.pdd              # PPD 文件（已打包）
```

位置：`src-tauri/target/release/bundle/macos/易点云打印机安装小精灵.app`

---

## 快速检查清单

构建完成后，检查以下文件是否存在：

- [ ] `src-tauri/target/release/易点云打印机安装小精灵.exe` 存在
- [ ] `src-tauri/target/release/printer_config.json` 存在
- [ ] 文件大小合理（exe 约 10-20 MB）
- [ ] 在干净的环境中测试运行正常
- [ ] 可以读取和修改 `printer_config.json`

