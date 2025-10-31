# 恢复环境变量脚本
# 如果刷新脚本破坏了环境变量，可以运行此脚本恢复

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "   恢复环境变量" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "正在恢复环境变量..." -ForegroundColor Yellow

# 添加 Rust 路径（如果存在）
$rustPath = "$env:USERPROFILE\.cargo\bin"
if (Test-Path $rustPath) {
    if ($env:Path -notlike "*$rustPath*") {
        $env:Path += ";$rustPath"
        Write-Host "✅ 已添加 Rust 路径" -ForegroundColor Green
    } else {
        Write-Host "✅ Rust 路径已存在" -ForegroundColor Green
    }
}

# 重新加载系统 PATH 和用户 PATH
$machinePath = [System.Environment]::GetEnvironmentVariable("Path","Machine")
$userPath = [System.Environment]::GetEnvironmentVariable("Path","User")

# 合并：系统 + 用户 + 当前（如果已添加了 Rust）
$newPath = @()
if ($machinePath) { $newPath += ($machinePath -split ';') | Where-Object { $_ } }
if ($userPath) { $newPath += ($userPath -split ';') | Where-Object { $_ } }

# 去重
$uniquePaths = @()
$seenPaths = @{}
foreach ($path in $newPath) {
    if ($path -and -not $seenPaths.ContainsKey($path)) {
        $seenPaths[$path] = $true
        $uniquePaths += $path
    }
}

# 添加 Rust 路径（如果还没有）
if ($rustPath -and -not $seenPaths.ContainsKey($rustPath)) {
    $uniquePaths += $rustPath
}

$env:Path = $uniquePaths -join ';'

Write-Host "✅ 环境变量已恢复（但可能缺少 fnm 动态路径）" -ForegroundColor Yellow
Write-Host "⚠️  建议：重启 VS Code 以获得完整的环境变量" -ForegroundColor Yellow
Write-Host ""

# 测试
Write-Host "测试命令..." -ForegroundColor Yellow
try {
    $rustVersion = rustc --version 2>$null
    if ($rustVersion) {
        Write-Host "✅ rustc 可用: $rustVersion" -ForegroundColor Green
    } else {
        Write-Host "❌ rustc 不可用" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ rustc 不可用" -ForegroundColor Red
}

try {
    $nodeVersion = node --version 2>$null
    if ($nodeVersion) {
        Write-Host "✅ node 可用: $nodeVersion" -ForegroundColor Green
    } else {
        Write-Host "❌ node 不可用（可能需要重启 VS Code 以加载 fnm 路径）" -ForegroundColor Yellow
    }
} catch {
    Write-Host "❌ node 不可用（可能需要重启 VS Code 以加载 fnm 路径）" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "提示：" -ForegroundColor Cyan
Write-Host "  - 如果 node 不可用，请重启 VS Code" -ForegroundColor Cyan
Write-Host "  - 重启 VS Code 是最可靠的解决方法" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

