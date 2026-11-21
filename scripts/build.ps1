# 构建前端 + Rust（Release）
$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Find-Node {
    $cmd = Get-Command node -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    if ($env:DSH_NODE -and (Test-Path $env:DSH_NODE)) { return $env:DSH_NODE }
    $fallback = "C:\Users\Admin\.dsh\dsh-runtimes\dsh-primary-runtime\dependencies\node\bin\node.exe"
    if (Test-Path $fallback) { return $fallback }
    throw "找不到 node，请先安装 Node.js 20+"
}

$node = Find-Node
Write-Host "使用 node: $node"

if (-not (Test-Path "$root\node_modules\vite")) {
    Write-Host "安装前端依赖..."
    & $node "$root\node_modules\pnpm\bin\pnpm.cjs" install 2>$null
    if (-not (Test-Path "$root\node_modules\vite")) {
        $pnpm = Get-Command pnpm -ErrorAction SilentlyContinue
        if ($pnpm) { & pnpm install } else { throw "请先执行 pnpm install / npm install" }
    }
}

Write-Host "构建前端..."
& $node "$root\node_modules\vite\bin\vite.js" build
if ($LASTEXITCODE -ne 0) { throw "前端构建失败" }

$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { $cargo = (Get-Command cargo).Source }
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH

Write-Host "编译 Rust（首次约 5-15 分钟）..."
& $cargo build --release --manifest-path "$root\src-tauri\Cargo.toml"
if ($LASTEXITCODE -ne 0) { throw "Rust 编译失败" }

$exe = "$root\src-tauri\target\release\macaron-timer.exe"
if (Test-Path $exe) {
    $mb = [math]::Round((Get-Item $exe).Length / 1MB, 2)
    Write-Host "完成：$exe（$mb MB）" -ForegroundColor Green
} else {
    throw "没有找到产物 $exe"
}
