# Fast dev loop: starts Tauri dev window with Vite HMR.
# Frontend edits reload instantly; only Rust changes trigger a debug rebuild.
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

$node = (Get-Command node -ErrorAction SilentlyContinue).Source
if (-not $node) { $node = "C:\Users\Admin\.dsh\dsh-runtimes\dsh-primary-runtime\dependencies\node\bin\node.exe" }
$env:PATH = "C:\Users\Admin\.dsh\dsh-runtimes\dsh-primary-runtime\dependencies\node\bin;" + $env:PATH

& $node "$root\node_modules\@tauri-apps\cli\tauri.js" dev
