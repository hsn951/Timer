# 启动已编译的程序
$root = Split-Path -Parent $PSScriptRoot
$exe = Join-Path $root "src-tauri\target\release\macaron-timer.exe"
if (-not (Test-Path $exe)) {
    Write-Host "还没有编译，先运行 scripts\build.ps1" -ForegroundColor Yellow
    exit 1
}
Start-Process -FilePath $exe
Write-Host "已启动 $exe"
