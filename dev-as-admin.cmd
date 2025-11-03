@echo off
:: Run Tauri dev mode as Administrator
:: Usage: Right-click this file -> "Run as administrator"

:: Check if running as Administrator
net session >nul 2>&1
if %errorLevel% == 0 (
    echo [OK] Running as Administrator
    goto :run
)

echo Not running as Administrator, requesting elevation...
:: Use PowerShell to elevate
powershell -Command "Start-Process '%~f0' -Verb RunAs"
exit /b

:run
echo Starting Tauri dev mode...
echo.

cd /d "%~dp0"
call npm run tauri dev

pause

