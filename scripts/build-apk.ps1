# Builds the arm64-v8a debug APK of the painhubproxy Android shell.
#
# The Rust part is compiled with the NDK clang directly: the Gradle cargoNdk
# task cannot start the NDK's .cmd wrappers on Windows, so the shared library
# is built here and the Gradle task is skipped.
#
# Requirements are taken from the parameters below or, when a parameter is
# omitted, from the standard environment variables (JAVA_HOME, ANDROID_HOME).
#
#   .\scripts\build-apk.ps1
#   .\scripts\build-apk.ps1 -Jdk C:\jdk-21 -AndroidSdk C:\android-sdk
#
# On Windows the host build scripts are compiled with the GNU toolchain, so
# MinGW must be available in PATH (or passed with -MinGwBin). On Linux/macOS
# the default host toolchain works and -MinGwBin is ignored.
param(
    [string]$Abi = 'arm64-v8a',
    [string]$NdkVersion = '28.1.13356709',
    [string]$Jdk = $env:JAVA_HOME,
    [string]$AndroidSdk = $env:ANDROID_HOME,
    [string]$MinGwBin = '',
    [string]$RustToolchain = ''
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$android = Join-Path $root 'android'
$jniLibs = Join-Path $android "app\build\generated\rustJniLibs\$Abi"

if (-not $Jdk) { throw 'JDK not specified: pass -Jdk or set JAVA_HOME' }
if (-not $AndroidSdk) { throw 'Android SDK not specified: pass -AndroidSdk or set ANDROID_HOME' }
if (-not (Test-Path $Jdk)) { throw "JDK not found: $Jdk" }
if (-not (Test-Path $AndroidSdk)) { throw "Android SDK not found: $AndroidSdk" }

$ndkBin = Join-Path $AndroidSdk "ndk\$NdkVersion\toolchains\llvm\prebuilt\windows-x86_64\bin"
if (-not (Test-Path $ndkBin)) {
    # Linux/macOS SDK layout
    $ndkBin = Join-Path $AndroidSdk "ndk\$NdkVersion\toolchains\llvm\prebuilt\linux-x86_64\bin"
}
if (-not (Test-Path $ndkBin)) { throw "NDK toolchain not found for $NdkVersion under $AndroidSdk" }

$clang = Join-Path $ndkBin 'clang'
if (Test-Path "$clang.exe") { $clang = "$clang.exe" }
$ar = Join-Path $ndkBin 'llvm-ar'
if (Test-Path "$ar.exe") { $ar = "$ar.exe" }

$env:JAVA_HOME = $Jdk
$env:ANDROID_HOME = $AndroidSdk
if ($RustToolchain) { $env:RUSTUP_TOOLCHAIN = $RustToolchain }
if ($MinGwBin) { $env:PATH = "$ndkBin;$MinGwBin;$env:PATH" } else { $env:PATH = "$ndkBin;$env:PATH" }

$env:CC_aarch64_linux_android = $clang
$env:CXX_aarch64_linux_android = (Join-Path $ndkBin 'clang++')
$env:AR_aarch64_linux_android = $ar
$env:CFLAGS_aarch64_linux_android = '--target=aarch64-linux-android26'
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER = $clang
$env:CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS = '-C link-arg=--target=aarch64-linux-android26'

# cargo and gradle write progress to stderr; with 'Stop' PowerShell would turn
# that into a terminating error, so the exit code is checked instead.
function Invoke-Native([scriptblock]$Call, [string]$What) {
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $Call } finally { $ErrorActionPreference = $prev }
    if ($LASTEXITCODE -ne 0) { throw "$What failed ($LASTEXITCODE)" }
}

Push-Location $root
try {
    Invoke-Native { cargo build -p tg-ws-proxy-jni --lib --target aarch64-linux-android --release --locked } 'cargo build'

    New-Item -ItemType Directory -Force -Path $jniLibs | Out-Null
    Copy-Item (Join-Path $root 'target\aarch64-linux-android\release\libtg_ws_proxy_jni.so') $jniLibs -Force
}
finally {
    Pop-Location
}

Push-Location $android
try {
    $abiArg = "-PTG_ANDROID_ABIS=$Abi"
    Invoke-Native { & .\gradlew.bat :app:assembleDebug $abiArg -x :app:cargoNdk --console=plain } 'gradle assembleDebug'
}
finally {
    Pop-Location
}

$apk = Join-Path $android "app\build\outputs\apk\debug\app-$Abi-debug.apk"
$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$out = Join-Path $dist "app-$Abi-debug.apk"
Copy-Item $apk $out -Force

$hash = (Get-FileHash $out -Algorithm SHA256).Hash
$size = (Get-Item $out).Length
"APK:    $out"
"Size:   $size bytes"
"SHA256: $hash"
