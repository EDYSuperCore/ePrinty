# 解决编译时的内存分配失败问题

## 问题描述

编译时出现错误：
```
memory allocation of 2088960 bytes failed
Allocation failed
error: could not compile `webview2-com-sys` (lib)
```

这通常是由于编译时内存不足导致的。

## 解决方案

### 方案 1：限制并行编译任务数（推荐）

已在项目根目录创建 `.cargo/config.toml` 文件，限制并行任务数为 2。

如果仍然内存不足，可以修改 `.cargo/config.toml`：

```toml
[build]
# 使用单线程编译（最省内存）
jobs = 1
```

### 方案 2：清理构建缓存后重新编译

```powershell
# 在项目根目录执行
cd src-tauri
cargo clean
cargo build
```

### 方案 3：增加虚拟内存

Windows 系统可以增加虚拟内存（页面文件）：

1. 打开"控制面板" → "系统" → "高级系统设置"
2. 点击"性能"区域的"设置"
3. 选择"高级"选项卡 → "虚拟内存" → "更改"
4. 增加虚拟内存大小（建议设置为物理内存的 1.5-2 倍）

### 方案 4：使用环境变量限制任务数

临时设置（仅当前会话有效）：

```powershell
# PowerShell
$env:CARGO_BUILD_JOBS = "1"
cargo build

# 或者
$env:CARGO_BUILD_JOBS = "2"
cargo build
```

### 方案 5：分批编译依赖

```powershell
# 先编译依赖，再编译主程序
cd src-tauri
cargo build --dependencies-only
cargo build
```

### 方案 6：关闭其他占用内存的程序

编译前关闭：
- 浏览器（特别是多个标签页）
- IDE 中的其他项目
- 其他大型应用程序

## 推荐执行顺序

1. **首先尝试**：方案 1（已自动配置）
2. **如果还是失败**：方案 2（清理缓存）+ 方案 1（jobs = 1）
3. **仍然失败**：方案 3（增加虚拟内存）+ 方案 1

## 检查当前配置

```powershell
# 检查 Cargo 配置
Get-Content .cargo/config.toml

# 检查系统内存
systeminfo | Select-String "Total Physical Memory"
```

## 注意事项

- `webview2-com-sys` 是一个大型依赖，编译时需要较多内存
- 第一次编译会比后续编译使用更多内存（需要下载和编译所有依赖）
- 如果经常遇到内存问题，建议增加系统内存或虚拟内存

