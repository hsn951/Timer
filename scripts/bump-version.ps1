param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Version
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

function Update-File($path, $pattern, $replacement) {
    $text = Get-Content $path -Raw -Encoding UTF8
    $next = [regex]::Replace($text, $pattern, $replacement)
    if ($next -eq $text) {
        Write-Warning "未在 $path 找到可替换的版本号"
    }
    Set-Content -Path $path -Value $next -Encoding UTF8 -NoNewline
    Write-Host "updated $path -> $Version"
}

Update-File "package.json" '"version":\s*"[^"]+"' ('"version": "' + $Version + '"')
Update-File "src-tauri/Cargo.toml" '(?m)^version = "[^"]+"' ('version = "' + $Version + '"')
Update-File "src-tauri/tauri.conf.json" '"version":\s*"[^"]+"' ('"version": "' + $Version + '"')

Write-Host ""
Write-Host "版本号已改为 $Version。别忘了：" -ForegroundColor Yellow
Write-Host "  1. 更新 CHANGELOG.md（把 Unreleased 改成 $Version）"
Write-Host "  2. git commit -am `"chore(release): v$Version`""
Write-Host "  3. git tag v$Version && git push origin main --tags"
