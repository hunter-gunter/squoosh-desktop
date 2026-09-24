#!/bin/sh
# Complete corresponding source, including the exact Cargo dependencies.
set -eu
cd "$(dirname "$0")/.."
. ./scripts/version.sh
target_dir=${CARGO_TARGET_DIR:-target}
stage="$target_dir/source-stage/squoosh-desktop-$version"
rm -rf "$stage"
mkdir -p "$stage/.cargo" "$target_dir/dist"
cp Cargo.toml Cargo.lock vcpkg.json README.md CHANGELOG.md THIRD_PARTY.md LICENSE LICENSE-APACHE-2.0 "$stage/"
cp -R app core codecs i18n docs packaging scripts "$stage/"
# Never copy makepkg artifacts into a source release.
rm -rf "$stage/packaging/pkg" "$stage/packaging/src" "$stage"/packaging/*.pkg.tar.*
cargo vendor --locked "$stage/vendor" > "$stage/.cargo/config.toml"
# Cargo's generated path is relative to this checkout; use a relocatable source path.
python3 - "$stage/.cargo/config.toml" <<'PY'
import pathlib,sys
p=pathlib.Path(sys.argv[1]);s=p.read_text();s='\n'.join('directory = "vendor"' if line.startswith('directory =') else line for line in s.splitlines())+'\n';p.write_text(s)
PY
# Keep the Windows build settings next to the vendored sources.
cat .cargo/config.toml >> "$stage/.cargo/config.toml"
tar -czf "$target_dir/dist/squoosh-desktop-$version-source.tar.gz" -C "$target_dir/source-stage" "squoosh-desktop-$version"
checksum=$(sha256sum "$target_dir/dist/squoosh-desktop-$version-source.tar.gz" | cut -d ' ' -f 1)
sed -e "s/^_version=REPLACED_BY_SOURCE_SCRIPT$/_version=$version/" \
  -e "s/REPLACED_BY_SOURCE_SCRIPT/$checksum/" packaging/PKGBUILD > "$target_dir/dist/PKGBUILD"
printf '%s\n' "Complete source and PKGBUILD: $target_dir/dist/"
