# 应用图标说明

## 图标文件要求

Tauri 应用需要以下图标文件：

### Windows 平台
- `icon.ico` - Windows 图标文件（256x256 或更大，包含多个尺寸）
- `32x32.png` - 小图标
- `128x128.png` - 中等图标
- `128x128@2x.png` (256x256) - 高分辨率图标

### macOS 平台（可选）
- `icon.icns` - macOS 图标文件

## 创建图标

### 方法 1: 使用在线工具

1. 准备一个 1024x1024 的 PNG 图标
2. 访问 https://convertio.co/png-ico/ 转换为 ICO 格式
3. 或使用 https://www.icoconverter.com/ 生成多尺寸图标

### 方法 2: 使用 ImageMagick

```bash
# 安装 ImageMagick
# 然后运行以下命令生成图标

# 生成 ICO 文件（包含多个尺寸）
magick convert icon.png -define icon:auto-resize=256,128,64,48,32,16 icon.ico

# 生成 PNG 文件
magick convert icon.png -resize 32x32 32x32.png
magick convert icon.png -resize 128x128 128x128.png
magick convert icon.png -resize 256x256 128x128@2x.png
```

### 方法 3: 使用专业工具

- **GIMP** (免费)
- **Photoshop** (付费)
- **IconWorkshop** (Windows)

## 图标放置

将生成的图标文件放在此目录（`src-tauri/icons/`）下。

如果没有图标文件，Tauri 会使用默认图标构建应用。

## 图标设计建议

1. **尺寸**: 原始图标建议 1024x1024 或更大
2. **格式**: PNG 格式，透明背景
3. **内容**: 简洁明了，在小尺寸下仍清晰可见
4. **颜色**: 避免过于复杂的渐变，在小图标下会模糊

