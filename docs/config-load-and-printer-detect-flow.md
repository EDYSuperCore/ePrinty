# 配置文件加载与打印机检测完整链路

## 一、应用启动时的配置加载流程

### 1.1 前端启动流程（App.vue - mounted）

```
应用启动
  ↓
mounted() 钩子
  ↓
checkVersionUpdate() - 检查版本更新
  ↓
loadData() - 加载配置数据（SWR 策略）
  ↓
startDetectInstalledPrinters() - 检测已安装打印机
```

### 1.2 配置加载详细流程（loadData）

#### 步骤 1: 读取缓存配置（立即渲染）

```javascript
// 前端调用
invoke('get_cached_config')
```

**后端处理流程：**

```
get_cached_config() [main.rs:552]
  ↓
seed_config_if_needed() - 如果本地配置不存在，从 seed 复制
  ↓
get_local_config_path() - 获取本地配置文件路径
  ↓
  ├─ Windows: %APPDATA%\easyPrinter\config\printer_config.json
  └─ 如果不存在 → 从 seed 复制（首次运行）
  ↓
fs::read_to_string() - 读取配置文件
  ↓
serde_json::from_str() - 解析 JSON
  ↓
validate_printer_config_v2() - 验证配置完整性
  ↓
返回 CachedConfigResult { config, source, timestamp, version }
```

**前端处理：**
- 立即使用缓存配置渲染 UI
- 设置 `this.config = cachedResult.config`
- 初始化打印机运行时状态
- 设置 `loading = false`，列表可见

#### 步骤 2: 后台刷新远程配置（非阻塞）

```javascript
// 前端调用（不 await，后台执行）
invoke('refresh_remote_config')
```

**后端处理流程：**

```
refresh_remote_config() [main.rs:600]
  ↓
load_remote_config() - 加载远程配置（3秒超时）
  ↓
  ├─ 创建 HTTP 客户端（5秒超时）
  ├─ GET https://p.edianyun.icu/printer_config.json
  └─ 解析 JSON 响应
  ↓
如果成功：
  ├─ save_config_to_local() - 保存到本地配置路径
  ├─ app.emit_all("config_updated", payload) - 发送配置更新事件
  └─ 返回 RefreshConfigResult { success, version }
  
如果失败：
  └─ app.emit_all("config_refresh_failed", error) - 发送失败事件
```

**前端处理：**
- 监听 `config_updated` 事件，热更新配置
- 监听 `config_refresh_failed` 事件，显示提示信息
- 不阻塞 UI，用户可立即使用

### 1.3 配置读取路径优先级

#### 有 AppHandle 时（Tauri 命令）

```
1. get_local_config_path()
   └─ %APPDATA%\easyPrinter\config\printer_config.json
   
2. 如果不存在 → seed_config_if_needed()
   └─ 从 seed 复制（资源目录或 exe 目录）
   
3. 如果仍不存在 → 回退到 load_local_config()
```

#### 无 AppHandle 时（Windows 平台模块）

```
load_local_config() [main.rs:694]
  ↓
按优先级搜索：
  1. 可执行文件所在目录
  2. 当前工作目录
  3. 项目根目录（开发模式）
```

## 二、打印机安装状态检测流程

### 2.1 前端检测流程（startDetectInstalledPrinters）

```
startDetectInstalledPrinters() [App.vue:2012]
  ↓
设置状态：printerDetect.status = 'running'
  ↓
循环尝试（最多 2 次）：
  ├─ 第 1 次：8 秒超时
  └─ 第 2 次：18 秒超时
  ↓
调用后端：invoke('list_printers')
  ↓
Promise.race([detectPromise, timeoutPromise])
  ↓
处理结果：
  ├─ 成功 → 更新 printerRuntime 状态
  ├─ 超时 → 重试或标记为 unknown
  └─ 错误 → 标记为 error
```

### 2.2 后端检测流程

```
list_printers() [main.rs:1334]
  ↓
crate::platform::list_printers() [platform/mod.rs:42]
  ↓
Windows 平台：
  └─ list_printers_windows() [windows/list.rs:18]
      ↓
      enum_printers_w() [windows/enum_printers.rs:190]
      ↓
      enum_printers_level_4() - 使用 Win32 API EnumPrintersW
      ↓
      返回 Vec<PrinterInfo>
      ↓
      转换为 Vec<PrinterDetectEntry>
      ↓
      返回前端
```

### 2.3 打印机匹配逻辑（前端）

前端收到已安装打印机列表后，进行匹配：

```
对每个配置中的打印机：
  ↓
1. 按 installedKey 匹配（精确匹配）
  ↓
2. 如果未匹配 → 按名称匹配（printerNameMatches）
  ↓
3. 如果仍未匹配 → 按 deviceUri 匹配
  ├─ buildDeviceUriFromPath() - 从配置路径构建 URI
  └─ normalizeDeviceUri() - 标准化系统 URI
  ↓
匹配结果：
  ├─ 匹配成功 → detectState = 'installed'
  │   └─ 保存 installedKey, systemQueueName, deviceUri
  └─ 未匹配 → detectState = 'not_installed'
```

### 2.4 打印机存在性检查（安装时）

在安装打印机前，会检查打印机是否已存在：

```
check_existing_printer() [windows/install.rs:1122]
  ↓
list_printers_windows() - 获取已安装打印机列表
  ↓
检查目标打印机名称是否在列表中
  ↓
返回 bool
```

或者使用更精确的检查：

```
printer_exists() [windows/printer_exists.rs:26]
  ↓
OpenPrinterW() - Win32 API 直接打开打印机
  ↓
返回 (exists, last_error, evidence)
```

## 三、完整时序图

```
应用启动
  │
  ├─→ [前端] mounted()
  │   │
  │   ├─→ checkVersionUpdate()
  │   │
  │   └─→ loadData()
  │       │
  │       ├─→ [1] invoke('get_cached_config')
  │       │   │
  │       │   └─→ [后端] get_cached_config()
  │       │       ├─→ seed_config_if_needed()
  │       │       ├─→ get_local_config_path()
  │       │       ├─→ fs::read_to_string()
  │       │       ├─→ serde_json::from_str()
  │       │       └─→ validate_printer_config_v2()
  │       │
  │       ├─→ [前端] 立即渲染 UI（使用缓存配置）
  │       │
  │       └─→ [2] invoke('refresh_remote_config') [后台，不阻塞]
  │           │
  │           └─→ [后端] refresh_remote_config()
  │               ├─→ load_remote_config() [3秒超时]
  │               ├─→ save_config_to_local()
  │               └─→ emit('config_updated')
  │
  └─→ startDetectInstalledPrinters()
      │
      └─→ invoke('list_printers') [最多 2 次重试]
          │
          └─→ [后端] list_printers()
              ├─→ platform::list_printers()
              ├─→ list_printers_windows()
              ├─→ enum_printers_w()
              └─→ EnumPrintersW() [Win32 API]
```

## 四、关键函数映射

### 配置加载相关

| 前端调用 | 后端函数 | 文件位置 | 说明 |
|---------|---------|---------|------|
| `get_cached_config` | `get_cached_config()` | `main.rs:552` | 读取本地缓存配置 |
| `refresh_remote_config` | `refresh_remote_config()` | `main.rs:600` | 刷新远程配置 |
| `load_config` | `load_config()` | `main.rs:937` | 加载配置（兼容旧接口） |

### 打印机检测相关

| 前端调用 | 后端函数 | 文件位置 | 说明 |
|---------|---------|---------|------|
| `list_printers` | `list_printers()` | `main.rs:1334` | 获取已安装打印机列表 |
| `list_printers_detailed` | `list_printers_detailed()` | `main.rs:1355` | 获取详细打印机信息 |

### 内部函数

| 函数名 | 文件位置 | 说明 |
|-------|---------|------|
| `get_local_config_path()` | `main.rs:449` | 获取本地配置文件路径 |
| `load_local_config()` | `main.rs:694` | 加载本地配置（回退机制） |
| `load_remote_config()` | `main.rs:1153` | 加载远程配置 |
| `list_printers_windows()` | `windows/list.rs:18` | Windows 平台打印机列表 |
| `enum_printers_w()` | `windows/enum_printers.rs:190` | 枚举打印机（Win32 API） |
| `printer_exists()` | `windows/printer_exists.rs:26` | 检查打印机是否存在 |

## 五、配置更新事件流

```
远程配置刷新成功
  ↓
emit('config_updated', { config, version })
  ↓
[前端] setupConfigUpdateListener()
  ↓
监听 'config_updated' 事件
  ↓
更新 this.config
  ↓
重新初始化打印机运行时状态
  ↓
显示更新提示
```

## 六、错误处理机制

### 配置加载错误

1. **本地配置不存在**
   - 尝试从 seed 复制
   - 如果 seed 也不存在 → 回退到 `load_local_config()` 搜索多个路径

2. **远程配置加载失败**
   - 不阻塞 UI，只显示提示
   - 继续使用本地缓存配置

3. **配置验证失败**
   - 返回详细错误信息
   - 前端显示错误提示

### 打印机检测错误

1. **检测超时**
   - 第 1 次：8 秒超时，自动重试
   - 第 2 次：18 秒超时，标记为 `unknown`

2. **检测失败**
   - 标记为 `error` 状态
   - 显示错误信息

3. **系统无打印机**
   - 返回空数组
   - 标记为 `empty` 状态

## 七、性能优化点

1. **SWR 策略**：先读缓存，后台刷新
2. **非阻塞刷新**：远程配置刷新不阻塞 UI
3. **超时控制**：远程请求 3 秒超时，检测最多 2 次重试
4. **直接 API 调用**：Windows 使用 EnumPrintersW，避免 PowerShell 冷启动延迟
5. **事件驱动更新**：配置更新通过事件通知，无需轮询
