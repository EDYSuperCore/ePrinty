# 图标文件缺失问题解决方案

## 错误信息
```
`icons/icon.ico` not found; required for generating a Windows Resource file during tauri-build
```

## 快速解决方案

### 方案 1：下载 Tauri 默认图标（推荐）

1. 访问 Tauri 图标生成工具：
   - https://tauri.app/v1/guides/building/icons
   - 或 https://github.com/tauri-apps/tauri-icon

2. 下载默认图标包或使用在线工具生成

3. 将图标文件放入 `src-tauri/icons/` 目录：
   - `icon.ico` (必需)
   - `icon.icns` (可选，macOS)
   - `32x32.png` (可选)
   - `128x128.png` (可选)
   - `128x128@2x.png` (可选)

### 方案 2：使用命令行工具生成（如果已安装）

```bash
# 如果有 SVG 图标
cd src-tauri
npm install -g @tauri-apps/cli
tauri icon path/to/icon.svg
```

### 方案 3：临时禁用图标（仅用于开发测试）

修改 `src-tauri/tauri.conf.json`，暂时移除图标配置：

```json
"bundle": {
  "active": false,  // 改为 false，禁用打包（仅用于开发）
  ...
}
```

或者修改 `bundle.active` 为 `false`，但这会禁用打包功能。

### 方案 4：创建最小占位图标

使用在线工具创建最小的 ICO 文件：
1. 访问 https://convertio.co/png-ico/
2. 上传任意 256x256 PNG 图片
3. 转换为 ICO 格式
4. 保存为 `src-tauri/icons/icon.ico`

## 推荐的图标尺寸

- **icon.ico**: 至少 256x256，包含多个尺寸
- **icon.icns**: macOS 图标格式
- **32x32.png**: 32x32 像素
- **128x128.png**: 128x128 像素
- **128x128@2x.png**: 256x256 像素（2倍分辨率）

## 快速修复（最小文件）

至少需要 `icon.ico` 文件。可以创建一个简单的占位图标：

1. 下载或创建一个 256x256 的 PNG 图标
2. 使用在线工具转换为 ICO：https://convertio.co/png-ico/
3. 保存为 `src-tauri/icons/icon.ico`

## 注意事项

- 开发模式下至少需要 `icon.ico` 文件
- 生产环境建议使用完整的图标集
- 图标文件大小会影响应用程序体积

