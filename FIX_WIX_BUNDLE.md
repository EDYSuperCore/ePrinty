# 解决 WIX 打包错误

## 问题描述

打包时出现错误：
```
Error failed to bundle project: error running light.exe
```

这通常是由于：
1. 产品名称包含中文字符，导致 MSI 文件名或内部标识出现问题
2. WIX 工具对中文字符的支持不完善

## 解决方案

### 方案 1：禁用 MSI 打包，只生成 exe（已应用，推荐）

已修改 `src-tauri/tauri.conf.json`，将 `targets` 从 `"all"` 改为 `["app"]`：

```json
"bundle": {
  "active": true,
  "targets": ["app"],  // 只生成 exe 文件
  ...
}
```

这样会生成独立的 `.exe` 文件，不包含 MSI 安装包。

### 方案 2：使用 NSIS 打包（需要安装 NSIS）

如果想生成安装包，可以使用 NSIS（对中文支持更好）：

1. 安装 NSIS：
   - 下载：https://nsis.sourceforge.io/Download
   - 安装后将 NSIS 添加到 PATH

2. 修改 `src-tauri/tauri.conf.json`：
   ```json
   "targets": ["nsis"]
   ```

3. 重新打包

### 方案 3：使用英文产品名称（临时方案）

如果确实需要 MSI 安装包，可以临时使用英文产品名称：

1. 修改 `src-tauri/tauri.conf.json`：
   ```json
   "package": {
     "productName": "EasyPrinter",
     ...
   },
   "bundle": {
     "shortDescription": "EasyPrinter",
     "longDescription": "Enterprise intranet printer installation tool"
   }
   ```

2. 窗口标题和显示名称可以在代码中单独设置中文

## 推荐方案

**方案 1（已应用）**：只生成 `.exe` 文件
- ✅ 简单直接
- ✅ 无需额外工具
- ✅ 可以手动复制配置文件
- ✅ 适合内部使用

## 使用说明

打包后的文件：
- **位置**: `src-tauri/target/release/easy-printer.exe`
- **配置**: 需要将 `printer_config.json` 放在 exe 文件同一目录
- **分发**: 可以压缩 `easy-printer.exe` 和 `printer_config.json` 一起分发

## 手动创建安装包（可选）

如果需要安装包，可以：

1. **使用 Inno Setup**（推荐，对中文支持好）：
   - 下载：https://jrsoftware.org/isdl.php
   - 创建安装脚本，包含 exe 和配置文件

2. **使用 7-Zip 自解压**：
   - 使用 7-Zip 创建自解压压缩包
   - 解压后自动运行 exe

