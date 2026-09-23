# Squoosh Desktop

Éditeur natif Rust pour Debian 13 et Arch Linux (x86_64), sous Wayland ou X11, et pour Windows 10 (1809) et 11 (x64). Conversion locale, sans télémétrie ni accès réseau au fonctionnement.

Portage natif de [Squoosh](https://github.com/GoogleChromeLabs/squoosh) (Google, Apache-2.0), dont il reprend l’interface, les réglages et les ressources graphiques.

## Compilation

Rust **1.95 ou supérieur**, compilateur C, NASM et pkg-config sont nécessaires. MozJPEG est compilé par Cargo ; libwebp, libavif avec encodeur AOM et Little CMS sont des bibliothèques système.

Debian 13 :

```sh
sudo apt install build-essential pkg-config nasm libwebp-dev libavif-dev liblcms2-dev libxkbcommon-dev libxkbcommon-x11-0 libwayland-dev libx11-dev libx11-xcb-dev libxcb1-dev libegl1-mesa-dev libgl1-mesa-dev xdg-desktop-portal xdg-desktop-portal-gtk
```

Arch Linux :

```sh
sudo pacman -S --needed rust base-devel nasm libwebp libavif aom lcms2 libxkbcommon libxkbcommon-x11 wayland libx11 libxcb libglvnd xdg-desktop-portal xdg-desktop-portal-gtk
```

Depuis la racine du dépôt :

```sh
cargo run --release --locked -p squoosh-desktop
# Ou ouvrir directement plusieurs fichiers :
cargo run --release --locked -p squoosh-desktop -- image.png photo.jpg
```

Les dialogues natifs utilisent le portail de fichiers du bureau. KDE peut utiliser `xdg-desktop-portal-kde` à la place du portail GTK. La compilation initiale télécharge les dépendances ; l’application ne télécharge rien.

### Windows

Visual Studio 2022 Build Tools (charge « Développement Desktop en C++ »), Git, Rust 1.95 ou supérieur (cible `x86_64-pc-windows-msvc`, celle par défaut de rustup), NASM et Inno Setup 6 sont nécessaires :

```powershell
winget install Microsoft.VisualStudio.2022.BuildTools Git.Git Rustlang.Rustup NASM.NASM JRSoftware.InnoSetup
./scripts/windows.ps1 -Test
```

Le script télécharge vcpkg à une révision fixée dans `target/vcpkg/`, compile libwebp, libavif/AOM et Little CMS en bibliothèques statiques (`vcpkg.json`), lance les tests puis produit dans `target/dist/` une archive portable et un installeur. L’exécutable n’a besoin d’aucune DLL tierce ni du redistribuable Visual C++. Pour un simple `cargo build`, définir `SQUOOSH_NATIVE_PREFIX` sur `vcpkg_installed\x64-windows-static` après une première exécution du script.

## Utilisation

L’interface reprend celle de [squoosh.app](https://squoosh.app) : l’image occupe toute la fenêtre, les deux panneaux de réglages flottent sur les bords (rose à gauche, bleu à droite) et les commandes de vue sont centrées en bas.

1. Ajouter des images ou un dossier, ou les déposer dans la fenêtre. Le bouton rose en haut à gauche ouvre le panneau **Images**, propre à la version bureau : liste des fichiers, réglages communs ou individuels, dossier de sortie et conversion par lot. L’option Sous-dossiers active la récursion. Les chemins identiques sont dédupliqués.
2. Configurer les deux côtés de comparaison. Chaque panneau contient **Édition** (Redimensionner, Réduire la palette) puis **Compression** (codec et ses réglages, avec Réglages avancés repliés). Le côté gauche montre initialement l’image originale ; le droit encode en AVIF qualité 50. Redimensionner propose, comme le web, une méthode, un préréglage (25 % à 400 %) et la largeur/hauteur en pixels ; décocher Conserver les proportions révèle l’ajustement (Étirer ou Recadrage central). Pendant le calcul d’un aperçu, la bulle de résultat s’estompe et affiche un indicateur de chargement. Les icônes du bandeau Édition copient les réglages vers l’autre côté, les enregistrent ou les réimportent.
3. Basculer entre Réglages communs et Cette image dans le panneau Images. Les modifications individuelles sont des exceptions par champ ; les autres champs continuent à suivre les réglages communs.
4. La bulle en bas de chaque panneau indique la taille obtenue et l’écart avec l’original. Son bouton rond choisit le côté à exporter et lance l’export vers le dossier de sortie. Exporter l’image sélectionnée ou tout le lot depuis le panneau Images. Les réglages sont copiés au lancement ; les changements suivants n’affectent pas ce lot.
5. Consulter le résultat de chaque fichier dans la liste. Une erreur n’arrête pas les autres conversions. L’annulation attend le retour du codec courant et n’efface pas les résultats déjà écrits.

Sous Windows, le glisser-déposer depuis l’Explorateur fonctionne directement et le rendu passe par Direct3D 12 (WARP sans pilote graphique, Vulkan ou OpenGL en secours ; `WGPU_BACKEND=gl` force OpenGL). Sous Linux, le glisser-déposer passe par X11 : sous Wayland, l’application démarre via XWayland, car winit ne gère pas le dépôt de fichiers en Wayland natif (`SQUOOSH_WAYLAND=1` force Wayland, sans glisser-déposer).

Déplacer le séparateur pour comparer ; faire glisser l’image pour la déplacer ; molette pour zoomer. Les commandes en bas règlent le zoom, la rotation, le lissage de l’aperçu et le fond clair ou sombre ; cliquer le pourcentage de zoom réajuste le cadrage. Les aperçus sont calculés après 300 ms sans modification ; les résultats périmés sont ignorés.

Les noms existants ne sont jamais écrasés : `photo.webp`, `photo_1.webp`, etc. Un temporaire créé dans le dossier de sortie est publié sans écrasement après l’écriture complète.

## Formats et limites explicites

Entrées JPEG, PNG, WebP, AVIF, SVG, GIF, BMP, TIFF. Sorties JPEG (MozJPEG), PNG (OxiPNG), WebP (libwebp), AVIF (libavif/AOM). GIF/animations : première image ; TIFF : première page. Le message correspondant apparaît à l’import. Pas d’export animé, HDR ou multipage.

Pipeline SDR RGBA8/sRGB, orientation EXIF appliquée, profils ICC convertis, métadonnées annexes retirées. JPEG reçoit un fond blanc configurable pour les pixels transparents. Les ressources externes SVG sont désactivées ; images embarquées et polices du système sont prises en charge. Un format ou profil invalide produit une erreur par fichier.

Limites : 40 mégapixels, 32 768 pixels par côté, fichier source de 512 Mio, liste de 100 000 chemins. Traitement d’une image à la fois ; AVIF limité à quatre threads, WebP à son mode multithread natif. Les miniatures sont limitées à 128 ; seuls les deux aperçus sélectionnés restent sur le GPU, découpés en tuiles de 1 024 pixels. Les lots et exceptions ne sont pas restaurés après fermeture.

Les contrôles applicables aux quatre formats sont répertoriés dans [la table de correspondance](docs/options.md). Les filtres spécifiques au navigateur ont des correspondances natives documentées, sans promesse d’identité pixel à pixel ou de fichiers binaires identiques. Le mode « contain » du code Squoosh d’origine effectue un recadrage central ; l’interface native le nomme explicitement ainsi.

## Paquets

Sur Debian 13, avec `dpkg-dev` installé :

```sh
./scripts/deb.sh
sudo apt install ./target/dist/squoosh-desktop_*_amd64.deb
```

AppImage portable, construite de préférence sur Debian 13 (la glibc de la machine de construction fixe la version minimale requise) :

```sh
./scripts/appimage.sh
./target/dist/squoosh-desktop-*-x86_64.AppImage
```

Le script télécharge `linuxdeploy` et `appimagetool` (versions *continuous*) dans `target/appimage-tools/` ; les variables `LINUXDEPLOY` et `APPIMAGETOOL` désignent des copies locales. libwebp, libavif/AOM et lcms2 sont embarquées ; X11, Wayland et OpenGL viennent du système.

Pour produire les sources complètes et la recette Arch avec checksum :

```sh
./scripts/source.sh
cd target/dist
makepkg -s
sudo pacman -U squoosh-desktop-*-x86_64.pkg.tar.zst
```

La recette Arch compile et teste depuis l’archive locale, sans publication AUR.

Windows (voir [Compilation](#windows)) : `./scripts/windows.ps1` produit `squoosh-desktop-<version>-windows-x64.zip` (portable) et `squoosh-desktop-<version>-windows-x64-setup.exe`. L’installeur s’installe pour l’utilisateur courant sans droits administrateur (ou pour tous sur demande), crée un raccourci dans le menu Démarrer et propose les formats d’image dans « Ouvrir avec », sans changer l’application par défaut. Le binaire n’est pas signé : SmartScreen affiche un avertissement au premier lancement tant qu’aucun certificat Authenticode n’est utilisé. Distribuer l’archive de sources avec le binaire ; voir [les notices](THIRD_PARTY.md).

### Version et publication

La version est définie une seule fois, par `version` dans la section `[workspace.package]` de `Cargo.toml`. Les trois crates, l’option `--version`, les paquets Debian, AppImage et Arch, l’archive de sources, l’exécutable et l’installeur Windows la reprennent (`./scripts/version.sh` l’affiche). Pour publier : modifier ce champ, lancer `cargo check` pour mettre `Cargo.lock` à jour, commiter, puis pousser le tag correspondant :

```sh
git tag -a v0.2.0 -m "Squoosh Desktop 0.2.0"
git push origin v0.2.0
```

Le workflow `Release` vérifie que le tag correspond à `Cargo.toml`, construit tous les paquets et les publie avec leurs sommes SHA-256 dans les Releases GitHub. Un tag de préversion (`v0.2.0-beta.1`) est publié comme tel.

## Vérification

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
cargo run --release --locked -p squoosh-core --example corpus -- target/corpus
```

La CI valide les tests et paquets sous Debian 13 (dont l’AppImage), Arch et Windows (tests, Clippy, archive, installeur et absence de DLL tierce). Les tests couvrent les 32 combinaisons de formats, les réglages, les filtres, les palettes, ICC/EXIF, les exceptions, collisions, erreurs, annulation et lots de 100 images. Le corpus généré sert aussi aux décodeurs indépendants et aux essais graphiques.

## Organisation du dépôt

| Dossier | Contenu |
|---|---|
| `app/` | Interface egui (`squoosh-desktop`) et ses ressources dans `app/assets/` |
| `core/` | Pipeline de conversion, réglages, file de travaux et tests |
| `codecs/` | Liaisons MozJPEG, OxiPNG, libwebp, libavif et registre `options.json` |
| `packaging/`, `scripts/` | Recette Arch, fichier `.desktop`, installeur Windows, construction des paquets et de l’AppImage |
| `docs/` | Correspondance des réglages avec Squoosh |

## Licence

GPL-3.0-or-later (voir `LICENSE`), en raison d’imagequant. Le code et les ressources repris de Squoosh restent sous Apache-2.0 (voir `LICENSE-APACHE-2.0` et [les notices](THIRD_PARTY.md)).
