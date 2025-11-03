# PowerShell 脚本：在构建后嵌入 Windows manifest（请求管理员权限）
# 使用方法：在构建完成后运行此脚本
# ./embed_manifest.ps1 -ExePath "target/release/easy-printer.exe"

param(
    [string]$ExePath = ""
)

# 获取脚本所在目录（src-tauri 目录）
$ScriptDir = if ($MyInvocation.MyCommand.Path) {
    Split-Path -Parent $MyInvocation.MyCommand.Path
} else {
    Split-Path -Parent $PSCommandPath
}

# 如果当前目录不是 src-tauri，则切换到脚本目录
$CurrentTargetPath = Join-Path $PWD "target"
$ScriptTargetPath = Join-Path $ScriptDir "target"

if (-not (Test-Path $CurrentTargetPath)) {
    if (Test-Path $ScriptTargetPath) {
        Set-Location $ScriptDir
        Write-Host "已切换到脚本目录: $ScriptDir" -ForegroundColor Cyan
    } else {
        Write-Warning "当前目录和脚本目录都找不到 target 目录"
        Write-Warning "当前目录: $PWD"
        Write-Warning "脚本目录: $ScriptDir"
    }
}

if ([string]::IsNullOrEmpty($ExePath)) {
    # 如果没有指定路径，尝试自动查找
    # 首先尝试 Release 目录
    $ReleaseDir = Join-Path $PWD "target\release"
    $DebugDir = Join-Path $PWD "target\debug"
    
    # 查找 Release 目录中的 exe 文件（Tauri 生成的文件名可能不同）
    if (Test-Path $ReleaseDir) {
        Write-Host "正在搜索 Release 目录: $ReleaseDir" -ForegroundColor Cyan
        $exeFiles = Get-ChildItem -Path $ReleaseDir -Filter "*.exe" -File -ErrorAction SilentlyContinue | Where-Object { 
            $_.Name -notlike "*deps*" -and 
            $_.Name -notlike "*build*" -and
            $_.DirectoryName -notlike "*\deps\*" -and
            $_.DirectoryName -notlike "*\build\*"
        }
        
        if ($exeFiles) {
            Write-Host "找到 $($exeFiles.Count) 个 exe 文件" -ForegroundColor Gray
            # 优先选择主程序 exe（不是依赖项，通常在 release 根目录）
            $mainExe = $exeFiles | Where-Object { 
                $_.DirectoryName -eq $ReleaseDir -and
                $_.Name -notmatch "^[a-f0-9]{16}"
            } | Select-Object -First 1
            
            if (-not $mainExe) {
                # 如果根目录没找到，尝试在所有文件中找
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
            Write-Host "Release 目录中未找到符合条件的 exe 文件" -ForegroundColor Gray
        }
    }
    
    # 如果 Release 未找到，尝试 Debug 目录
    if ([string]::IsNullOrEmpty($ExePath) -and (Test-Path $DebugDir)) {
        Write-Host "正在搜索 Debug 目录: $DebugDir" -ForegroundColor Cyan
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
            Write-Host "Debug 目录中未找到符合条件的 exe 文件" -ForegroundColor Gray
        }
    }
    
    if ([string]::IsNullOrEmpty($ExePath)) {
        Write-Host "未找到 exe 文件，请手动指定路径。例如：" -ForegroundColor Red
        Write-Host "  .\embed_manifest.ps1 -ExePath `"target\release\易点云打印机安装小精灵.exe`"" -ForegroundColor Yellow
        Write-Host "  .\embed_manifest.ps1 -ExePath `"target\debug\易点云打印机安装小精灵.exe`"" -ForegroundColor Yellow
        exit 1
    }
}

# 如果 ExePath 是相对路径，确保基于当前目录
if (-not [System.IO.Path]::IsPathRooted($ExePath)) {
    $ExePath = Join-Path $PWD $ExePath
}

if (-not (Test-Path $ExePath)) {
    Write-Error "指定的 exe 文件不存在: $ExePath"
    exit 1
}

# Manifest 路径也基于当前目录
$ManifestPath = "app.manifest"
# 如果当前目录没有，尝试脚本目录
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

Write-Host "正在嵌入 manifest 到: $ExePath"
Write-Host "使用 manifest: $ManifestPath"

# 检查是否有 mt.exe（Windows SDK 工具）
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
    Write-Warning "请手动安装 Windows SDK，或使用 Resource Hacker 等工具"
    Write-Warning "或者右键点击 exe -> 属性 -> 兼容性 -> 以管理员身份运行此程序"
    exit 1
}

# 使用 mt.exe 嵌入 manifest
try {
    $result = & $MtPath -manifest $ManifestPath -outputresource:"$ExePath;1" 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ 成功嵌入 manifest！" -ForegroundColor Green
        Write-Host "现在应用将以管理员权限运行"
    } else {
        Write-Error "嵌入 manifest 失败: $result"
        exit 1
    }
} catch {
    Write-Error "执行 mt.exe 时出错: $_"
    exit 1
}

