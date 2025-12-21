# 使用 Visual Studio 开发环境编译 Tauri 项目
# 这个脚本会设置正确的环境变量，然后运行编译命令

$ErrorActionPreference = "Stop"

# 查找 Visual Studio 2022 的 VsDevCmd.bat（包括 Build Tools）
$vsPaths = @(
    "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat",
    "C:\Program Files\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat",
    "C:\Program Files\Microsoft Visual Studio\2022\Professional\Common7\Tools\VsDevCmd.bat",
    "C:\Program Files\Microsoft Visual Studio\2022\Enterprise\Common7\Tools\VsDevCmd.bat",
    "C:\Program Files (x86)\Microsoft Visual Studio\2019\BuildTools\Common7\Tools\VsDevCmd.bat",
    "C:\Program Files (x86)\Microsoft Visual Studio\2019\Community\Common7\Tools\VsDevCmd.bat"
)

$vsDevCmd = $null
foreach ($path in $vsPaths) {
    if (Test-Path $path) {
        $vsDevCmd = $path
        Write-Host "找到 Visual Studio Developer Command Prompt: $vsDevCmd" -ForegroundColor Green
        break
    }
}

if ($null -eq $vsDevCmd) {
    Write-Host "`n[错误] 未找到 Visual Studio Developer Command Prompt" -ForegroundColor Red
    Write-Host "`n请确保已安装以下之一：" -ForegroundColor Yellow
    Write-Host "  - Visual Studio 2022 (Community/Professional/Enterprise)" -ForegroundColor Yellow
    Write-Host "  - Visual Studio 2022 Build Tools" -ForegroundColor Yellow
    Write-Host "  - Visual Studio 2019 (Community/Professional/Enterprise)" -ForegroundColor Yellow
    Write-Host "`n并且必须包含 '使用 C++ 的桌面开发' 工作负载" -ForegroundColor Yellow
    Write-Host "`n安装步骤：" -ForegroundColor Cyan
    Write-Host "1. 下载 Visual Studio Installer" -ForegroundColor Cyan
    Write-Host "2. 选择 '使用 C++ 的桌面开发' 工作负载" -ForegroundColor Cyan
    Write-Host "3. 确保包含 Windows 10/11 SDK" -ForegroundColor Cyan
    exit 1
}

# 获取项目根目录
$projectRoot = $PSScriptRoot
if ($null -eq $projectRoot) {
    $projectRoot = Get-Location
}

Write-Host "项目目录: $projectRoot" -ForegroundColor Cyan

# 使用更可靠的方法设置 Visual Studio 环境
Write-Host "`n正在设置 Visual Studio 开发环境..." -ForegroundColor Yellow

# 创建一个临时批处理文件来设置环境并执行构建
$tempBatFile = Join-Path $env:TEMP "tauri-build-$(Get-Date -Format 'yyyyMMddHHmmss').bat"
$batContent = @"
@echo off
setlocal enabledelayedexpansion

cd /d "$projectRoot"

echo 正在加载 Visual Studio 开发环境...
call "$vsDevCmd" -arch=x64 -host_arch=x64
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [错误] 无法加载 Visual Studio 开发环境
    echo 请确保已安装 Visual Studio 2022 或 2019，并包含 C++ 桌面开发工作负载
    exit /b 1
)

echo.
echo ========================================
echo Visual Studio 环境已加载
echo ========================================
echo VCINSTALLDIR: %VCINSTALLDIR%
echo WindowsSdkDir: %WindowsSdkDir%
echo.

echo 开始构建 Tauri 应用...
echo.
npm run tauri build

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo ========================================
    echo [错误] 构建失败
    echo ========================================
    exit /b 1
)

echo.
echo ========================================
echo [成功] 构建完成！
echo ========================================
"@

try {
    $batContent | Out-File -FilePath $tempBatFile -Encoding ASCII -Force
    Write-Host "临时批处理文件已创建: $tempBatFile" -ForegroundColor Cyan
    
    Write-Host "`n正在使用 Visual Studio 开发环境编译..." -ForegroundColor Yellow
    Write-Host "这可能需要一些时间，请耐心等待...`n" -ForegroundColor Yellow
    
    # 使用 cmd 执行批处理文件，并显示输出
    $process = Start-Process -FilePath "cmd.exe" -ArgumentList "/c", "`"$tempBatFile`"" -Wait -NoNewWindow -PassThru
    
    if ($process.ExitCode -ne 0) {
        Write-Host "`n构建失败，退出代码: $($process.ExitCode)" -ForegroundColor Red
        Write-Host "`n如果遇到 'excpt.h' 或类似错误，请确保：" -ForegroundColor Yellow
        Write-Host "1. 已安装 Visual Studio 2022 或 2019" -ForegroundColor Yellow
        Write-Host "2. 已安装 '使用 C++ 的桌面开发' 工作负载" -ForegroundColor Yellow
        Write-Host "3. 已安装 Windows 10/11 SDK" -ForegroundColor Yellow
        exit $process.ExitCode
    } else {
        Write-Host "`n构建完成！" -ForegroundColor Green
    }
} finally {
    # 清理临时文件
    if (Test-Path $tempBatFile) {
        Remove-Item $tempBatFile -Force -ErrorAction SilentlyContinue
    }
}

