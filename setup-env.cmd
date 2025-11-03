@echo off
REM 设置开发环境脚本 (CMD 版本)
REM 自动添加 Rust 和 Node.js 路径到当前会话的 PATH

echo 正在配置开发环境...

REM 添加 Rust/Cargo 路径
set CARGO_PATH=%USERPROFILE%\.cargo\bin
if exist "%CARGO_PATH%" (
    echo | set /p="%PATH%" | findstr /C:"%CARGO_PATH%" >nul
    if errorlevel 1 (
        set "PATH=%CARGO_PATH%;%PATH%"
        echo [OK] 已添加 Cargo 路径
    ) else (
        echo [OK] Cargo 路径已存在
    )
) else (
    echo [警告] 未找到 Cargo 路径
)

REM 添加 Node.js/npm 路径（fnm 安装的版本）
set NODE_PATH=%LOCALAPPDATA%\Microsoft\WinGet\Packages\Schniz.fnm_Microsoft.Winget.Source_8wekyb3d8bbwe\node-versions\v22.12.0\installation
if exist "%NODE_PATH%\npm.cmd" (
    echo | set /p="%PATH%" | findstr /C:"%NODE_PATH%" >nul
    if errorlevel 1 (
        set "PATH=%NODE_PATH%;%PATH%"
        echo [OK] 已添加 Node.js 路径
    ) else (
        echo [OK] Node.js 路径已存在
    )
) else (
    echo [警告] 未找到 Node.js/npm 路径
)

REM 验证工具
echo.
echo 验证工具可用性:
cargo --version >nul 2>&1
if %errorlevel% equ 0 (
    echo [OK] Cargo 可用
) else (
    echo [错误] Cargo 不可用
)

npm --version >nul 2>&1
if %errorlevel% equ 0 (
    echo [OK] npm 可用
) else (
    echo [错误] npm 不可用
)

echo.
echo 环境配置完成！现在可以运行 'npm run tauri dev' 了
echo.

