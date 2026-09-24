//! `--features screenshot`: with `SQUOOSH_CAPTURE=shot.png`, the window is
//! saved once the previews have settled, then the application quits. This
//! produces the README screenshots. `SQUOOSH_CAPTURE_FORMATS=jpeg,webp`
//! picks the codecs of the two sides (`original`, `jpeg`, `png`, `webp`,
//! `avif`), and `SQUOOSH_CAPTURE_DRAWER=1` opens the image list.
use crate::App;
use eframe::egui;
use squoosh_codecs::Format;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

pub struct Capture {
    path: PathBuf,
    started: Instant,
    requested: bool,
}
impl Capture {
    pub fn from_env() -> Option<Self> {
        Some(Self {
            path: std::env::var_os("SQUOOSH_CAPTURE")?.into(),
            started: Instant::now(),
            requested: false,
        })
    }
}
impl App {
    pub(crate) fn capture_setup(&mut self) {
        if let Ok(formats) = std::env::var("SQUOOSH_CAPTURE_FORMATS") {
            for (side, name) in self.common.sides.iter_mut().zip(formats.split(',')) {
                side.format = Format::ALL.into_iter().find(|f| f.key() == name.trim());
            }
        }
        self.drawer = std::env::var_os("SQUOOSH_CAPTURE_DRAWER").is_some();
    }
    pub(crate) fn capture(&mut self, ctx: &egui::Context) {
        let Some(capture) = &mut self.capture else {
            return;
        };
        if let Some(image) = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        }) {
            let [w, h] = image.size;
            if let Err(error) = image::save_buffer(
                &capture.path,
                image.as_raw(),
                w as u32,
                h as u32,
                image::ExtendedColorType::Rgba8,
            ) {
                eprintln!("{}: {error}", capture.path.display());
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let settled = !self.batch
            && self.dirty.is_none()
            && !self.preview_pending
            && self
                .entries
                .iter()
                .all(|e| !e.inspected || e.dimensions.is_some());
        if !capture.requested && settled && capture.started.elapsed() > Duration::from_secs(2) {
            // Leave out the transient status line, and shoot on the next frame.
            if self.message.is_empty() {
                capture.requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            self.message.clear();
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
