# Construit la version Windows x64 : exécutable autonome, archive portable
# et installeur Inno Setup dans target/dist.
#
# Prérequis : Visual Studio 2022 Build Tools (charge « Développement Desktop
# en C++ »), Git, Rust, NASM et Inno Setup 6 (ISCC.exe) dans le PATH ou à
# leur emplacement par défaut.
#
#   ./scripts/windows.ps1          # construction + paquets
#   ./scripts/windows.ps1 -Test    # lance aussi les tests
param([switch]$Test, [switch]$NoPackage)
$ErrorActionPreference = 'Stop'
Set-Location (Join-Path $PSScriptRoot '..')
# Version unique : `version` de [workspace.package] dans Cargo.toml.
$version = (cargo metadata --no-deps --format-version 1 --locked | ConvertFrom-Json).packages |
    Where-Object name -eq 'squoosh-desktop' | ForEach-Object version
if (-not $version) { throw 'Version introuvable dans Cargo.toml' }
# Révision de vcpkg figée : mêmes versions de libwebp, libavif, AOM et lcms2
# à chaque construction.
$vcpkgTag = '2026.07.29'
$triplet = 'x64-windows-static'
$target = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { 'target' }

function Invoke-Checked {
    param([string]$File, [string[]]$Arguments)
    & $File @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$File a échoué (code $LASTEXITCODE)" }
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
$installed = (Join-Path (Get-Location) 'vcpkg_installed')
Invoke-Checked $exe @('install', "--vcpkg-root=$vcpkg", "--triplet=$triplet", "--x-install-root=$installed", '--disable-metrics')

$prefix = Join-Path $installed $triplet
$env:SQUOOSH_NATIVE_PREFIX = $prefix
# lcms2-sys (tests) se lie à la même bibliothèque statique.
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
# Notices des bibliothèques C liées statiquement (licences BSD et MIT).
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
