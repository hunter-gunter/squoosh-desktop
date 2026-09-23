#!/bin/sh
# Portable AppImage bundling the linked codec libraries (libwebp, libavif/AOM, lcms2).
# Build it on the oldest distribution to support: the host glibc sets the minimum.
set -eu
cd "$(dirname "$0")/.."
. ./scripts/version.sh
arch=x86_64
target_dir=${CARGO_TARGET_DIR:-target}
tools="$target_dir/appimage-tools"
# Continuous releases; set LINUXDEPLOY or APPIMAGETOOL to use pinned local copies.
linuxdeploy=${LINUXDEPLOY:-$tools/linuxdeploy-$arch.AppImage}
appimagetool=${APPIMAGETOOL:-$tools/appimagetool-$arch.AppImage}
fetch() {
  if [ ! -x "$1" ]; then
    mkdir -p "$(dirname "$1")"
    curl -fL --proto '=https' --tlsv1.2 -o "$1.part" "$2"
    chmod +x "$1.part"
    mv "$1.part" "$1"
  fi
}
fetch "$linuxdeploy" "https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-$arch.AppImage"
fetch "$appimagetool" "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-$arch.AppImage"
# Containers and CI runners usually lack FUSE.
export APPIMAGE_EXTRACT_AND_RUN=1
cargo build --release --locked -p squoosh-desktop
appdir="$target_dir/appimage-stage/SquooshDesktop.AppDir"
rm -rf "$appdir"
mkdir -p "$target_dir/dist"
install -Dm755 "$target_dir/release/squoosh-desktop" "$appdir/usr/bin/squoosh-desktop"
# The 1024 px icon exceeds the hicolor sizes linuxdeploy accepts; pixmaps has no size check.
install -Dm644 app/assets/icon.png "$appdir/usr/share/pixmaps/squoosh-desktop.png"
for file in README.md THIRD_PARTY.md LICENSE LICENSE-APACHE-2.0; do
  install -Dm644 "$file" "$appdir/usr/share/doc/squoosh-desktop/$file"
done
# linuxdeploy copies DT_NEEDED libraries outside its exclude list and sets RPATH.
# X11, Wayland, xkbcommon and GL are loaded dynamically by the window toolkit from the host.
"$linuxdeploy" --appdir "$appdir" \
  --executable "$appdir/usr/bin/squoosh-desktop" \
  --desktop-file packaging/squoosh-desktop.desktop
output="$target_dir/dist/squoosh-desktop-$version-$arch.AppImage"
rm -f "$output"
ARCH=$arch "$appimagetool" --no-appstream "$appdir" "$output"
printf '%s\n' "AppImage : $output"
