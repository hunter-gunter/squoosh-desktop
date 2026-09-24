#!/bin/sh
# Resized AVIF and WebP copies of docs/screenshots for the website, in the
# widths listed in WIDTHS of site/build.py. Run it after changing a screenshot.
# Needs ImageMagick and avifenc (libavif).
set -eu
cd "$(dirname "$0")/.."
out=site/assets/screenshots
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
mkdir -p "$out"
for name in editor welcome batch; do
  for width in 740 1110 1480; do
    magick "docs/screenshots/$name.webp" -resize "${width}x" "$tmp/$name.png"
    # 4:4:4 keeps the coloured interface text sharp.
    avifenc -q 60 -s 4 -y 444 -j all "$tmp/$name.png" "$out/$name-$width.avif" > /dev/null
    magick "$tmp/$name.png" -quality 82 -define webp:method=6 "$out/$name-$width.webp"
  done
done
ls -l "$out"
