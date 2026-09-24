# Builds the Windows x64 release: standalone executable, portable archive
# and Inno Setup installer in target/dist.
#
# Requirements: Visual Studio 2022 Build Tools ("Desktop development with
# C++" workload), Git, Rust, NASM and Inno Setup 6 (ISCC.exe) in the PATH or
# at their default location.
#
#   ./scripts/windows.ps1          # construction + paquets
#   ./scripts/windows.ps1 -Test    # also runs the tests
param([switch]$Test, [switch]$NoPackage)
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
# Single version: `version` of [workspace.package] in Cargo.toml.
$version = (cargo metadata --no-deps --format-version 1 --locked | ConvertFrom-Json).packages |
    Where-Object name -eq 'squoosh-desktop' | ForEach-Object version
if (-not $version) { throw 'Version not found in Cargo.toml' }
# Pinned vcpkg revision: the same libwebp, libavif, AOM and lcms2 versions
# for every build.
$vcpkgTag = '2026.07.29'
$triplet = 'x64-windows-static'
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }

function Invoke-Checked {
    param([string]$File, [string[]]$Arguments)
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$File failed (code $LASTEXITCODE)" }
}

if (-not (Get-Command nasm -ErrorAction SilentlyContinue)) {
    $nasm = Join-Path $env:ProgramFiles 'NASM'
    if (Test-Path (Join-Path $nasm 'nasm.exe')) { $env:PATH = "$nasm;$env:PATH" }
    else { throw 'NASM est introuvable (winget install NASM.NASM)' }
}

$vcpkg = Join-Path $target 'vcpkg'
if (-not (Test-Path (Join-Path $vcpkg '.git'))) {
    Invoke-Checked git @('clone', '--depth', '1', '--branch', $vcpkgTag, 'https://github.com/microsoft/vcpkg', $vcpkg)
}
$exe = Join-Path $vcpkg 'vcpkg.exe'
if (-not (Test-Path $exe)) {
    Invoke-Checked (Join-Path $vcpkg 'bootstrap-vcpkg.bat') @('-disableMetrics')
}
# vcpkg's CMake scripts call vcpkg back: keep them on this clone, not on a
# preinstalled one (GitHub runners ship C:\vcpkg).
$env:VCPKG_ROOT = (Resolve-Path $vcpkg).Path
$env:VCPKG_DOWNLOADS = Join-Path $env:VCPKG_ROOT 'downloads'
$installed = (Join-Path (Get-Location) 'vcpkg_installed')
$arguments = @('install', "--vcpkg-root=$env:VCPKG_ROOT", "--triplet=$triplet", "--x-install-root=$installed", '--disable-metrics')
# A second attempt absorbs transient tool download failures.
& $exe @arguments
if ($LASTEXITCODE -ne 0) { Invoke-Checked $exe $arguments }

$prefix = Join-Path $installed $triplet
$env:SQUOOSH_NATIVE_PREFIX = $prefix
# lcms2-sys (tests) links against the same static library.
$env:LCMS2_LIB_DIR = Join-Path $prefix 'lib'
$env:LCMS2_INCLUDE_DIR = Join-Path $prefix 'include'

if ($Test) {
    Invoke-Checked cargo @('test', '--workspace', '--locked')
}
Invoke-Checked cargo @('build', '--release', '--locked', '-p', 'squoosh-desktop')
if ($NoPackage) { return }

$dist = Join-Path $target 'dist'
$stage = Join-Path $target "windows-stage/squoosh-desktop-$version-windows-x64"
Remove-Item -Recurse -Force (Split-Path $stage) -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $stage, $dist | Out-Null
Copy-Item (Join-Path $target 'release/squoosh-desktop.exe') $stage
foreach ($file in 'README.md', 'THIRD_PARTY.md', 'LICENSE', 'LICENSE-APACHE-2.0') {
    Copy-Item $file $stage
}
# Notices of the statically linked C libraries (BSD and MIT licenses).
$licenses = New-Item -ItemType Directory -Force (Join-Path $stage 'licenses')
foreach ($port in 'libavif', 'aom', 'libyuv', 'libwebp', 'lcms') {
    Copy-Item (Join-Path $prefix "share/$port/copyright") (Join-Path $licenses "$port.txt")
}
$zip = Join-Path $dist "squoosh-desktop-$version-windows-x64.zip"
Remove-Item $zip -ErrorAction SilentlyContinue
Compress-Archive -Path $stage -DestinationPath $zip

$iscc = (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source
if (-not $iscc) { $iscc = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6/ISCC.exe' }
if (-not (Test-Path $iscc)) { throw 'Inno Setup 6 est introuvable (winget install JRSoftware.InnoSetup)' }
Invoke-Checked $iscc @('/Qp', "/DAppVersion=$version", "/DStage=$((Resolve-Path $stage).Path)",
    "/O$((Resolve-Path $dist).Path)", 'packaging/windows/squoosh-desktop.iss')
