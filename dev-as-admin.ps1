# Run Tauri dev mode as Administrator
# Usage: Right-click this file -> "Run with PowerShell"

# Check if running as Administrator
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "Not running as Administrator, requesting elevation..." -ForegroundColor Yellow
    # Use Start-Process to re-run script as Administrator
    $scriptPath = $MyInvocation.MyCommand.Path
    Start-Process powershell -Verb RunAs -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$scriptPath`""
    exit
}

Write-Host "[OK] Running as Administrator" -ForegroundColor Green
Write-Host "Starting Tauri dev mode..." -ForegroundColor Cyan
Write-Host ""

# Change to project directory
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

# Run npm run tauri dev
npm run tauri dev

