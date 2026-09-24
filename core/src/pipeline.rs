use crate::settings::{Filter, Settings, Side};
use anyhow::{Context, Result, ensure};
use image::{DynamicImage, ImageDecoder, ImageReader, RgbaImage};
use rgb::{FromSlice, RGBA};
use squoosh_codecs::{Format, checked_dimensions};
use squoosh_i18n::{t, tr};
use std::{
    fs,
    io::Cursor,
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
};

pub struct Source {
    pub image: RgbaImage,
    pub bytes: u64,
    pub warning: Option<String>,
    pub svg: Option<Vec<u8>>,
}
pub struct Converted {
    pub image: RgbaImage,
    pub bytes: Vec<u8>,
    pub format: Format,
}
pub const MAX_FILE_BYTES: u64 = 512 * 1024 * 1024;
fn too_large() -> &'static str {
    t("File larger than 512 MiB")
}
fn first_frame_only(kind: &str) -> String {
    tr!(
        "{kind}: only the first frame will be converted",
        kind = kind
    )
}
pub fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    ensure!(!cancel.load(Ordering::Relaxed), t("Cancelled"));
    Ok(())
}
pub fn supported(path: &Path) -> bool {
    path.extension().and_then(|x| x.to_str()).is_some_and(|x| {
        [
            "jpg", "jpeg", "png", "webp", "avif", "svg", "gif", "bmp", "tif", "tiff",
        ]
        .contains(&x.to_ascii_lowercase().as_str())
    })
}
fn render_svg(bytes: &[u8], dimensions: Option<(u32, u32)>, crop: bool) -> Result<RgbaImage> {
    let mut opt = resvg::usvg::Options::default();
    // Embedded resources only. No file-system or network fetches from an SVG.
    opt.image_href_resolver.resolve_string = Box::new(|_, _| None);
    opt.fontdb_mut().load_system_fonts();
    let tree = resvg::usvg::Tree::from_data(bytes, &opt).context("SVG invalide")?;
    let size = tree.size();
    let (w, h) = dimensions.unwrap_or((size.width().ceil() as u32, size.height().ceil() as u32));
    checked_dimensions(w, h)?;
    let mut pix = resvg::tiny_skia::Pixmap::new(w, h).context("Allocation SVG impossible")?;
    let sx = w as f32 / size.width();
    let sy = h as f32 / size.height();
    let transform = if crop {
        let scale = sx.max(sy);
        resvg::tiny_skia::Transform::from_row(
            scale,
            0.,
            0.,
            scale,
            (w as f32 - size.width() * scale) / 2.,
            (h as f32 - size.height() * scale) / 2.,
        )
    } else {
        resvg::tiny_skia::Transform::from_scale(sx, sy)
    };
    resvg::render(&tree, transform, &mut pix.as_mut());
    let mut raw = pix.take();
    for p in raw.as_chunks_mut::<4>().0 {
        if p[3] > 0 {
            for c in 0..3 {
                p[c] = ((u32::from(p[c]) * 255 + u32::from(p[3]) / 2) / u32::from(p[3])).min(255)
                    as u8;
            }
        }
    }
    RgbaImage::from_raw(w, h, raw).context("Pixels SVG invalides")
}
pub fn load(path: &Path) -> Result<Source> {
    let file = fs::File::open(path).with_context(|| tr!("Cannot open {}", path.display()))?;
    let len = file.metadata()?.len();
    ensure!(len <= MAX_FILE_BYTES, too_large());
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= MAX_FILE_BYTES, too_large());
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "svg" {
        return Ok(Source {
            image: render_svg(&bytes, None, false)?,
            bytes: bytes.len() as u64,
            warning: None,
            svg: Some(bytes),
        });
    }
    let guessed = image::guess_format(&bytes).context(t("Unrecognized image format"))?;
    if guessed == image::ImageFormat::Avif {
        let (image, animated) = squoosh_codecs::decode_avif(&bytes)?;
        return Ok(Source {
            image,
            bytes: bytes.len() as u64,
            warning: animated.then(|| first_frame_only("Animation")),
            svg: None,
        });
    }
    if guessed == image::ImageFormat::Jpeg {
        let image = squoosh_codecs::decode_jpeg(&bytes)?;
        let mut dynamic = DynamicImage::ImageRgba8(image);
        if let Ok(exif) = exif::Reader::new().read_from_container(&mut Cursor::new(&bytes))
            && let Some(field) = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
            && let Some(value) = field
                .value
                .get_uint(0)
                .and_then(|v| image::metadata::Orientation::from_exif(v as u8))
        {
            dynamic.apply_orientation(value);
        }
        return Ok(Source {
            image: dynamic.into_rgba8(),
            bytes: bytes.len() as u64,
            warning: None,
            svg: None,
        });
    }
    let mut reader = ImageReader::with_format(Cursor::new(&bytes), guessed);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(32768);
    limits.max_image_height = Some(32768);
    limits.max_alloc = Some(512 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder()?;
    let (w, h) = decoder.dimensions();
    checked_dimensions(w, h)?;
    let orientation = decoder.orientation()?;
    let icc = decoder.icc_profile()?;
    let mut dynamic = DynamicImage::from_decoder(decoder)?;
    dynamic.apply_orientation(orientation);
    let mut image = dynamic.to_rgba8();
    if let Some(icc) = icc {
        squoosh_codecs::apply_icc(&mut image, &icc)?;
    }
    let warning = match guessed {
        image::ImageFormat::Gif => Some(first_frame_only("GIF")),
        image::ImageFormat::Tiff => Some(tr!("TIFF: only the first page will be converted")),
        image::ImageFormat::WebP | image::ImageFormat::Png => {
            let animated = bytes.windows(4).any(|x| x == b"ANIM" || x == b"acTL");
            animated.then(|| first_frame_only("Animation"))
        }
        _ => None,
    };
    Ok(Source {
        image,
        bytes: bytes.len() as u64,
        warning,
        svg: None,
    })
}
pub fn rotate(img: &RgbaImage, degrees: u16) -> RgbaImage {
    match degrees {
        90 => image::imageops::rotate90(img),
        180 => image::imageops::rotate180(img),
        270 => image::imageops::rotate270(img),
        _ => img.clone(),
    }
}
pub fn target_dimensions(w: u32, h: u32, side: &Side) -> Result<(u32, u32)> {
    let r = &side.resize;
    let (dw, dh) = if r.width == 0 && r.height == 0 {
        (w, h)
    } else if r.width == 0 {
        (
            ((u64::from(w) * u64::from(r.height)) / u64::from(h)).max(1) as u32,
            r.height,
        )
    } else if r.height == 0 || r.lock_ratio {
        (
            r.width,
            ((u64::from(h) * u64::from(r.width)) / u64::from(w)).max(1) as u32,
        )
    } else {
        (r.width, r.height)
    };
    checked_dimensions(dw, dh)?;
    Ok((dw, dh))
}
fn srgb_to_linear(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn linear_to_srgb(v: f32) -> f32 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}
fn resize_image(mut image: RgbaImage, side: &Side, dw: u32, dh: u32) -> Result<RgbaImage> {
    let r = &side.resize;
    // `Vector` only applies to SVG sources (handled by the caller); a raster
    // image in the same batch falls back to Lanczos3 instead of failing.
    if r.filter == Filter::Hqx {
        let factor = ((dw as f64 / image.width() as f64)
            .max(dh as f64 / image.height() as f64)
            .ceil() as u32)
            .clamp(1, 4);
        if factor > 1 {
            let (w, h) = (image.width(), image.height());
            checked_dimensions(w * factor, h * factor)?;
            let source: Vec<u32> = image
                .pixels()
                .map(|p| u32::from_be_bytes([p[3], p[0], p[1], p[2]]))
                .collect();
            let mut dest = vec![0; source.len() * (factor * factor) as usize];
            match factor {
                2 => hqx::hq2x(&source, &mut dest, w, h),
                3 => hqx::hq3x(&source, &mut dest, w, h),
                _ => hqx::hq4x(&source, &mut dest, w, h),
            }
            let raw: Vec<u8> = dest
                .into_iter()
                .flat_map(|p| {
                    let [a, r, g, b] = p.to_be_bytes();
                    [r, g, b, a]
                })
                .collect();
            image = RgbaImage::from_raw(w * factor, h * factor, raw).context("HQX invalide")?;
        }
    }
    if r.crop {
        let (w, h) = image.dimensions();
        let ratio = dw as f64 / dh as f64;
        let (cw, ch) = if ratio > w as f64 / h as f64 {
            (w, (w as f64 / ratio).round().clamp(1., h as f64) as u32)
        } else {
            ((h as f64 * ratio).round().clamp(1., w as f64) as u32, h)
        };
        image = image::imageops::crop_imm(&image, (w - cw) / 2, (h - ch) / 2, cw, ch).to_image();
    }
    if r.filter == Filter::Pixelated {
        return Ok(image::imageops::resize(
            &image,
            dw,
            dh,
            image::imageops::FilterType::Nearest,
        ));
    }
    let filter = match r.filter {
        Filter::Triangle | Filter::BrowserLow => resize::Type::Triangle,
        Filter::Catrom | Filter::BrowserMedium | Filter::Hqx => resize::Type::Catrom,
        Filter::Mitchell => resize::Type::Mitchell,
        _ => resize::Type::Lanczos3,
    };
    let mut input = Vec::<RGBA<f32>>::with_capacity(image.len() / 4);
    for p in image.pixels() {
        let a = p[3] as f32 / 255.;
        let color = |v: u8| {
            let v = v as f32 / 255.;
            let v = if r.linear_rgb { srgb_to_linear(v) } else { v };
            if r.premultiply { v * a } else { v }
        };
        input.push(RGBA::new(color(p[0]), color(p[1]), color(p[2]), a));
    }
    let mut output = vec![RGBA::new(0., 0., 0., 0.); (dw * dh) as usize];
    resize::new(
        image.width() as usize,
        image.height() as usize,
        dw as usize,
        dh as usize,
        resize::Pixel::RGBAF32,
        filter,
    )?
    .resize(&input, &mut output)?;
    let mut raw = Vec::with_capacity(output.len() * 4);
    for p in output {
        let a = p.a.clamp(0., 1.);
        let color = |v: f32| {
            let v = if r.premultiply {
                if a > 0. { v / a } else { 0. }
            } else {
                v
            };
            let v = if r.linear_rgb { linear_to_srgb(v) } else { v };
            (v * 255.).round().clamp(0., 255.) as u8
        };
        raw.extend_from_slice(&[color(p.r), color(p.g), color(p.b), (a * 255.).round() as u8]);
    }
    RgbaImage::from_raw(dw, dh, raw).context(t("Invalid resize"))
}
fn quantize(image: RgbaImage, colors: u32, dither: f32, fixed: &[[u8; 4]]) -> Result<RgbaImage> {
    let mut attr = imagequant::new();
    attr.set_max_colors(colors)?;
    let mut input = attr.new_image(
        image.as_raw().as_rgba(),
        image.width() as usize,
        image.height() as usize,
        0.,
    )?;
    for c in fixed {
        input.add_fixed_color(imagequant::RGBA::new(c[0], c[1], c[2], c[3]))?;
    }
    let mut result = attr.quantize(&mut input)?;
    result.set_dithering_level(dither)?;
    let (palette, indices) = result.remapped(&mut input)?;
    let raw: Vec<u8> = indices
        .into_iter()
        .flat_map(|i| {
            let p = palette[i as usize];
            [p.r, p.g, p.b, p.a]
        })
        .collect();
    RgbaImage::from_raw(image.width(), image.height(), raw).context(t("Invalid palette"))
}
fn zx(image: &RgbaImage, dither: f32, cancel: &AtomicBool) -> Result<RgbaImage> {
    const COLORS: [[u8; 4]; 15] = [
        [0, 0, 0, 255],
        [0, 0, 215, 255],
        [215, 0, 0, 255],
        [215, 0, 215, 255],
        [0, 215, 0, 255],
        [0, 215, 215, 255],
        [215, 215, 0, 255],
        [215, 215, 215, 255],
        [0, 0, 255, 255],
        [255, 0, 0, 255],
        [255, 0, 255, 255],
        [0, 255, 0, 255],
        [0, 255, 255, 255],
        [255, 255, 0, 255],
        [255, 255, 255, 255],
    ];
    let mut output = RgbaImage::new(image.width(), image.height());
    for y in (0..image.height()).step_by(8) {
        check_cancel(cancel)?;
        for x in (0..image.width()).step_by(8) {
            let block = image::imageops::crop_imm(
                image,
                x,
                y,
                8.min(image.width() - x),
                8.min(image.height() - y),
            )
            .to_image();
            let mut counts = [0; 15];
            for p in block.pixels() {
                let index = COLORS
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, c)| {
                        (0..3)
                            .map(|i| (i32::from(p[i]) - i32::from(c[i])).pow(2))
                            .sum::<i32>()
                    })
                    .unwrap()
                    .0;
                counts[index] += 1;
            }
            let mut order: Vec<usize> = (0..15).collect();
            order.sort_by_key(|&i| std::cmp::Reverse(counts[i]));
            let first = order[0];
            let mut second = order[1];
            for candidate in order.iter().skip(1) {
                second = *candidate;
                if first != 0 && second != 0 {
                    if first >= 8 && second < 8 {
                        second += 7;
                    } else if first < 8 && second >= 8 {
                        second -= 7;
                    }
                }
                if first != second {
                    break;
                }
            }
            let block = quantize(block, 2, dither, &[COLORS[first], COLORS[second]])?;
            image::imageops::replace(&mut output, &block, x.into(), y.into());
        }
    }
    Ok(output)
}
pub fn convert(
    source: &Source,
    settings: &Settings,
    side_index: usize,
    cancel: &AtomicBool,
    mut progress: impl FnMut(&str),
) -> Result<Converted> {
    settings.validate()?;
    ensure!(side_index < 2, t("Invalid side"));
    check_cancel(cancel)?;
    let side = &settings.sides[side_index];
    let format = side.format.context(t(
        "Choose an output format; the original side cannot be exported",
    ))?;
    progress(t("Processing"));
    let mut image = rotate(&source.image, settings.rotation);
    if side.resize.enabled {
        let (dw, dh) = target_dimensions(image.width(), image.height(), side)?;
        image = if side.resize.filter == Filter::Vector
            && let Some(bytes) = source.svg.as_ref()
        {
            let dims = if settings.rotation == 90 || settings.rotation == 270 {
                (dh, dw)
            } else {
                (dw, dh)
            };
            rotate(
                &render_svg(bytes, Some(dims), side.resize.crop)?,
                settings.rotation,
            )
        } else {
            resize_image(image, side, dw, dh)?
        };
    }
    check_cancel(cancel)?;
    if side.palette.enabled {
        image = if side.palette.zx {
            zx(&image, side.palette.dither, cancel)?
        } else {
            quantize(image, side.palette.colors, side.palette.dither, &[])?
        };
    }
    if format == Format::Jpeg {
        for p in image.pixels_mut() {
            let a = u32::from(p[3]);
            for c in 0..3 {
                p[c] = ((u32::from(p[c]) * a + u32::from(side.background[c]) * (255 - a) + 127)
                    / 255) as u8;
            }
            p[3] = 255;
        }
    }
    check_cancel(cancel)?;
    progress(t("Encoding"));
    let bytes = squoosh_codecs::encode(&image, format, &side.options[format.key()])?;
    check_cancel(cancel)?;
    progress(t("Decoding"));
    let image = match format {
        Format::Avif => squoosh_codecs::decode_avif(&bytes)?.0,
        Format::Jpeg => squoosh_codecs::decode_jpeg(&bytes)?,
        _ => image::load_from_memory(&bytes)?.to_rgba8(),
    };
    check_cancel(cancel)?;
    Ok(Converted {
        image,
        bytes,
        format,
    })
}
