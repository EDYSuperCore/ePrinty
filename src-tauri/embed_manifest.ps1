# PowerShell 脚本用于嵌入 Windows manifest 到已编译的 exe 文件
# 用法示例（可选指定 exe 路径）：
# ./embed_manifest.ps1 -ExePath "target/release/easy-printer.exe"

param(
    [string]$ExePath = ""
)

# 获取脚本所在的 src-tauri 目录
$ScriptDir = if ($MyInvocation.MyCommand.Path) {
    Split-Path -Parent $MyInvocation.MyCommand.Path
} else {
    Split-Path -Parent $PSCommandPath
}

# 确保在正确的 src-tauri 目录中（检查是否有 target 目录）
$CurrentTargetPath = Join-Path $PWD "target"
$ScriptTargetPath = Join-Path $ScriptDir "target"

if (-not (Test-Path $CurrentTargetPath)) {
    if (Test-Path $ScriptTargetPath) {
        Set-Location $ScriptDir
        Write-Host "已切换到脚本目录: $ScriptDir" -ForegroundColor Cyan
    } else {
        Write-Warning "当前目录和脚本目录都没有 target 目录"
        Write-Warning "当前目录: $PWD"
        Write-Warning "脚本目录: $ScriptDir"
    }
}

if ([string]::IsNullOrEmpty($ExePath)) {
    # 如果未指定路径，自动查找构建的 exe
    # 优先查找 Release 版本
    $ReleaseDir = Join-Path $PWD "target\release"
    $DebugDir = Join-Path $PWD "target\debug"
    
    # 优先 Release 版本的 exe（因为 Tauri 默认构建到这里）
    if (Test-Path $ReleaseDir) {
        Write-Host "正在查找 Release 版本: $ReleaseDir" -ForegroundColor Cyan
        $exeFiles = Get-ChildItem -Path $ReleaseDir -Filter "*.exe" -File -ErrorAction SilentlyContinue | Where-Object { 
            $_.Name -notlike "*deps*" -and 
            $_.Name -notlike "*build*" -and
            $_.DirectoryName -notlike "*\deps\*" -and
            $_.DirectoryName -notlike "*\build\*"
        }
        
        if ($exeFiles) {
            Write-Host "找到 $($exeFiles.Count) 个 exe 文件" -ForegroundColor Gray
            # 优先选择在主 exe（排除编译依赖的临时 release 产物）
            $mainExe = $exeFiles | Where-Object { 
                $_.DirectoryName -eq $ReleaseDir -and
                $_.Name -notmatch "^[a-f0-9]{16}"
            } | Select-Object -First 1
            
            if (-not $mainExe) {
                # 如果没找到，放宽条件重试
                $mainExe = $exeFiles | Where-Object { 
                    $_.Name -notmatch "^[a-f0-9]{16}"
                } | Select-Object -First 1
            }
            
            if (-not $mainExe) {
                $mainExe = $exeFiles | Select-Object -First 1
            }
            
            if ($mainExe) {
                $ExePath = $mainExe.FullName
                Write-Host "找到 Release 版本: $ExePath" -ForegroundColor Green
            }
        } else {
            Write-Host "Release 目录中没有找到符合条件的 exe 文件" -ForegroundColor Gray
        }
    }
    
    # 如果 Release 找不到，尝试 Debug 版本
    if ([string]::IsNullOrEmpty($ExePath) -and (Test-Path $DebugDir)) {
        Write-Host "正在查找 Debug 版本: $DebugDir" -ForegroundColor Cyan
        $exeFiles = Get-ChildItem -Path $DebugDir -Filter "*.exe" -File -ErrorAction SilentlyContinue | Where-Object { 
            $_.Name -notlike "*deps*" -and 
            $_.Name -notlike "*build*" -and
            $_.DirectoryName -notlike "*\deps\*" -and
            $_.DirectoryName -notlike "*\build\*"
        }
        
        if ($exeFiles) {
            Write-Host "找到 $($exeFiles.Count) 个 exe 文件" -ForegroundColor Gray
            $mainExe = $exeFiles | Where-Object { 
                $_.DirectoryName -eq $DebugDir -and
                $_.Name -notmatch "^[a-f0-9]{16}"
            } | Select-Object -First 1
            
            if (-not $mainExe) {
                $mainExe = $exeFiles | Where-Object { 
                    $_.Name -notmatch "^[a-f0-9]{16}"
                } | Select-Object -First 1
            }
            
            if (-not $mainExe) {
                $mainExe = $exeFiles | Select-Object -First 1
            }
            
            if ($mainExe) {
                $ExePath = $mainExe.FullName
                Write-Host "找到 Debug 版本: $ExePath" -ForegroundColor Yellow
            }
        } else {
            Write-Host "Debug 目录中没有找到符合条件的 exe 文件" -ForegroundColor Gray
        }
    }
    
    if ([string]::IsNullOrEmpty($ExePath)) {
        Write-Host "未找到 exe 文件，请手动指定路径或先构建项目：" -ForegroundColor Red
        Write-Host "  .\embed_manifest.ps1 -ExePath `"target\release\ePrinty.exe`"" -ForegroundColor Yellow
        Write-Host "  .\embed_manifest.ps1 -ExePath `"target\debug\ePrinty.exe`"" -ForegroundColor Yellow
        exit 1
    }
}

# 如果 ExePath 是相对路径，转换为绝对路径
if (-not [System.IO.Path]::IsPathRooted($ExePath)) {
    $ExePath = Join-Path $PWD $ExePath
}

if (-not (Test-Path $ExePath)) {
    Write-Error "指定的 exe 文件不存在: $ExePath"
    exit 1
}

# 检查文件是否被占用（是否可以写入）
try {
    $fileStream = [System.IO.File]::Open($ExePath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Write, [System.IO.FileShare]::None)
    $fileStream.Close()
    $fileStream.Dispose()
} catch {
    Write-Error "无法写入 exe 文件，可能文件正在使用中: $ExePath"
    Write-Error "请尝试："
    Write-Error "  1. ePrinty.exe 是否正在运行？请关闭"
    Write-Error "  2. 是否有杀毒软件占用？请暂时关闭"
    Write-Error "  3. 是否有管理员权限？以管理员身份运行 PowerShell"
    exit 1
}

# 检查是否有管理员权限
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Warning "当前未以管理员身份运行 PowerShell"
    Write-Warning "嵌入 manifest 操作可能会失败"
    Write-Warning "建议右键点击 PowerShell 并选择'以管理员身份运行'"
    $continue = Read-Host "是否继续？(Y/N)"
    if ($continue -ne "Y" -and $continue -ne "y") {
        Write-Host "已取消" -ForegroundColor Yellow
        exit 0
    }
}

# Manifest 文件路径（在当前目录）
$ManifestPath = "app.manifest"
# 如果不在当前目录，尝试脚本目录
if (-not (Test-Path $ManifestPath)) {
    $ScriptManifestPath = Join-Path $ScriptDir "app.manifest"
    if (Test-Path $ScriptManifestPath) {
        $ManifestPath = $ScriptManifestPath
    }
}
if (-not (Test-Path $ManifestPath)) {
    Write-Error "manifest 文件不存在: $ManifestPath"
    exit 1
}

Write-Host "准备嵌入 manifest 到: $ExePath"
Write-Host "使用 manifest: $ManifestPath"

# 查找并使用 mt.exe（Windows SDK 工具）
$MtPath = $null
$PossiblePaths = @(
    "C:\Program Files (x86)\Windows Kits\10\bin\10.0.*\x64\mt.exe",
    "C:\Program Files (x86)\Windows Kits\10\bin\x64\mt.exe",
    "C:\Program Files\Microsoft Visual Studio\*\SDK\Windows\v10.0A\bin\NETFX 4.8 Tools\x64\mt.exe"
)

foreach ($pattern in $PossiblePaths) {
    $found = Get-ChildItem -Path $pattern -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) {
        $MtPath = $found.FullName
        Write-Host "找到 mt.exe: $MtPath"
        break
    }
}

if ($null -eq $MtPath) {
    Write-Warning "未找到 mt.exe，无法自动嵌入 manifest"
    Write-Warning "请手动安装 Windows SDK 或使用 Resource Hacker 等工具"
    Write-Warning "手动嵌入方式：右键 exe -> 资源 -> 添加 -> 选择 app.manifest 作为资源类型 24 项 1"
    exit 1
}

# 使用 mt.exe 嵌入 manifest
Write-Host "正在嵌入 manifest..." -ForegroundColor Cyan
try {
    # 使用 -nologo 参数减少输出，2>&1 捕获所有输出
    $result = & $MtPath -nologo -manifest $ManifestPath -outputresource:"$ExePath;1" 2>&1
    $exitCode = $LASTEXITCODE
    
    if ($exitCode -eq 0) {
        Write-Host "? 成功嵌入 manifest！" -ForegroundColor Green
        Write-Host "现在应用将以管理员身份运行。" -ForegroundColor Green
    } else {
        Write-Error "嵌入 manifest 失败 (退出代码: $exitCode)"
        Write-Error "错误输出: $result"
        Write-Host ""
        Write-Host "可能的解决方案：" -ForegroundColor Yellow
        Write-Host "  1. 确保 ePrinty.exe 没有正在运行" -ForegroundColor Yellow
        Write-Host "  2. 关闭杀毒软件或将应用添加到白名单（杀毒软件可能会锁定文件）" -ForegroundColor Yellow
        Write-Host "  3. 以管理员身份运行 PowerShell（右键选择'以管理员身份运行'）" -ForegroundColor Yellow
        Write-Host "  4. 检查文件是否有只读属性（右键文件->属性->取消只读）" -ForegroundColor Yellow
        exit 1
    }
} catch {
    Write-Error "执行 mt.exe 时出错: $_"
    Write-Host ""
    Write-Host "可能的解决方案：" -ForegroundColor Yellow
    Write-Host "  1. 确保 ePrinty.exe 没有正在运行" -ForegroundColor Yellow
    Write-Host "  2. 以管理员身份运行 PowerShell" -ForegroundColor Yellow
    exit 1
}

