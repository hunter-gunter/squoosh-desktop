//! Writes a fixed corpus for independent decoders and GUI smoke tests.
use image::{Rgba, RgbaImage};
use squoosh_codecs::Format;
use squoosh_core::{
    pipeline::{Source, convert},
    settings::Settings,
};
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};
fn main() -> anyhow::Result<()> {
    let dir = PathBuf::from(std::env::args_os().nth(1).unwrap_or("target/corpus".into()));
    fs::create_dir_all(&dir)?;
    let image = RgbaImage::from_fn(640, 480, |x, y| {
        let inside = (x as i32 - 320).pow(2) + (y as i32 - 240).pow(2) < 150 * 150;
        if inside {
            Rgba([240, (y * 255 / 480) as u8, 110, 255])
        } else {
            Rgba([
                (x * 255 / 640) as u8,
                75,
                (y * 255 / 480) as u8,
                if x < 100 { 100 } else { 255 },
            ])
        }
    });
    image.save(dir.join("source.png"))?;
    let source = Source {
        image,
        bytes: 0,
        warning: None,
        svg: None,
    };
    for format in Format::ALL {
        let mut settings = Settings::default();
        settings.sides[1].format = Some(format);
        let result = convert(&source, &settings, 1, &AtomicBool::new(false), |_| {})?;
        fs::write(
            dir.join(format!("output.{}", format.extension())),
            result.bytes,
        )?;
    }
    Ok(())
}
