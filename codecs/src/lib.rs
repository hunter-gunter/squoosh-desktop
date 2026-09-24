use anyhow::{Context, Result, bail, ensure};
use image::{ImageEncoder, RgbaImage};
use serde::{Deserialize, Serialize};
use squoosh_i18n::{n, t, tr};
use std::{
    collections::BTreeMap,
    ffi::{CStr, c_char, c_void},
    sync::OnceLock,
};
// Ensure Cargo links the statically built MozJPEG used by native.c.
use mozjpeg_sys as _;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Jpeg,
    Png,
    Webp,
    Avif,
}
impl Format {
    pub const ALL: [Self; 4] = [Self::Jpeg, Self::Png, Self::Webp, Self::Avif];
    pub fn key(self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::Webp => "webp",
            Self::Avif => "avif",
        }
    }
    pub fn extension(self) -> &'static str {
        if self == Self::Jpeg {
            "jpg"
        } else {
            self.key()
        }
    }
}
#[derive(Debug, Clone, Deserialize)]
pub struct OptionSpec {
    pub key: String,
    pub label: String,
    pub default: i32,
    pub min: i32,
    pub max: i32,
    pub choices: Vec<String>,
}
pub fn specs(format: Format) -> &'static [OptionSpec] {
    static REGISTRY: OnceLock<BTreeMap<String, Vec<OptionSpec>>> = OnceLock::new();
    &REGISTRY.get_or_init(|| {
        serde_json::from_str(include_str!("../options.json")).expect("checked-in options")
    })[format.key()]
}
pub type Options = BTreeMap<String, i32>;
pub fn defaults(format: Format) -> Options {
    specs(format)
        .iter()
        .map(|s| (s.key.clone(), s.default))
        .collect()
}
pub fn validate(format: Format, options: &Options) -> Result<()> {
    ensure!(
        options.len() == specs(format).len(),
        t("Invalid number of settings")
    );
    for s in specs(format) {
        let v = options
            .get(&s.key)
            .with_context(|| tr!("Missing setting: {}", s.key))?;
        ensure!(
            (s.min..=s.max).contains(v),
            tr!("{} must be between {} and {}", t(&s.label), s.min, s.max)
        );
    }
    Ok(())
}
pub fn checked_dimensions(w: u32, h: u32) -> Result<usize> {
    ensure!(
        w > 0 && h > 0 && w <= 32768 && h <= 32768,
        t("Invalid dimensions (at most 32,768 per side)")
    );
    let pixels = u64::from(w) * u64::from(h);
    ensure!(
        pixels <= 40_000_000,
        t("Image too large (at most 40 megapixels)")
    );
    Ok(pixels as usize * 4)
}
unsafe extern "C" {
    fn sq_free(p: *mut c_void);
    fn sq_jpeg_decode(
        p: *const u8,
        size: usize,
        out: *mut *mut u8,
        len: *mut usize,
        w: *mut u32,
        h: *mut u32,
        error: *mut c_char,
    ) -> i32;
    fn sq_jpeg(
        p: *const u8,
        w: u32,
        h: u32,
        o: *const i32,
        out: *mut *mut u8,
        len: *mut usize,
        error: *mut c_char,
    ) -> i32;
    fn sq_webp(
        p: *const u8,
        w: u32,
        h: u32,
        o: *const i32,
        out: *mut *mut u8,
        len: *mut usize,
        error: *mut c_char,
    ) -> i32;
    fn sq_avif(
        p: *const u8,
        w: u32,
        h: u32,
        o: *const i32,
        out: *mut *mut u8,
        len: *mut usize,
        error: *mut c_char,
    ) -> i32;
    fn sq_icc(p: *mut u8, count: usize, icc: *const u8, size: usize, error: *mut c_char) -> i32;
    fn sq_avif_decode(
        p: *const u8,
        size: usize,
        out: *mut *mut u8,
        len: *mut usize,
        w: *mut u32,
        h: *mut u32,
        rot: *mut i32,
        mirror: *mut i32,
        animated: *mut i32,
        error: *mut c_char,
    ) -> i32;
}
fn message(error: &[c_char; 512]) -> String {
    // Native functions always NUL-terminate their fixed-size error buffer.
    let message = unsafe {
        CStr::from_ptr(error.as_ptr())
            .to_string_lossy()
            .into_owned()
    };
    translate_native(message)
}
/// `native.c` reports in English; library messages (libjpeg, libavif) stay as is.
fn translate_native(message: String) -> String {
    const MESSAGES: [&str; 14] = [
        n("Cannot allocate the result"),
        n("Cannot allocate JPEG memory"),
        n("Cannot allocate AVIF memory"),
        n("Incompatible libwebp ABI"),
        n("Incompatible WebP ABI"),
        n("Invalid WebP settings"),
        n("Profile or image too large"),
        n("Invalid ICC profile"),
        n("Unsupported ICC profile"),
        n("HDR AVIF is not supported by the SDR pipeline"),
        n("Invalid AVIF crop"),
        n("JPEG exceeds the dimension limit"),
        n("Invalid CMYK profile"),
        // Followed by " (code N)".
        n("WebP failed"),
    ];
    match MESSAGES.iter().find(|en| message.starts_with(*en)) {
        Some(en) => format!("{}{}", t(en), &message[en.len()..]),
        None => message,
    }
}
unsafe fn take_buffer(ptr: *mut u8, len: usize) -> Vec<u8> {
    // SAFETY: successful native calls return a malloc-owned buffer of exactly len bytes.
    let bytes = unsafe { std::slice::from_raw_parts(ptr, len) }.to_vec();
    unsafe { sq_free(ptr.cast()) };
    bytes
}
pub fn encode(image: &RgbaImage, format: Format, options: &Options) -> Result<Vec<u8>> {
    checked_dimensions(image.width(), image.height())?;
    validate(format, options)?;
    if format == Format::Png {
        let mut png = Vec::new();
        image::codecs::png::PngEncoder::new(&mut png).write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            image::ExtendedColorType::Rgba8,
        )?;
        let mut opt = oxipng::Options::from_preset(options["level"] as u8);
        opt.interlace = Some(options["interlace"] == 1);
        opt.strip = oxipng::StripChunks::Safe;
        return Ok(oxipng::optimize_from_memory(&png, &opt)?);
    }
    let values: Vec<i32> = specs(format).iter().map(|s| options[&s.key]).collect();
    let (mut ptr, mut len) = (std::ptr::null_mut(), 0);
    let mut error = [0; 512];
    // SAFETY: dimensions and all option bounds are validated. The build script generates
    // the C struct in the same registry order as values, using only int32_t fields.
    let ok = unsafe {
        match format {
            Format::Jpeg => sq_jpeg(
                image.as_ptr(),
                image.width(),
                image.height(),
                values.as_ptr(),
                &mut ptr,
                &mut len,
                error.as_mut_ptr(),
            ),
            Format::Webp => sq_webp(
                image.as_ptr(),
                image.width(),
                image.height(),
                values.as_ptr(),
                &mut ptr,
                &mut len,
                error.as_mut_ptr(),
            ),
            Format::Avif => sq_avif(
                image.as_ptr(),
                image.width(),
                image.height(),
                values.as_ptr(),
                &mut ptr,
                &mut len,
                error.as_mut_ptr(),
            ),
            Format::Png => unreachable!(),
        }
    };
    if ok == 0 {
        bail!(message(&error));
    }
    Ok(unsafe { take_buffer(ptr, len) })
}
pub fn apply_icc(image: &mut RgbaImage, profile: &[u8]) -> Result<()> {
    if profile.is_empty() {
        return Ok(());
    }
    let mut error = [0; 512];
    // SAFETY: Rust slices provide valid lengths and remain exclusively borrowed throughout.
    let ok = unsafe {
        sq_icc(
            image.as_mut_ptr(),
            image.len() / 4,
            profile.as_ptr(),
            profile.len(),
            error.as_mut_ptr(),
        )
    };
    ensure!(ok != 0, "{}", message(&error));
    Ok(())
}
pub fn decode_avif(bytes: &[u8]) -> Result<(RgbaImage, bool)> {
    let (mut ptr, mut len, mut w, mut h, mut rot, mut mirror, mut animated) =
        (std::ptr::null_mut(), 0, 0, 0, 0, -1, 0);
    let mut error = [0; 512];
    // SAFETY: native decoder bounds allocations and initializes every output on success.
    let ok = unsafe {
        sq_avif_decode(
            bytes.as_ptr(),
            bytes.len(),
            &mut ptr,
            &mut len,
            &mut w,
            &mut h,
            &mut rot,
            &mut mirror,
            &mut animated,
            error.as_mut_ptr(),
        )
    };
    ensure!(ok != 0, "{}", message(&error));
    let bytes = unsafe { take_buffer(ptr, len) };
    checked_dimensions(w, h)?;
    let mut img = RgbaImage::from_raw(w, h, bytes).context(t("Invalid AVIF pixels"))?;
    img = match rot {
        1 => image::imageops::rotate270(&img),
        2 => image::imageops::rotate180(&img),
        3 => image::imageops::rotate90(&img),
        _ => img,
    };
    img = match mirror {
        0 => image::imageops::flip_vertical(&img),
        1 => image::imageops::flip_horizontal(&img),
        _ => img,
    };
    Ok((img, animated != 0))
}

pub fn decode_jpeg(bytes: &[u8]) -> Result<RgbaImage> {
    let (mut ptr, mut len, mut w, mut h) = (std::ptr::null_mut(), 0, 0, 0);
    let mut error = [0; 512];
    // SAFETY: C bounds the decoded dimensions before allocating and owns its longjmp scope.
    let ok = unsafe {
        sq_jpeg_decode(
            bytes.as_ptr(),
            bytes.len(),
            &mut ptr,
            &mut len,
            &mut w,
            &mut h,
            error.as_mut_ptr(),
        )
    };
    ensure!(ok != 0, "{}", message(&error));
    let bytes = unsafe { take_buffer(ptr, len) };
    checked_dimensions(w, h)?;
    RgbaImage::from_raw(w, h, bytes).context(t("Invalid JPEG pixels"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use squoosh_i18n::{Lang, set_lang};

    // The only test of this crate that switches the process-wide language.
    #[test]
    fn messages_and_labels_follow_the_language() {
        set_lang(Lang::Fr);
        assert_eq!(
            translate_native("WebP failed (code 3)".into()),
            "Échec WebP (code 3)"
        );
        assert_eq!(
            translate_native("libavif says no".into()),
            "libavif says no"
        );
        assert_eq!(t(&specs(Format::Jpeg)[0].label), "Qualité");
        let error = validate(Format::Png, &Options::new()).unwrap_err();
        assert_eq!(error.to_string(), "Nombre de réglages invalide");
        set_lang(Lang::En);
        assert_eq!(
            translate_native("Invalid ICC profile".into()),
            "Invalid ICC profile"
        );
        // Every registry entry parses, and its choices fit its range.
        for f in Format::ALL {
            for s in specs(f) {
                assert!(s.choices.is_empty() || s.choices.len() == (s.max - s.min + 1) as usize);
            }
        }
    }
}
