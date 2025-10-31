# 修复 Rust PATH 问题
# 这个脚本会将 Rust 添加到当前会话的 PATH 中，并检查系统 PATH

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "   修复 Rust PATH 问题" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$rustPath = "$env:USERPROFILE\.cargo\bin"

# 检查 Rust 是否安装
Write-Host "检查 Rust 安装..." -ForegroundColor Yellow
if (Test-Path "$rustPath\rustc.exe") {
    Write-Host "✅ 找到 Rust 安装: $rustPath" -ForegroundColor Green
    
    # 检查 Rust 版本
    try {
        $rustVersion = & "$rustPath\rustc.exe" --version
        Write-Host "✅ Rust 版本: $rustVersion" -ForegroundColor Green
    } catch {
        Write-Host "❌ 无法运行 rustc.exe" -ForegroundColor Red
    }
} else {
    Write-Host "❌ 未找到 Rust 安装" -ForegroundColor Red
    Write-Host "请先安装 Rust: https://rustup.rs/" -ForegroundColor Yellow
    exit
}

Write-Host ""

# 检查系统 PATH
Write-Host "检查系统 PATH..." -ForegroundColor Yellow
$systemPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
if ($systemPath -like "*$rustPath*") {
    Write-Host "✅ Rust 已在用户 PATH 中" -ForegroundColor Green
} else {
    Write-Host "⚠️  Rust 不在用户 PATH 中" -ForegroundColor Yellow
    Write-Host "尝试添加到用户 PATH..." -ForegroundColor Yellow
    
    try {
        $currentUserPath = [System.Environment]::GetEnvironmentVariable("Path", "User")
        if ($currentUserPath -notlike "*$rustPath*") {
            [System.Environment]::SetEnvironmentVariable(
                "Path",
                "$currentUserPath;$rustPath",
                [System.EnvironmentVariableTarget]::User
            )
            Write-Host "✅ 已添加到用户 PATH" -ForegroundColor Green
            Write-Host "⚠️  需要重新打开终端或重启 VS Code 才能生效" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "❌ 无法修改系统 PATH（可能需要管理员权限）" -ForegroundColor Red
    }
}

Write-Host ""

# 添加到当前会话的 PATH
Write-Host "添加到当前会话 PATH..." -ForegroundColor Yellow
if ($env:Path -notlike "*$rustPath*") {
    $env:Path += ";$rustPath"
    Write-Host "✅ 已添加到当前会话 PATH" -ForegroundColor Green
} else {
    Write-Host "✅ 已在当前会话 PATH 中" -ForegroundColor Green
}

Write-Host ""

# 测试命令
Write-Host "测试命令..." -ForegroundColor Yellow
try {
    $rustVersion = rustc --version
    Write-Host "✅ rustc 可用: $rustVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ rustc 仍然不可用" -ForegroundColor Red
}

try {
    $cargoVersion = cargo --version
    Write-Host "✅ cargo 可用: $cargoVersion" -ForegroundColor Green
} catch {
    Write-Host "❌ cargo 仍然不可用" -ForegroundColor Red
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "完成！" -ForegroundColor Cyan
Write-Host ""
Write-Host "提示：" -ForegroundColor Yellow
Write-Host "  - 当前会话中 Rust 现在应该可用了" -ForegroundColor Cyan
Write-Host "  - 如果已添加到系统 PATH，需要重启 VS Code 才能永久生效" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

