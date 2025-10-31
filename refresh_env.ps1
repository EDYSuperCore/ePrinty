# 刷新 PowerShell 环境变量脚本
# 在 VS Code 终端中运行此脚本可以刷新环境变量

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "   刷新 PowerShell 环境变量" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

Write-Host "正在刷新环境变量..." -ForegroundColor Yellow

# 方法：保留当前 PATH，然后添加缺失的系统路径和用户路径
# 同时自动查找最新的 fnm 动态路径

# 获取系统 PATH 和用户 PATH
$machinePath = [System.Environment]::GetEnvironmentVariable("Path","Machine")
$userPath = [System.Environment]::GetEnvironmentVariable("Path","User")

# 将当前 PATH、系统 PATH 和用户 PATH 都分割成数组
$currentPathArray = $env:Path -split ';' | Where-Object { $_ }
$machinePathArray = if ($machinePath) { $machinePath -split ';' | Where-Object { $_ } } else { @() }
$userPathArray = if ($userPath) { $userPath -split ';' | Where-Object { $_ } } else { @() }

# 查找最新的 fnm 路径（如果存在）
$fnmBasePath = "$env:USERPROFILE\AppData\Local\fnm_multishells"
$latestFnmPath = $null
if (Test-Path $fnmBasePath) {
    try {
        # 获取最新的 fnm 目录（按修改时间排序）
        $latestFnmDir = Get-ChildItem $fnmBasePath -Directory -ErrorAction SilentlyContinue | 
            Sort-Object LastWriteTime -Descending | 
            Select-Object -First 1
        
        if ($latestFnmDir) {
            $latestFnmPath = $latestFnmDir.FullName
            Write-Host "🔍 找到最新的 fnm 路径: $latestFnmPath" -ForegroundColor Cyan
        }
    } catch {
        # 忽略错误，继续执行
    }
}

# 合并所有路径：先系统路径，再用户路径，最后当前会话路径（保持优先级）
$allPaths = @()
$allPaths += $machinePathArray
$allPaths += $userPathArray
$allPaths += $currentPathArray

# 如果找到最新的 fnm 路径，添加到路径列表
if ($latestFnmPath) {
    $allPaths += $latestFnmPath
}

# 去重（保留第一次出现的顺序）
$uniquePaths = @()
$seenPaths = @{}
foreach ($path in $allPaths) {
    # 清理路径（移除末尾的斜杠等）
    $cleanPath = $path.Trim().TrimEnd('\')
    if ($cleanPath -and -not $seenPaths.ContainsKey($cleanPath)) {
        $seenPaths[$cleanPath] = $true
        $uniquePaths += $cleanPath
    }
}

# 更新环境变量
$env:Path = $uniquePaths -join ';'

Write-Host "✅ 已合并系统 PATH、用户 PATH 和当前会话 PATH" -ForegroundColor Green
if ($latestFnmPath) {
    Write-Host "✅ 已添加最新的 fnm 路径" -ForegroundColor Green
}

Write-Host "✅ 环境变量已刷新！" -ForegroundColor Green
Write-Host ""

# 测试 Node.js
Write-Host "测试 Node.js 安装..." -ForegroundColor Yellow
try {
    $nodeVersion = node --version 2>$null
    if ($nodeVersion) {
        Write-Host "✅ Node.js 可用: $nodeVersion" -ForegroundColor Green
    } else {
        Write-Host "❌ Node.js 不可用" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ Node.js 仍然不可用，请重启 VS Code" -ForegroundColor Red
}

# 测试 npm
Write-Host "测试 npm 安装..." -ForegroundColor Yellow
try {
    $npmVersion = npm --version 2>$null
    if ($npmVersion) {
        Write-Host "✅ npm 可用: v$npmVersion" -ForegroundColor Green
    } else {
        Write-Host "❌ npm 不可用" -ForegroundColor Red
    }
} catch {
    Write-Host "❌ npm 仍然不可用，请重启 VS Code" -ForegroundColor Red
}

# 测试 Rust
Write-Host "测试 Rust 安装..." -ForegroundColor Yellow
try {
    $rustVersion = rustc --version 2>$null
    if ($rustVersion) {
        Write-Host "✅ Rust 可用: $rustVersion" -ForegroundColor Green
    } else {
        Write-Host "⚠️  Rust 未安装（如果不需要可以忽略）" -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠️  Rust 未安装或不可用（如果不需要可以忽略）" -ForegroundColor Yellow
}

# 测试 Cargo
Write-Host "测试 Cargo 安装..." -ForegroundColor Yellow
try {
    $cargoVersion = cargo --version 2>$null
    if ($cargoVersion) {
        Write-Host "✅ Cargo 可用: $cargoVersion" -ForegroundColor Green
    } else {
        Write-Host "⚠️  Cargo 未安装（如果不需要可以忽略）" -ForegroundColor Yellow
    }
} catch {
    Write-Host "⚠️  Cargo 未安装或不可用（如果不需要可以忽略）" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "提示：" -ForegroundColor Cyan
Write-Host "  - 如果某些命令仍然不可用，请重启 VS Code" -ForegroundColor Cyan
Write-Host "  - 重启 VS Code 是最可靠的解决方法" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

