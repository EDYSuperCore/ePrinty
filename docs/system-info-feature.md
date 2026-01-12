# 系统信息功能说明

## 功能概述

在主界面设置对话框中新增"信息"标签页，展示当前操作系统的详细信息，包括操作系统版本、系统架构（x64/x86/arm64）、应用版本等。

## 使用方法

1. 点击主界面右上角的"设置"按钮
2. 在弹出的设置对话框左侧导航中，点击"信息"标签
3. 系统会自动加载并显示系统信息

## 显示内容

### 1. 操作系统
- **Windows**: 显示完整的 Windows 版本信息，例如 "Microsoft Windows 11 Pro"
- **macOS**: 显示 macOS 版本，例如 "macOS 14.0"
- **Linux**: 显示发行版信息，例如 "Ubuntu 22.04"

### 2. 系统架构
显示当前系统的 CPU 架构：
- `x64`: 64位 x86 架构（Intel/AMD）
- `x86`: 32位 x86 架构
- `arm64`: 64位 ARM 架构（如 Apple Silicon）

### 3. 应用版本
显示当前 Easy Printer 应用的版本号，例如 "v1.4.1"

### 4. 平台类型
显示操作系统平台标识：
- `windows`
- `macos`
- `linux`

## 技术实现

### 后端 API

新增 Tauri 命令 `get_system_info()`，支持跨平台系统信息获取：

```rust
#[tauri::command]
fn get_system_info() -> Result<SystemInfo, String>
```

**返回结构**：
```typescript
interface SystemInfo {
  platform: string;       // "windows" | "macos" | "linux"
  osVersion: string;      // 操作系统版本
  arch: string;           // "x64" | "x86" | "arm64"
  appVersion: string;     // 应用版本号
  kernelVersion?: string; // 内核版本（可选）
}
```

### 前端调用

在 Vue 组件中通过 Tauri invoke API 调用：

```javascript
import { invoke } from '@tauri-apps/api/tauri'

async loadSystemInfo() {
  try {
    const info = await invoke('get_system_info')
    this.systemInfo = info
  } catch (err) {
    this.systemInfoError = err.toString()
  }
}
```

## 平台特性

### Windows
- 使用 `systeminfo` 命令或 `cmd /c ver` 获取系统版本
- 自动检测 Windows 10/11 版本号

### macOS
- 使用 `sw_vers` 命令获取系统版本
- 格式：产品名称 + 版本号

### Linux
- 优先读取 `/etc/os-release` 文件
- 备选方案：使用 `lsb_release -d` 命令
- 显示发行版名称和版本号

## 错误处理

- **加载中**: 显示加载动画和提示文字
- **加载失败**: 显示错误信息和"重试"按钮
- **未加载**: 显示"加载系统信息"按钮

## 代码改动

### 修改的文件

1. **src-tauri/src/main.rs**
   - 新增 `SystemInfo` 结构体
   - 新增 `get_system_info()` 命令
   - 新增平台特定的版本获取函数（`get_windows_version`, `get_macos_version`, `get_linux_version`）
   - 在命令处理器中注册 `get_system_info`

2. **src/App.vue**
   - 在 data() 中添加系统信息相关状态（`systemInfo`, `systemInfoLoading`, `systemInfoError`）
   - 新增 `loadSystemInfo()` 方法
   - 在设置对话框左侧导航添加"信息"按钮
   - 新增"信息"标签页内容，展示系统信息

## 验证测试

### 编译验证
- ✅ Rust 后端编译通过（`cargo check`）
- ✅ 前端构建成功（`npm run build`）
- ✅ 无 TypeScript/Vue 错误

### 功能测试建议
1. 在 Windows 10/11 上测试系统信息显示是否正确
2. 验证架构信息（x64/x86/arm64）是否正确
3. 测试错误处理（模拟 API 失败）
4. 验证"重试"按钮功能

## 注意事项

1. **跨平台兼容性**: 代码已适配 Windows、macOS 和 Linux，但需在不同平台上实际测试
2. **错误恢复**: 系统信息获取失败不会影响应用其他功能
3. **性能**: 系统信息仅在用户点击"信息"标签时加载，避免应用启动时的性能开销
4. **缓存**: 当前实现不缓存系统信息，每次切换到"信息"标签页都会重新加载（可根据需要优化）

## 未来改进

- [ ] 添加更多系统信息（内存、CPU、硬盘等）
- [ ] 添加系统信息复制到剪贴板功能
- [ ] 缓存系统信息以提升响应速度
- [ ] 显示网络适配器信息
- [ ] 添加系统健康检查功能
