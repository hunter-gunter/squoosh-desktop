# Licences et provenance

Ce projet est dérivé de [Squoosh](https://github.com/GoogleChromeLabs/squoosh), copyright Google Inc. Le code et les ressources qui en sont repris, et les adaptations de ses wrappers, portent la licence Apache-2.0 (voir `LICENSE-APACHE-2.0`).

Le binaire distribué avec imagequant est soumis à GPL-3.0-or-later. La licence Apache-2.0 des fichiers originaux est conservée. Le fichier `LICENSE` (GPL-3.0) accompagne le paquet. Distribuer le binaire avec l’archive de sources correspondantes générée par `scripts/source.sh` ; cette archive contient les versions exactes des dépendances Cargo et permet aussi de reconstruire/recombiner HQX.

Principales dépendances :

| Composant | Licence | Usage |
|---|---|---|
| egui / eframe | MIT ou Apache-2.0 | Interface et rendu |
| image | MIT ou Apache-2.0 | Images et formats matriciels |
| mozjpeg-sys / MozJPEG | IJG, BSD-3-Clause, Zlib | JPEG, bibliothèque intégrée au binaire |
| libwebp | BSD-3-Clause | WebP, bibliothèque du système (intégrée sous Windows) |
| libavif | BSD-2-Clause | AVIF, bibliothèque du système (intégrée sous Windows) |
| libyuv | BSD-3-Clause | Conversions YUV de libavif, intégrée sous Windows |
| AOM | BSD-2-Clause et licence de brevets AOM | Encodage AV1 via libavif |
| OxiPNG | MIT | Optimisation PNG |
| resvg / usvg / tiny-skia | MIT ou Apache-2.0 / BSD-3-Clause selon composant | SVG |
| resize | MIT | Filtres de redimensionnement |
| HQX | LGPL-2.1-or-later | Agrandissement de pixel art |
| imagequant | GPL-3.0-or-later | Quantification et tramage |
| Little CMS 2 | MIT | Conversion ICC, bibliothèque du système (intégrée sous Windows) |
| rfd | MIT | Dialogues de fichiers |

`Cargo.lock` fixe les versions. Les licences complètes des dépendances Cargo sont incluses avec leurs sources dans `vendor/` de l’archive de distribution. Les paquets Debian/Arch des bibliothèques système fournissent leurs propres notices et sources selon les mécanismes de la distribution. Sous Windows, libwebp, libavif, AOM, libyuv et Little CMS sont compilées par vcpkg (révision fixée dans `scripts/windows.ps1`) et liées statiquement ; leurs notices sont copiées dans le dossier `licenses/` de l’archive et de l’installation.

Les préréglages, algorithmes de rotation/redimensionnement, mode ZX et adaptateurs d’encodage sont dérivés du code source de Squoosh, copyright Google Inc., Apache-2.0. Les tables de quantification JPEG sont celles de MozJPEG.

L’interface reprend les ressources graphiques de l’application web, copyright Google Inc., Apache-2.0 : icônes SVG (`src/client/lazy-app/icons` et composants de Squoosh, recopiées dans `app/src/icons.rs`), logo (`app/assets/logo-with-text.svg`, dont le texte a été converti en tracés) et le sous-ensemble de chiffres de Roboto Mono embarqué par le web (`app/assets/roboto-mono-numbers.ttf`, Apache-2.0), icône d’application (`app/assets/icon.png`). Les quelques icônes propres à la version bureau (dossier, corbeille, menu, fermeture, alerte) viennent de Material Icons, Apache-2.0.
