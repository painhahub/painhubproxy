# Builds the desktop (tray) binary of painhubproxy.
#
#   .\scripts\build-desktop.ps1
#   .\scripts\build-desktop.ps1 -MinGwBin C:\mingw64\bin -RustupHome C:\rustup
#
# On Windows the build needs a linker for the host toolchain: the MSVC one when
# the Rust host toolchain is *-pc-windows-msvc, MinGW when it is *-pc-windows-gnu.
# The script detects MinGW through `gcc` in PATH and leaves the Rust environment
# alone unless -RustToolchain / -RustupHome are given.
param(
    [string]$MinGwBin = '',
    [string]$RustToolchain = '',
    [string]$RustupHome = ''
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot

if ($RustupHome) { $env:RUSTUP_HOME = $RustupHome }
if ($RustToolchain) { $env:RUSTUP_TOOLCHAIN = $RustToolchain }

if ($MinGwBin) {
    $env:PATH = "$MinGwBin;$env:PATH"
} else {
    $gcc = Get-Command gcc -ErrorAction SilentlyContinue
    if ($gcc) { $env:PATH = "$(Split-Path $gcc.Source -Parent);$env:PATH" }
}

# cargo writes progress to stderr; with 'Stop' PowerShell turns that into a
# terminating error, so the exit code is checked instead.
function Invoke-Native([scriptblock]$Call, [string]$What) {
    $prev = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $Call } finally { $ErrorActionPreference = $prev }
    if ($LASTEXITCODE -ne 0) { throw "$What failed ($LASTEXITCODE)" }
}

Push-Location $root
try {
    Invoke-Native { cargo build -p painhubproxy-desktop --release } 'cargo build'
}
finally {
    Pop-Location
}

$exe = Join-Path $root 'target\release\painhubproxy.exe'
if (-not (Test-Path $exe)) { throw "binary not found: $exe" }

$dist = Join-Path $root 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$out = Join-Path $dist 'painhubproxy-desktop.exe'
Copy-Item $exe $out -Force

"EXE:    $out"
"Size:   $((Get-Item $out).Length) bytes"
"SHA256: $((Get-FileHash $out -Algorithm SHA256).Hash)"
