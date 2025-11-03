# Setup Development Environment Script
# Auto-add Rust and Node.js paths to current session PATH

Write-Host "Configuring development environment..." -ForegroundColor Cyan

# Add Rust/Cargo path
$cargoPath = "$env:USERPROFILE\.cargo\bin"
if (Test-Path $cargoPath) {
    if ($env:PATH -notlike "*$cargoPath*") {
        $env:PATH = "$cargoPath;$env:PATH"
        Write-Host "[OK] Added Cargo path: $cargoPath" -ForegroundColor Green
    } else {
        Write-Host "[OK] Cargo path already exists" -ForegroundColor Green
    }
} else {
    Write-Host "[WARNING] Cargo path not found: $cargoPath" -ForegroundColor Yellow
}

# Add Node.js/npm paths (fnm installed version)
$nodePaths = @(
    "$env:LOCALAPPDATA\Microsoft\WinGet\Packages\Schniz.fnm_Microsoft.Winget.Source_8wekyb3d8bbwe\node-versions\v22.12.0\installation",
    "$env:ProgramFiles\nodejs",
    "$env:ProgramFiles(x86)\nodejs",
    "$env:APPDATA\npm"
)

$nodePathAdded = $false
foreach ($nodePath in $nodePaths) {
    if (Test-Path $nodePath) {
        if ($env:PATH -notlike "*$nodePath*") {
            $env:PATH = "$nodePath;$env:PATH"
            Write-Host "[OK] Added Node.js path: $nodePath" -ForegroundColor Green
            $nodePathAdded = $true
            break
        } else {
            Write-Host "[OK] Node.js path already exists: $nodePath" -ForegroundColor Green
            $nodePathAdded = $true
            break
        }
    }
}

if (-not $nodePathAdded) {
    # Try to find and activate fnm
    $fnmPaths = @(
        "$env:USERPROFILE\.fnm",
        "$env:LOCALAPPDATA\fnm"
    )
    
    foreach ($fnmPath in $fnmPaths) {
        $fnmEnvFile = Join-Path $fnmPath "multishells\current\env.ps1"
        if (Test-Path $fnmEnvFile) {
            . $fnmEnvFile
            Write-Host "[OK] Activated fnm environment" -ForegroundColor Green
            $nodePathAdded = $true
            break
        }
    }
}

if (-not $nodePathAdded) {
    Write-Host "[WARNING] Node.js/npm path not found" -ForegroundColor Yellow
}

# Verify tools availability
Write-Host ""
Write-Host "Verifying tools:" -ForegroundColor Cyan
try {
    $cargoVersion = & cargo --version 2>&1
    Write-Host "[OK] Cargo: $cargoVersion" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Cargo not available" -ForegroundColor Red
}

try {
    $npmVersion = & npm --version 2>&1
    Write-Host "[OK] npm: $npmVersion" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] npm not available" -ForegroundColor Red
}

Write-Host ""
Write-Host "Environment setup complete! You can now run 'npm run tauri dev'" -ForegroundColor Cyan
