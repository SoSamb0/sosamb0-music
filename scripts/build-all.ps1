# Builds both versions from the same code and puts them side by side in release\<version>\:
# the Windows installer, the Windows program itself and the Android APK.
#
#   powershell -ExecutionPolicy Bypass -File scripts\build-all.ps1              # check, then build both
#   powershell -ExecutionPolicy Bypass -File scripts\build-all.ps1 -Install     # also install the APK on the phone
#   powershell -ExecutionPolicy Bypass -File scripts\build-all.ps1 -SkipChecks  # skip type check and engine tests
#   powershell -ExecutionPolicy Bypass -File scripts\build-all.ps1 -Only pc     # or -Only android
#
# The version comes from src-tauri\tauri.conf.json; Tauri gives the APK the same one.

param(
    [switch]$Install,
    [switch]$SkipChecks,
    [ValidateSet('both', 'pc', 'android')][string]$Only = 'both'
)
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$version = (Get-Content (Join-Path $root 'src-tauri\tauri.conf.json') -Raw | ConvertFrom-Json).version
$out = Join-Path $root "release\$version"
New-Item -ItemType Directory -Force $out | Out-Null
$started = Get-Date

function Step($text) { Write-Host "`n=== $text ===" -ForegroundColor Cyan }
function Run($what, [scriptblock]$block) {
    & $block
    if ($LASTEXITCODE -ne 0) { throw "$what failed (exit code $LASTEXITCODE), see the output above." }
}

Write-Host "SoSamb0 Music $version -> $out"

Push-Location $root
try {
    if (-not $SkipChecks) {
        Step 'Type check'
        Run 'npm run check' { npm run check }
        Step 'Audio engine tests'
        Push-Location (Join-Path $root 'src-tauri')
        try { Run 'cargo test' { cargo test --lib } } finally { Pop-Location }
    }

    if ($Only -ne 'android') {
        Step 'Windows'
        # A running copy keeps the exe locked.
        Get-Process 'SoSamb0 Music' -ErrorAction SilentlyContinue | Stop-Process -Force
        Run 'Windows build' { npm run tauri build }
        $release = Join-Path $root 'src-tauri\target\release'
        $setup = Get-ChildItem (Join-Path $release 'bundle\nsis') -Filter "*$version*setup.exe" | Select-Object -First 1
        if (-not $setup) { throw 'Windows installer not found in src-tauri\target\release\bundle\nsis.' }
        Copy-Item $setup.FullName (Join-Path $out "SoSamb0-Music-$version-setup.exe") -Force
        Copy-Item (Join-Path $release 'SoSamb0 Music.exe') (Join-Path $out "SoSamb0-Music-$version-portable.exe") -Force
    }

    if ($Only -ne 'pc') {
        Step 'Android'
        $androidArgs = @('-ExecutionPolicy', 'Bypass', '-File', (Join-Path $PSScriptRoot 'android-build.ps1'))
        if ($Install) { $androidArgs += '-Install' }
        Run 'Android build' { & powershell.exe @androidArgs }
        Copy-Item (Join-Path $root 'SoSamb0-Music-arm64.apk') (Join-Path $out "SoSamb0-Music-$version-arm64.apk") -Force
    }
} finally {
    Pop-Location
}

Step 'Done'
Get-ChildItem $out | Where-Object { $_.LastWriteTime -ge $started } | ForEach-Object {
    Write-Host ("{0,-40} {1,6:N1} MB" -f $_.Name, ($_.Length / 1MB))
}
Write-Host "Folder: $out"
