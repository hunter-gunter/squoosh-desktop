use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use squoosh_codecs::{Format, Options, defaults, validate};
use squoosh_i18n::{t, tr};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Filter {
    Triangle,
    Catrom,
    Mitchell,
    Lanczos3,
    Hqx,
    Pixelated,
    BrowserLow,
    BrowserMedium,
    BrowserHigh,
    Vector,
}
impl Filter {
    pub const ALL: [Self; 10] = [
        Self::Triangle,
        Self::Catrom,
        Self::Mitchell,
        Self::Lanczos3,
        Self::Hqx,
        Self::Pixelated,
        Self::BrowserLow,
        Self::BrowserMedium,
        Self::BrowserHigh,
        Self::Vector,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Triangle => "Triangle",
            Self::Catrom => "Catmull-Rom",
            Self::Mitchell => "Mitchell",
            Self::Lanczos3 => "Lanczos3",
            Self::Hqx => "HQX",
            Self::Pixelated => t("Pixelated (nearest)"),
            Self::BrowserLow => t("Browser low (triangle)"),
            Self::BrowserMedium => t("Browser medium (bicubic)"),
            Self::BrowserHigh => t("Browser high (Lanczos3)"),
            Self::Vector => t("Vector (SVG)"),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Resize {
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
    pub lock_ratio: bool,
    pub crop: bool,
    pub filter: Filter,
    pub premultiply: bool,
    pub linear_rgb: bool,
}
impl Default for Resize {
    fn default() -> Self {
        Self {
            enabled: false,
            width: 0,
            height: 0,
            lock_ratio: true,
            crop: false,
            filter: Filter::Lanczos3,
            premultiply: true,
            linear_rgb: true,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Palette {
    pub enabled: bool,
    pub colors: u32,
    pub dither: f32,
    pub zx: bool,
}
impl Default for Palette {
    fn default() -> Self {
        Self {
            enabled: false,
            colors: 256,
            dither: 1.0,
            zx: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Side {
    pub format: Option<Format>,
    pub options: BTreeMap<String, Options>,
    pub resize: Resize,
    pub palette: Palette,
    pub background: [u8; 3],
}
impl Default for Side {
    fn default() -> Self {
        Self {
            format: Some(Format::Jpeg),
            options: Format::ALL
                .into_iter()
                .map(|f| (f.key().into(), defaults(f)))
                .collect(),
            resize: Resize::default(),
            palette: Palette::default(),
            background: [255; 3],
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub rotation: u16,
    pub sides: [Side; 2],
    pub export_side: usize,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            rotation: 0,
            sides: [
                Side {
                    format: None,
                    ..Side::default()
                },
                Side {
                    format: Some(Format::Avif),
                    ..Side::default()
                },
            ],
            export_side: 1,
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            [0, 90, 180, 270].contains(&self.rotation),
            t("Invalid rotation")
        );
        ensure!(self.export_side < 2, t("Invalid side"));
        for s in &self.sides {
            ensure!(
                s.resize.width <= 32768 && s.resize.height <= 32768,
                t("Dimensions too large")
            );
            ensure!(
                (2..=256).contains(&s.palette.colors),
                t("Palette: 2 to 256 colors")
            );
            ensure!(
                s.palette.dither.is_finite() && (0.0..=1.0).contains(&s.palette.dither),
                t("Invalid dithering")
            );
            for f in Format::ALL {
                validate(
                    f,
                    s.options
                        .get(f.key())
                        .ok_or_else(|| anyhow::anyhow!(t("Missing settings")))?,
                )?;
            }
        }
        Ok(())
    }
}
/// JSON pointers to individual typed settings. Pins survive unrelated global edits.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Overrides(pub BTreeMap<String, Value>);
impl Overrides {
    pub fn resolve(&self, common: &Settings) -> Result<Settings> {
        let mut value = serde_json::to_value(common)?;
        for (path, replacement) in &self.0 {
            *value
                .pointer_mut(path)
                .ok_or_else(|| anyhow::anyhow!(tr!("Unknown setting: {path}", path = path)))? =
                replacement.clone();
        }
        let result: Settings = serde_json::from_value(value)?;
        result.validate()?;
        Ok(result)
    }
    pub fn record(&mut self, before: &Settings, after: &Settings) {
        fn walk(path: String, a: &Value, b: &Value, out: &mut BTreeMap<String, Value>) {
            match (a, b) {
                (Value::Object(a), Value::Object(b)) => {
                    for (k, v) in b {
                        walk(format!("{path}/{k}"), &a[k], v, out);
                    }
                }
                (Value::Array(a), Value::Array(b)) => {
                    for (i, v) in b.iter().enumerate() {
                        walk(format!("{path}/{i}"), &a[i], v, out);
                    }
                }
                _ => {
                    if a != b {
                        out.insert(path, b.clone());
                    }
                }
            }
        }
        walk(
            String::new(),
            &serde_json::to_value(before).unwrap(),
            &serde_json::to_value(after).unwrap(),
            &mut self.0,
        );
    }
}
