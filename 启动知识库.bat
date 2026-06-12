@echo off
chcp 65001 >nul 2>&1
title 兽医知识库 - 启动中

cd /d "%~dp0"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "NPM_CONFIG_CACHE=%~dp0\.npm-cache"

:: 关闭可能残留的旧进程
taskkill /IM vet-knowledge.exe /F >nul 2>&1
taskkill /IM cargo.exe /F >nul 2>&1

:: 清理被锁定的旧 exe
if exist "src-tauri\target\debug\vet-knowledge.exe" (
    del /f "src-tauri\target\debug\vet-knowledge.exe" >nul 2>&1
)

echo ========================================
echo   兽医知识库 - 开发模式启动
echo ========================================
echo.
echo [1/3] 正在编译 Rust 后端...
echo （首次编译约需 20-30 秒，请稍候）
echo.

npm run tauri:dev

exit
