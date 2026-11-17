# Builds the Android APK (arm64) and optionally installs it on the phone connected over USB.
#
#   powershell -ExecutionPolicy Bypass -File scripts\android-build.ps1            # build
#   powershell -ExecutionPolicy Bypass -File scripts\android-build.ps1 -Install   # build and install
#
# The Rust engine is always built optimised: an unoptimised FLAC decoder and resampler can stutter on
# a phone. The APK itself is the debug variant (signed with the local debug key), so it installs
# without a release keystore.
#
# Tauri links the Rust library into the Gradle project with a symbolic link, which Windows only
# allows in Developer Mode. Without it this script copies the library instead.

param([switch]$Install)
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$tools = if ($env:LUMEN_ANDROID_TOOLS) { $env:LUMEN_ANDROID_TOOLS } else { Join-Path $env:USERPROFILE 'Android' }
$env:JAVA_HOME = Join-Path $tools 'jdk17'
$env:ANDROID_HOME = Join-Path $tools 'Sdk'
$ndk = Get-ChildItem (Join-Path $env:ANDROID_HOME 'ndk') | Sort-Object Name | Select-Object -Last 1
$env:NDK_HOME = $ndk.FullName
# Java's internal pipes use a socket file in the temp folder; keep that path short and real.
$tmp = Join-Path $tools 'tmp'
New-Item -ItemType Directory -Force $tmp | Out-Null
$env:JAVA_TOOL_OPTIONS = "-Djdk.net.unixdomain.tmpdir=$tmp"

$android = Join-Path $root 'src-tauri\gen\android'
$so = Join-Path $root 'src-tauri\target\aarch64-linux-android\release\liblumen_lib.so'
$jni = Join-Path $android 'app\src\main\jniLibs\arm64-v8a'

Push-Location $root
try {
    # Builds the interface and the Rust library; with Developer Mode it also finishes the APK.
    # Its own APK step may fail on the symbolic link; that is handled below.
    $ErrorActionPreference = 'Continue'
    & npm run tauri android build -- --apk --target aarch64
    $ErrorActionPreference = 'Stop'
} finally {
    Pop-Location
}
if (-not (Test-Path $so)) { throw 'Rust library was not built, see the output above.' }

# Always produce the debug-signed APK from the optimised library.
New-Item -ItemType Directory -Force $jni | Out-Null
Remove-Item (Join-Path $jni 'liblumen_lib.so') -Force -ErrorAction SilentlyContinue
Copy-Item $so $jni
Push-Location $android
try {
    & (Join-Path $android 'gradlew.bat') assembleArm64Debug -x rustBuildArm64Debug --console=plain
    if ($LASTEXITCODE -ne 0) { throw 'Gradle build failed.' }
} finally {
    Pop-Location
}

$apk = Get-ChildItem (Join-Path $android 'app\build\outputs\apk') -Recurse -Filter '*arm64*debug*.apk' | Select-Object -First 1
$out = Join-Path $root 'SoSamb0-Music-arm64.apk'
Copy-Item $apk.FullName $out -Force
Write-Host "APK: $out ($([math]::Round((Get-Item $out).Length / 1MB, 1)) MB)"

if ($Install) {
    & (Join-Path $env:ANDROID_HOME 'platform-tools\adb.exe') install -r $out
}
