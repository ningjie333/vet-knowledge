# vet-knowledge 构建脚本：自动设置 Windows SUBSYSTEM
# 解决 Tauri exe 启动时出现空白终端的问题
param(
    [switch]$SkipBundle
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$tauriDir = Join-Path $projectRoot "src-tauri"
$exePath = Join-Path $tauriDir "target\release\vet-knowledge.exe"
$nsisScript = Join-Path $tauriDir "target\release\nsis\x64\installer.nsi"
$nsisOutput = Join-Path $tauriDir "target\release\nsis\x64\nsis-output.exe"
$bundleDir = Join-Path $tauriDir "target\release\bundle\nsis"
$installerFinal = Join-Path $bundleDir "兽医知识库_0.1.0_x64-setup.exe"
$editbin = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.50.35717\bin\Hostx64\x64\editbin.exe"
$makensis = Join-Path $env:LOCALAPPDATA "tauri\NSIS\makensis.exe"

# 1. 构建（跳过 bundle）
Write-Host "==> Building release..."
npm run tauri:build -- --no-bundle --config "$tauriDir\tauri.conf.json"
if ($LASTEXITCODE -ne 0) { exit 1 }

# 2. 应用 SUBSYSTEM:WINDOWS
Write-Host "==> Patching subsystem to WINDOWS..."
& $editbin /SUBSYSTEM:WINDOWS $exePath

# 3. 复制到 bundle 目录
Write-Host "==> Copying patched exe to bundle dir..."
Copy-Item $exePath (Join-Path $bundleDir "vet-knowledge.exe") -Force

if (-not $SkipBundle) {
    # 4. 重新打包 NSIS
    Write-Host "==> Running makensis..."
    & $makensis -INPUTCHARSET UTF8 -OUTPUTCHARSET UTF8 -V3 $nsisScript
    if ($LASTEXITCODE -ne 0) { exit 1 }

    # 5. 移到最终位置
    Move-Item $nsisOutput $installerFinal -Force
    Write-Host "==> Installer ready: $installerFinal"
} else {
    Write-Host "==> Bundling skipped. Patched exe: $exePath"
}

Write-Host "Done."
