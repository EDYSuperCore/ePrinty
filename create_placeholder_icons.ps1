# 创建占位图标文件的脚本
# 注意：这只是一个临时解决方案，实际应用中应该使用真实的图标

Write-Host "创建占位图标文件..." -ForegroundColor Yellow

$iconsDir = "src-tauri\icons"

# 确保目录存在
if (-not (Test-Path $iconsDir)) {
    New-Item -ItemType Directory -Path $iconsDir -Force | Out-Null
}

# 创建占位图标说明
$placeholderInfo = @"
# 占位图标文件

这些图标文件是占位符。在正式发布前，应该替换为真实的应用程序图标。

## 需要的图标文件

- icon.ico (Windows 图标文件，至少 256x256)
- icon.icns (macOS 图标文件)
- 32x32.png
- 128x128.png
- 128x128@2x.png (256x256)

## 创建图标的方法

1. 准备一个 1024x1024 的 PNG 图标
2. 使用在线工具转换为 ICO 格式：https://convertio.co/png-ico/
3. 将生成的文件放入此目录

注意：Tauri 开发模式需要至少 icon.ico 文件才能正常构建。
"@

$placeholderInfo | Out-File -FilePath "$iconsDir\PLACEHOLDER_INFO.txt" -Encoding UTF8

Write-Host ""
Write-Host "⚠️  图标文件缺失！" -ForegroundColor Red
Write-Host ""
Write-Host "Tauri 需要图标文件才能构建。有几种解决方案：" -ForegroundColor Yellow
Write-Host ""
Write-Host "方案 1：创建占位图标（快速）" -ForegroundColor Cyan
Write-Host "  1. 访问 https://tauri.app/v1/guides/building/icons" -ForegroundColor White
Write-Host "  2. 下载 Tauri 默认图标" -ForegroundColor White
Write-Host "  3. 放入 $iconsDir 目录" -ForegroundColor White
Write-Host ""
Write-Host "方案 2：临时禁用图标检查（仅开发）" -ForegroundColor Cyan
Write-Host "  修改 src-tauri/tauri.conf.json，移除 icon 配置" -ForegroundColor White
Write-Host ""
Write-Host "方案 3：使用在线工具生成图标" -ForegroundColor Cyan
Write-Host "  1. 准备 1024x1024 PNG 图标" -ForegroundColor White
Write-Host "  2. 访问 https://convertio.co/png-ico/ 转换" -ForegroundColor White
Write-Host "  3. 放入 $iconsDir 目录" -ForegroundColor White

