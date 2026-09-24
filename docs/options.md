# Squoosh → Rust settings mapping

The `codecs/options.json` registry defines the defaults, bounds and English labels shared by the interface and the engine; the labels are translated in `i18n/locales/`. Source option names are kept as in Squoosh. Every option is passed to the native codecs; `use_delta_palette` is reserved and has no effect in libwebp, so it stays visible, disabled and documented.

## JPEG

| Source option | Label | Default | Range | Choices |
|---|---|---:|---|---|
| `quality` | Quality | 75 | 0–100 |  |
| `baseline` | Baseline JPEG | 0 | 0–1 |  |
| `arithmetic` | Arithmetic coding | 0 | 0–1 |  |
| `progressive` | Progressive rendering | 1 | 0–1 |  |
| `optimize_coding` | Optimize Huffman table | 1 | 0–1 |  |
| `smoothing` | Smoothing | 0 | 0–100 |  |
| `color_space` | Channels | 3 | 1–3 | 1 = Grayscale, 2 = RGB, 3 = YCbCr |
| `quant_table` | Quantization table | 3 | 0–8 | 0 = JPEG Annex K, 1 = Flat, 2 = MSSIM-tuned Kodak, 3 = ImageMagick, 4 = PSNR-HVS-M-tuned Kodak, 5 = Klein et al, 6 = Watson et al, 7 = Ahumada et al, 8 = Peterson et al |
| `trellis_multipass` | Trellis multipass | 0 | 0–1 |  |
| `trellis_opt_zero` | Optimize zero block runs | 0 | 0–1 |  |
| `trellis_opt_table` | Optimize after trellis quantization | 0 | 0–1 |  |
| `trellis_loops` | Trellis quantization passes | 1 | 1–50 |  |
| `auto_subsample` | Auto subsample chroma | 1 | 0–1 |  |
| `chroma_subsample` | Subsample chroma by | 2 | 1–4 |  |
| `separate_chroma_quality` | Separate chroma quality | 0 | 0–1 |  |
| `chroma_quality` | Chroma quality | 75 | 0–100 |  |

## PNG

| Source option | Label | Default | Range | Choices |
|---|---|---:|---|---|
| `level` | Optimization level | 2 | 0–6 |  |
| `interlace` | Interlace | 0 | 0–1 |  |

## WEBP

| Source option | Label | Default | Range | Choices |
|---|---|---:|---|---|
| `quality` | Quality | 75 | 0–100 |  |
| `target_size` | Target size (bytes) | 0 | 0–2147483647 |  |
| `target_PSNR` | Target PSNR | 0 | 0–100 |  |
| `method` | Effort | 4 | 0–6 |  |
| `sns_strength` | Spatial noise shaping | 50 | 0–100 |  |
| `filter_strength` | Filter strength | 60 | 0–100 |  |
| `filter_sharpness` | Filter sharpness | 0 | 0–7 |  |
| `filter_type` | Strong filter | 1 | 0–1 |  |
| `partitions` | Partitions | 0 | 0–3 |  |
| `segments` | Segments | 4 | 1–4 |  |
| `pass` | Passes | 1 | 1–10 |  |
| `show_compressed` | Show compressed | 0 | 0–1 |  |
| `preprocessing` | Preprocessing | 0 | 0–2 | 0 = None, 1 = Segment smoothing, 2 = Pseudo-random dithering |
| `autofilter` | Auto adjust filter strength | 0 | 0–1 |  |
| `partition_limit` | Partition quality limit | 0 | 0–100 |  |
| `alpha_compression` | Compress alpha | 1 | 0–1 |  |
| `alpha_filtering` | Alpha filtering | 1 | 0–2 |  |
| `alpha_quality` | Alpha quality | 100 | 0–100 |  |
| `lossless` | Lossless | 0 | 0–1 |  |
| `exact` | Preserve transparent data | 0 | 0–1 |  |
| `image_hint` | Image type | 0 | 0–3 | 0 = Default, 1 = Picture, 2 = Photo, 3 = Graph |
| `emulate_jpeg_size` | Emulate JPEG size | 0 | 0–1 |  |
| `thread_level` | Multithreading | 0 | 0–1 |  |
| `low_memory` | Low memory | 0 | 0–1 |  |
| `near_lossless` | Near-lossless fidelity | 100 | 0–100 |  |
| `use_delta_palette` | Delta palette (reserved by libwebp) | 0 | 0–1 |  |
| `use_sharp_yuv` | Sharp YUV | 0 | 0–1 |  |

## AVIF

| Source option | Label | Default | Range | Choices |
|---|---|---:|---|---|
| `quality` | Quality | 50 | 0–100 |  |
| `qualityAlpha` | Alpha quality (−1: linked) | -1 | -1–100 |  |
| `denoiseLevel` | Noise synthesis | 0 | 0–50 |  |
| `tileColsLog2` | Log2 of tile columns | 0 | 0–6 |  |
| `tileRowsLog2` | Log2 of tile rows | 0 | 0–6 |  |
| `speed` | Speed | 6 | 0–10 |  |
| `subsample` | Subsample chroma | 1 | 0–3 | 0 = 4:0:0, 1 = 4:2:0, 2 = 4:2:2, 3 = 4:4:4 |
| `chromaDeltaQ` | Extra chroma compression | 0 | 0–1 |  |
| `sharpness` | Sharpness | 0 | 0–7 |  |
| `tune` | Tuning | 0 | 0–2 | 0 = Auto, 1 = PSNR, 2 = SSIM |
| `enableSharpYUV` | Sharp YUV | 0 | 0–1 |  |

## Derived controls and processing

- **WebP**: the lossless presets 0–9 map to the same method/quality pairs as the TypeScript code; *slight loss* = 100 − `near_lossless`; displayed *filter smoothing* = 7 − `filter_sharpness`.
- **AVIF**: *effort* = 10 − `speed`; lossless sets color and alpha quality to 100 and 4:4:4; linked alpha quality is −1.
- **JPEG**: baseline disables progressive; arithmetic coding disables Huffman optimization; chroma subsampling and chroma quality only apply to YCbCr; *optimize zero block runs* requires trellis multipass.
- **Rotation** is shared by both sides: 0/90/180/270°.
- **Resize** per side: dimensions, locked aspect ratio, stretch or center crop (the source mode named *contain* performs a crop), alpha premultiplication, linear RGB.
- **Filters**: triangle, Catmull-Rom, Mitchell, Lanczos3, HQX (2/3/4× pre-upscale, then Catmull-Rom), SVG vector. As on the web, selecting an SVG image picks the vector filter and a raster image picks a raster filter; in a batch, a raster image set to vector is resized with Lanczos3.
- **Browser filters**: pixelated → nearest neighbor; low → triangle; medium → Catmull-Rom; high → Lanczos3. These mappings do not reproduce the exact pixels of each browser.
- **Palette**: 2–256 colors, dithering 0–1, ZX mode on 8×8 blocks with two compatible colors per block.
- Browser JPEG/PNG inputs become aliases of the matching native format; output codecs other than JPEG/PNG/WebP/AVIF are out of scope.
