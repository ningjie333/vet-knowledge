@echo off
chcp 65001 >nul
title 兽医知识库

set "PATH=%USERPROFILE%\.cargo/bin;%PATH%"
set "NPM_CONFIG_CACHE=%~dp0\.npm-cache"
cd /d "%~dp0src-tauri"

echo 正在启动兽医知识库...
npx tauri dev
