# Licenses and provenance

This project is derived from [Squoosh](https://github.com/GoogleChromeLabs/squoosh), copyright Google Inc. The code and assets taken from it, and the adaptations of its wrappers, are licensed under Apache-2.0 (see `LICENSE-APACHE-2.0`).

A binary built with imagequant is covered by GPL-3.0-or-later. The Apache-2.0 license of the original files is kept. The `LICENSE` file (GPL-3.0) ships with every package. Distribute the binary together with the corresponding source archive produced by `scripts/source.sh`; that archive contains the exact versions of the Cargo dependencies and also makes it possible to rebuild or relink HQX.

Main dependencies:

| Component | License | Use |
|---|---|---|
| egui / eframe | MIT or Apache-2.0 | Interface and rendering |
| image | MIT or Apache-2.0 | Raster images and formats |
| mozjpeg-sys / MozJPEG | IJG, BSD-3-Clause, Zlib | JPEG, library built into the binary |
| libwebp | BSD-3-Clause | WebP, system library (built in on Windows) |
| libavif | BSD-2-Clause | AVIF, system library (built in on Windows) |
| libyuv | BSD-3-Clause | YUV conversions for libavif, built in on Windows |
| AOM | BSD-2-Clause and AOM patent license | AV1 encoding through libavif |
| OxiPNG | MIT | PNG optimization |
| resvg / usvg / tiny-skia | MIT or Apache-2.0 / BSD-3-Clause depending on the component | SVG |
| resize | MIT | Resize filters |
| HQX | LGPL-2.1-or-later | Pixel-art upscaling |
| imagequant | GPL-3.0-or-later | Quantization and dithering |
| Little CMS 2 | MIT | ICC conversion, system library (built in on Windows) |
| rfd | MIT | File dialogs |
| sys-locale | MIT or Apache-2.0 | System language detection |
| fontdb / skrifa | MIT or Apache-2.0 | Finding system fonts for Chinese, Japanese, Korean and Vietnamese |

`Cargo.lock` pins the versions. The full licenses of the Cargo dependencies ship with their sources in the `vendor/` directory of the source archive. The Debian and Arch packages of the system libraries provide their own notices and sources through the distribution's mechanisms. On Windows, libwebp, libavif, AOM, libyuv and Little CMS are built by vcpkg (revision pinned in `scripts/windows.ps1`) and linked statically; their notices are copied to the `licenses/` folder of the archive and of the installation.

The presets, rotation and resize algorithms, ZX mode and encoder adapters are derived from the Squoosh source code, copyright Google Inc., Apache-2.0. The JPEG quantization tables are MozJPEG's.

The interface reuses the web app's graphic assets, copyright Google Inc., Apache-2.0: SVG icons (`src/client/lazy-app/icons` and Squoosh components, copied into `app/src/icons.rs`), the logo (`app/assets/logo-with-text.svg`, with its text converted to paths), the subset of Roboto Mono digits embedded by the web app (`app/assets/roboto-mono-numbers.ttf`, Apache-2.0) and the application icon (`app/assets/icon.png`). The few desktop-only icons (folder, trash, menu, close, warning) come from Material Icons, Apache-2.0.

## Screenshot photos

The photos shown in `docs/screenshots/` come from Wikimedia Commons and are not part of the application:

| Photo | Author | License |
|---|---|---|
| [Macaw parrot sitting on a tree branch](https://commons.wikimedia.org/wiki/File:Macaw_parrot_sitting_on_a_tree_branch.jpg_.jpg) | Christopher Kuszajewski | CC0 |
| [USA Antelope-Canyon](https://commons.wikimedia.org/wiki/File:USA_Antelope-Canyon.jpg) | Lucas Löffler | Public domain |
| [Colorful thermophiles at Grand Prismatic Spring](https://commons.wikimedia.org/wiki/File:Colorful_thermophiles_at_Grand_Prismatic_Spring_(33669630821).jpg) | NPS / Neal Herbert | Public domain |
| [Grand Prismatic Spring and boardwalk](https://commons.wikimedia.org/wiki/File:Grand_Prismatic_Spring_and_boardwalk.jpg) | NPS / Jacob W. Frank | Public domain |
