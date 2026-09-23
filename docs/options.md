# Correspondance des réglages Squoosh → Rust

Le registre `codecs/options.json` définit les valeurs initiales et les bornes partagées par l’interface et le moteur. Les noms source sont conservés. Toutes les options sont transmises aux codecs natifs ; `use_delta_palette` est réservé/inopérant dans libwebp et reste visible, désactivé et documenté.

## JPEG

| Option source | Libellé | Défaut | Bornes |
|---|---|---:|---|
| `quality` | Qualité | 75 | 0–100 |
| `baseline` | JPEG de base | 0 | 0–1 |
| `arithmetic` | Codage arithmétique | 0 | 0–1 |
| `progressive` | Progressif | 1 | 0–1 |
| `optimize_coding` | Optimiser le codage | 1 | 0–1 |
| `smoothing` | Lissage | 0 | 0–100 |
| `color_space` | Espace couleur | 3 | 1–3 |
| `quant_table` | Table de quantification | 3 | 0–8 |
| `trellis_multipass` | Trellis multipasse | 0 | 0–1 |
| `trellis_opt_zero` | Optimiser les zéros | 0 | 0–1 |
| `trellis_opt_table` | Optimiser la table | 0 | 0–1 |
| `trellis_loops` | Passes trellis | 1 | 1–50 |
| `auto_subsample` | Sous-échantillonnage automatique | 1 | 0–1 |
| `chroma_subsample` | Facteur chromatique | 2 | 1–4 |
| `separate_chroma_quality` | Qualité chromatique distincte | 0 | 0–1 |
| `chroma_quality` | Qualité chromatique | 75 | 0–100 |

## PNG

| Option source | Libellé | Défaut | Bornes |
|---|---|---:|---|
| `level` | Niveau d’optimisation | 2 | 0–6 |
| `interlace` | Entrelacement | 0 | 0–1 |

## WEBP

| Option source | Libellé | Défaut | Bornes |
|---|---|---:|---|
| `quality` | Qualité | 75 | 0–100 |
| `target_size` | Taille cible (octets) | 0 | 0–2147483647 |
| `target_PSNR` | PSNR cible | 0 | 0–100 |
| `method` | Effort | 4 | 0–6 |
| `sns_strength` | Mise en forme du bruit | 50 | 0–100 |
| `filter_strength` | Force du filtre | 60 | 0–100 |
| `filter_sharpness` | Netteté du filtre | 0 | 0–7 |
| `filter_type` | Filtre fort | 1 | 0–1 |
| `partitions` | Partitions | 0 | 0–3 |
| `segments` | Segments | 4 | 1–4 |
| `pass` | Passes | 1 | 1–10 |
| `show_compressed` | Reconstruction compressée | 0 | 0–1 |
| `preprocessing` | Prétraitement | 0 | 0–2 |
| `autofilter` | Filtre automatique | 0 | 0–1 |
| `partition_limit` | Limite de partition | 0 | 0–100 |
| `alpha_compression` | Compresser l’alpha | 1 | 0–1 |
| `alpha_filtering` | Filtrage alpha | 1 | 0–2 |
| `alpha_quality` | Qualité alpha | 100 | 0–100 |
| `lossless` | Sans perte | 0 | 0–1 |
| `exact` | Préserver les couleurs transparentes | 0 | 0–1 |
| `image_hint` | Type d’image | 0 | 0–3 |
| `emulate_jpeg_size` | Imiter la taille JPEG | 0 | 0–1 |
| `thread_level` | Multithread | 0 | 0–1 |
| `low_memory` | Mémoire réduite | 0 | 0–1 |
| `near_lossless` | Fidélité presque sans perte | 100 | 0–100 |
| `use_delta_palette` | Palette delta (réservé par libwebp) | 0 | 0–1 |
| `use_sharp_yuv` | Sharp YUV | 0 | 0–1 |

## AVIF

| Option source | Libellé | Défaut | Bornes |
|---|---|---:|---|
| `quality` | Qualité | 50 | 0–100 |
| `qualityAlpha` | Qualité alpha (−1 : liée) | -1 | -1–100 |
| `denoiseLevel` | Débruitage | 0 | 0–50 |
| `tileColsLog2` | Colonnes de tuiles (log₂) | 0 | 0–6 |
| `tileRowsLog2` | Lignes de tuiles (log₂) | 0 | 0–6 |
| `speed` | Vitesse | 6 | 0–10 |
| `subsample` | Sous-échantillonnage | 1 | 0–3 |
| `chromaDeltaQ` | Quantification chromatique distincte | 0 | 0–1 |
| `sharpness` | Netteté | 0 | 0–7 |
| `tune` | Optimisation | 0 | 0–2 |
| `enableSharpYUV` | Sharp YUV | 0 | 0–1 |

## Contrôles dérivés et traitements

- WebP : préréglages sans perte 0–9 → couples méthode/qualité identiques au code TypeScript ; perte légère = 100 − near_lossless ; netteté affichée = 7 − filter_sharpness.
- AVIF : effort = 10 − speed ; sans perte → qualité couleur/alpha 100 et 4:4:4 ; qualité alpha liée → −1.
- JPEG : baseline désactive progressif ; arithmétique désactive optimisation Huffman ; sous-échantillonnage et qualité chromatique ne s’appliquent qu’en YCbCr ; trellis zéro dépend du multipasse.
- Rotation commune aux deux côtés, 0/90/180/270°.
- Redimensionnement par côté : dimensions, proportions verrouillées, étirement ou recadrage central (le mode source nommé contain effectue un recadrage), prémultiplication alpha, RGB linéaire.
- Filtres : triangle, Catmull-Rom, Mitchell, Lanczos3, HQX (préagrandissement 2/3/4× puis Catmull-Rom), vectoriel SVG. Comme sur le web, sélectionner une image SVG choisit le filtre vectoriel et une image matricielle un filtre matriciel ; dans un lot, une image matricielle réglée en vectoriel est redimensionnée en Lanczos3.
- Filtres navigateur : pixelated → voisin le plus proche ; low → triangle ; medium → Catmull-Rom ; high → Lanczos3. Ces correspondances ne reproduisent pas les pixels propres à chaque navigateur.
- Palette : 2–256 couleurs, tramage 0–1, mode ZX par blocs de 8×8, deux couleurs compatibles par bloc.
- Les entrées JPEG/PNG du navigateur deviennent des alias du format natif correspondant ; les codecs de sortie hors JPEG/PNG/WebP/AVIF sont exclus conformément au périmètre.
