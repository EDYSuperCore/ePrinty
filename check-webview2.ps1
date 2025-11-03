# PowerShell 脚本：检查 WebView2 运行时是否已安装
# 使用方法：.\check-webview2.ps1

Write-Host "正在检查 WebView2 运行时..." -ForegroundColor Cyan
Write-Host ""

# 方法1：检查注册表（最常见的安装位置）
$webView2Installed = $false
$webView2Version = $null

# 检查 64 位注册表
$regPath64 = "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E9C5}"
# 检查 32 位注册表
$regPath32 = "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E9C5}"

if (Test-Path $regPath64) {
    $webView2Version = (Get-ItemProperty -Path $regPath64 -ErrorAction SilentlyContinue).pv
    if ($webView2Version) {
        $webView2Installed = $true
        Write-Host "✓ 找到 WebView2 (64位): 版本 $webView2Version" -ForegroundColor Green
    }
}

if (Test-Path $regPath32) {
    $version32 = (Get-ItemProperty -Path $regPath32 -ErrorAction SilentlyContinue).pv
    if ($version32) {
        $webView2Installed = $true
        if ($webView2Version) {
            Write-Host "✓ 找到 WebView2 (32位): 版本 $version32" -ForegroundColor Green
        } else {
            Write-Host "✓ 找到 WebView2 (32位): 版本 $version32" -ForegroundColor Green
            $webView2Version = $version32
        }
    }
}

# 方法2：检查文件系统（备用方法）
if (-not $webView2Installed) {
    $possiblePaths = @(
        "${env:ProgramFiles(x86)}\Microsoft\EdgeWebView\Application",
        "${env:ProgramFiles}\Microsoft\EdgeWebView\Application",
        "${env:LOCALAPPDATA}\Microsoft\EdgeWebView\Application"
    )
    
    foreach ($path in $possiblePaths) {
        if (Test-Path $path) {
            $versions = Get-ChildItem -Path $path -Directory -ErrorAction SilentlyContinue | 
                        Sort-Object Name -Descending | Select-Object -First 1
            if ($versions) {
                $webView2Installed = $true
                $webView2Version = $versions.Name
                Write-Host "✓ 找到 WebView2: 版本 $webView2Version (路径: $path)" -ForegroundColor Green
                break
            }
        }
    }
}

# 方法3：检查是否安装了 Edge 浏览器（Edge 包含 WebView2）
if (-not $webView2Installed) {
    $edgePaths = @(
        "${env:ProgramFiles(x86)}\Microsoft\Edge\Application",
        "${env:ProgramFiles}\Microsoft\Edge\Application",
        "${env:LOCALAPPDATA}\Microsoft\Edge\Application"
    )
    
    foreach ($path in $edgePaths) {
        if (Test-Path $path) {
            Write-Host "⚠ 找到 Microsoft Edge，但未找到独立的 WebView2 运行时" -ForegroundColor Yellow
            Write-Host "  Edge 通常包含 WebView2，但可能需要重新安装" -ForegroundColor Yellow
            break
        }
    }
}

Write-Host ""

if ($webView2Installed) {
    Write-Host "✓ WebView2 已安装" -ForegroundColor Green
    Write-Host "  版本: $webView2Version" -ForegroundColor Gray
    Write-Host ""
    Write-Host "如果应用仍然无法启动，请尝试：" -ForegroundColor Yellow
    Write-Host "  1. 重启计算机" -ForegroundColor Yellow
    Write-Host "  2. 重新安装 WebView2 运行时" -ForegroundColor Yellow
    Write-Host "  3. 检查应用是否以管理员权限运行" -ForegroundColor Yellow
} else {
    Write-Host "✗ WebView2 未安装或未找到" -ForegroundColor Red
    Write-Host ""
    Write-Host "解决方案：" -ForegroundColor Yellow
    Write-Host "  1. 下载并安装 Microsoft Edge WebView2 运行时" -ForegroundColor Yellow
    Write-Host "     下载地址: https://go.microsoft.com/fwlink/p/?LinkId=2124703" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  2. 或者安装 Microsoft Edge 浏览器（会自动安装 WebView2）" -ForegroundColor Yellow
    Write-Host "     下载地址: https://www.microsoft.com/edge" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "  3. 安装完成后，重启应用程序" -ForegroundColor Yellow
    Write-Host ""
    
    # 询问是否打开下载页面
    $openPage = Read-Host "是否打开 WebView2 下载页面？(Y/N)"
    if ($openPage -eq "Y" -or $openPage -eq "y") {
        Start-Process "https://go.microsoft.com/fwlink/p/?LinkId=2124703"
    }
}

Write-Host ""
Write-Host "按任意键退出..."
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")

