#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
. ./scripts/version.sh
target_dir=${CARGO_TARGET_DIR:-target}
cargo build --release --locked -p squoosh-desktop
stage="$target_dir/deb-stage"
rm -rf "$stage"
mkdir -p "$stage/DEBIAN" "$target_dir/dist"
install -Dm755 "$target_dir/release/squoosh-desktop" "$stage/usr/bin/squoosh-desktop"
install -Dm644 packaging/squoosh-desktop.desktop "$stage/usr/share/applications/squoosh-desktop.desktop"
install -Dm644 app/assets/icon.png "$stage/usr/share/icons/hicolor/512x512/apps/squoosh-desktop.png"
for file in README.md THIRD_PARTY.md LICENSE LICENSE-APACHE-2.0; do
  install -Dm644 "$file" "$stage/usr/share/doc/squoosh-desktop/$file"
done
# Resolve linked Debian library dependencies against the build environment.
# libgl/Wayland/portal dependencies are loaded dynamically by the window toolkit.
mkdir -p "$target_dir/debian-shlibdeps/debian"
cat > "$target_dir/debian-shlibdeps/debian/control" <<'CONTROL'
Source: squoosh-desktop
Section: graphics
Priority: optional
Maintainer: Squoosh Desktop contributors <noreply@example.invalid>

Package: squoosh-desktop
Architecture: amd64
Description: Native image editor and batch converter
CONTROL
binary=$(realpath "$stage/usr/bin/squoosh-desktop")
deps=$(cd "$target_dir/debian-shlibdeps" && dpkg-shlibdeps --ignore-missing-info -O -e "$binary" | sed -n 's/^shlibs:Depends=//p')
cat > "$stage/DEBIAN/control" <<CONTROL
Package: squoosh-desktop
Version: $version
Section: graphics
Priority: optional
Architecture: amd64
Maintainer: Squoosh Desktop contributors <noreply@example.invalid>
Depends: $deps, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libx11-6, libxcb1, libegl1, libgl1
Recommends: xdg-desktop-portal, xdg-desktop-portal-gtk
Description: Native image editor and batch converter
 Local JPEG, PNG, WebP and AVIF conversion with an interactive comparison editor.
CONTROL
dpkg-deb --root-owner-group --build "$stage" "$target_dir/dist/squoosh-desktop_${version}_amd64.deb"
