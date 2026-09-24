#!/bin/sh
# Prints the GitHub release notes of a version: its CHANGELOG.md section,
# then a download table. Fails when the changelog has no entry for it.
#   ./scripts/release-notes.sh [version] [owner/repo]
set -eu
cd "$(dirname "$0")/.."
. ./scripts/version.sh
version=${1:-$version}
repo=${2:-${GITHUB_REPOSITORY:-hunter-gunter/squoosh-desktop}}
tag="v$version"

# Lines between "## [<version>]" and the next "## [" heading.
notes=$(awk -v v="$version" '
  index($0, "## [" v "]") == 1 { found = 1; next }
  found && /^## \[/ { exit }
  found && /^\[[^]]+\]: / { exit }
  found { print }
' CHANGELOG.md | sed -e '/./,$!d')
if [ -z "$(printf '%s' "$notes" | tr -d '[:space:]')" ]; then
  echo "CHANGELOG.md has no entry for $version: add a \"## [$version]\" section." >&2
  exit 1
fi

url="https://github.com/$repo/releases/download/$tag"
pkgver=$(printf '%s' "$version" | tr '-' '_')
cat <<NOTES
$notes

## Downloads

| Platform | File |
|---|---|
| **Windows 10/11** installer | [squoosh-desktop-$version-windows-x64-setup.exe]($url/squoosh-desktop-$version-windows-x64-setup.exe) |
| **Windows 10/11** portable | [squoosh-desktop-$version-windows-x64.zip]($url/squoosh-desktop-$version-windows-x64.zip) |
| **Debian 13** | [squoosh-desktop_${version}_amd64.deb]($url/squoosh-desktop_${version}_amd64.deb) |
| **Linux** AppImage | [squoosh-desktop-$version-x86_64.AppImage]($url/squoosh-desktop-$version-x86_64.AppImage) |
| **Arch Linux** | [squoosh-desktop-$pkgver-1-x86_64.pkg.tar.zst]($url/squoosh-desktop-$pkgver-1-x86_64.pkg.tar.zst) |
| Source code and \`PKGBUILD\` | [squoosh-desktop-$version-source.tar.gz]($url/squoosh-desktop-$version-source.tar.gz) |

Checksums are in [SHA256SUMS.txt]($url/SHA256SUMS.txt). The Windows binaries are not signed yet, so SmartScreen shows a warning on first launch. See the [README](https://github.com/$repo/blob/$tag/README.md#install) for installation details.
NOTES
