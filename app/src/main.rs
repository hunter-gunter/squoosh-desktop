// No console window next to the GUI on Windows.
#![cfg_attr(windows, windows_subsystem = "windows")]
mod controls;
mod icons;
mod theme;

use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Id, Layout, Pos2, Rect, Sense, Shape,
    Stroke, TextureHandle, TextureOptions, Vec2, vec2,
};
use squoosh_core::{
    jobs::{Command, Event, PreviewSide, Task, Worker},
    settings::{Filter, Overrides, Settings, Side},
};
use std::{
    collections::{HashSet, VecDeque},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use theme::{Icon, Theme};

struct Entry {
    id: u64,
    path: PathBuf,
    overrides: Overrides,
    status: String,
    output: Option<PathBuf>,
    bytes: Option<u64>,
    dimensions: Option<(u32, u32)>,
    warning: Option<String>,
    thumbnail: Option<TextureHandle>,
    inspected: bool,
}
struct Tile {
    texture: TextureHandle,
    offset: Vec2,
    size: Vec2,
}
struct Preview {
    image: image::RgbaImage,
    tiles: Vec<Tile>,
    dimensions: (u32, u32),
    bytes: u64,
}
impl Preview {
    fn upload(ctx: &egui::Context, side: PreviewSide, index: usize, aliasing: bool) -> Self {
        let mut preview = Self {
            image: side.image,
            tiles: Vec::new(),
            dimensions: side.dimensions,
            bytes: side.bytes,
        };
        preview.retile(ctx, index, aliasing);
        preview
    }
    /// Re-uploads the tiles, e.g. when the smoothing toggle flips the sampler.
    fn retile(&mut self, ctx: &egui::Context, index: usize, aliasing: bool) {
        let options = if aliasing {
            TextureOptions::NEAREST
        } else {
            TextureOptions::LINEAR
        };
        self.tiles.clear();
        for y in (0..self.image.height()).step_by(1024) {
            for x in (0..self.image.width()).step_by(1024) {
                let w = 1024.min(self.image.width() - x);
                let h = 1024.min(self.image.height() - y);
                let image = image::imageops::crop_imm(&self.image, x, y, w, h).to_image();
                let texture = ctx.load_texture(
                    format!("preview-{index}-{x}-{y}"),
                    egui::ColorImage::from_rgba_unmultiplied(
                        [w as usize, h as usize],
                        image.as_raw(),
                    ),
                    options,
                );
                self.tiles.push(Tile {
                    texture,
                    offset: vec2(x as f32, y as f32),
                    size: vec2(w as f32, h as f32),
                });
            }
        }
    }
}
enum Dialog {
    Import(Vec<PathBuf>),
    Directory(PathBuf),
    DirectoryCancelled,
    SaveSide(usize, PathBuf),
    LoadSide(usize, PathBuf),
}
/// Deferred edits raised from inside the panel closures.
enum Action {
    Export(usize),
    CopySide(usize),
    SaveSide(usize),
    LoadSide(usize),
}
struct App {
    ctx: egui::Context,
    worker: Worker,
    entries: Vec<Entry>,
    next_id: u64,
    selected: Option<usize>,
    common: Settings,
    individual: bool,
    recursive: bool,
    output: Option<PathBuf>,
    dialogs: crossbeam_channel::Receiver<Dialog>,
    dialog_tx: crossbeam_channel::Sender<Dialog>,
    previews: [Option<Preview>; 2],
    preview_errors: [Option<String>; 2],
    generation: u64,
    dirty: Option<Instant>,
    preview_pending: bool,
    /// When the current preview request started, to delay the spinner.
    pending_since: Instant,
    zoom: f32,
    pan: Vec2,
    split: f32,
    drag_split: bool,
    thumbnail_lru: VecDeque<u64>,
    batch: bool,
    cancel: Arc<AtomicBool>,
    batch_ids: HashSet<u64>,
    done: usize,
    total: usize,
    message: String,
    message_at: Instant,
    /// An export waiting for the output folder dialog (`true` = whole batch).
    pending_export: Option<bool>,
    // Viewport chrome, mirroring the web controls.
    drawer: bool,
    alt_background: bool,
    aliasing: bool,
    checker: Option<TextureHandle>,
    /// Scale that fits the image in the viewport, for the zoom readout.
    fit_scale: f32,
}
/// The two-up bar width, grip radius and grab band, as in `two-up`.
const SPLIT_BAR: f32 = 10.;
const SPLIT_GRIP: f32 = 33.;
const SPLIT_TOUCH: f32 = 40.;
/// Gap between the viewport control bar and the bottom of the window.
const CONTROLS_MARGIN: f32 = 24.;
/// Width of the desktop-only image list.
const DRAWER_WIDTH: f32 = 272.;
fn size(bytes: u64) -> String {
    let (value, unit) = theme::pretty_bytes(bytes);
    format!("{value} {unit}")
}
impl App {
    fn new(cc: &eframe::CreationContext<'_>, paths: Vec<PathBuf>) -> Self {
        let ctx = cc.egui_ctx.clone();
        let worker = Worker::new(move || ctx.request_repaint());
        theme::install(&cc.egui_ctx);
        let (dialog_tx, dialogs) = crossbeam_channel::unbounded();
        let app = Self {
            ctx: cc.egui_ctx.clone(),
            worker,
            entries: Vec::new(),
            next_id: 1,
            selected: None,
            common: Settings::default(),
            individual: false,
            recursive: false,
            output: None,
            dialogs,
            dialog_tx,
            previews: [None, None],
            preview_errors: [None, None],
            generation: 0,
            dirty: None,
            preview_pending: false,
            pending_since: Instant::now(),
            zoom: 1.,
            pan: Vec2::ZERO,
            split: 0.5,
            drag_split: false,
            thumbnail_lru: VecDeque::new(),
            batch: false,
            cancel: Arc::new(AtomicBool::new(false)),
            batch_ids: HashSet::new(),
            done: 0,
            total: 0,
            message: String::new(),
            message_at: Instant::now(),
            pending_export: None,
            drawer: false,
            alt_background: false,
            aliasing: false,
            checker: None,
            fit_scale: 1.,
        };
        if !paths.is_empty() {
            app.command(Command::Import {
                paths,
                recursive: false,
            });
        }
        app
    }
    /// Shows a transient status line, like `shared/custom-els/snack-bar`.
    fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.message_at = Instant::now();
    }
    fn command(&self, c: Command) {
        let _ = self.worker.commands.send(c);
    }
    fn resolved(&self) -> Settings {
        self.selected
            .and_then(|i| self.entries.get(i))
            .and_then(|e| e.overrides.resolve(&self.common).ok())
            .unwrap_or_else(|| self.common.clone())
    }
    fn changed(&mut self) {
        self.generation += 1;
        self.worker
            .latest_preview
            .store(self.generation, Ordering::Relaxed);
        self.dirty = Some(Instant::now());
        if !self.preview_pending {
            self.pending_since = Instant::now();
        }
        self.preview_pending = true;
    }
    fn select(&mut self, i: usize) {
        self.selected = Some(i);
        self.previews = [None, None];
        self.preview_errors = [None, None];
        self.zoom = 1.;
        self.pan = Vec2::ZERO;
        self.changed();
        // Like the web, a new source picks its resize method: vector for an
        // SVG, and a raster filter otherwise.
        let svg = self.entries[i]
            .path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
        let before = if self.individual {
            self.resolved()
        } else {
            self.common.clone()
        };
        let mut after = before.clone();
        for side in &mut after.sides {
            let resize = &mut side.resize;
            if svg && resize.filter != Filter::Vector {
                resize.filter = Filter::Vector;
            } else if !svg && resize.filter == Filter::Vector {
                resize.filter = Filter::Lanczos3;
            }
        }
        self.apply(before, after);
    }
    fn dialog(&self, kind: u8, ctx: &egui::Context) {
        let tx = self.dialog_tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let dialog = rfd::FileDialog::new();
            let result = match kind {
                0 => dialog
                    .set_title("Ajouter des images")
                    .add_filter(
                        "Images",
                        &[
                            "jpg", "jpeg", "png", "webp", "avif", "svg", "gif", "bmp", "tif",
                            "tiff",
                        ],
                    )
                    .pick_files()
                    .map(Dialog::Import),
                1 => dialog
                    .set_title("Ajouter un dossier")
                    .pick_folder()
                    .map(|p| Dialog::Import(vec![p])),
                2 => dialog
                    .set_title("Dossier de sortie")
                    .pick_folder()
                    .map(Dialog::Directory),
                3 | 4 => {
                    let index = usize::from(kind == 4);
                    dialog
                        .set_title("Réglages du côté")
                        .add_filter("Réglages Squoosh", &["json"])
                        .set_file_name("squoosh-side.json")
                        .save_file()
                        .map(|p| Dialog::SaveSide(index, p))
                }
                _ => {
                    let index = usize::from(kind == 6);
                    dialog
                        .set_title("Importer des réglages")
                        .add_filter("Réglages Squoosh", &["json"])
                        .pick_file()
                        .map(|p| Dialog::LoadSide(index, p))
                }
            };
            let result = result.or((kind == 2).then_some(Dialog::DirectoryCancelled));
            if let Some(result) = result {
                let _ = tx.send(result);
                ctx.request_repaint();
            }
        });
    }
    fn events(&mut self, ctx: &egui::Context) {
        while let Ok(dialog) = self.dialogs.try_recv() {
            match dialog {
                Dialog::Import(paths) => self.command(Command::Import {
                    paths,
                    recursive: self.recursive,
                }),
                Dialog::Directory(path) => {
                    self.output = Some(path);
                    if let Some(all) = self.pending_export.take() {
                        self.start(all);
                    }
                }
                Dialog::DirectoryCancelled => {
                    if self.pending_export.take().is_some() {
                        self.set_message("Export annulé : aucun dossier de sortie choisi.");
                    }
                }
                Dialog::SaveSide(index, path) => {
                    let side = &self.resolved().sides[index];
                    match serde_json::to_vec_pretty(side)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| std::fs::write(&path, bytes).map_err(|e| e.to_string()))
                    {
                        Ok(()) => {
                            self.set_message(format!("Réglages enregistrés : {}", path.display()));
                        }
                        Err(error) => {
                            self.set_message(format!("Enregistrement impossible : {error}"));
                        }
                    }
                }
                Dialog::LoadSide(index, path) => {
                    match std::fs::read(&path)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| {
                            serde_json::from_slice::<Side>(&bytes).map_err(|e| e.to_string())
                        }) {
                        Ok(side) => {
                            let mut settings = if self.individual {
                                self.resolved()
                            } else {
                                self.common.clone()
                            };
                            let before = settings.clone();
                            settings.sides[index] = side;
                            self.apply(before, settings);
                            self.set_message("Réglages importés.");
                        }
                        Err(error) => self.set_message(format!("Import impossible : {error}")),
                    }
                }
            }
        }
        // Limit GPU uploads per frame to retain responsiveness during import.
        for _ in 0..4 {
            let Ok(event) = self.worker.events.try_recv() else {
                break;
            };
            match event {
                Event::Imported(paths) => {
                    let first_new = self.entries.len();
                    let mut known: HashSet<PathBuf> =
                        self.entries.iter().map(|e| e.path.clone()).collect();
                    for path in paths {
                        if known.insert(path.clone()) {
                            self.entries.push(Entry {
                                id: self.next_id,
                                path,
                                overrides: Overrides::default(),
                                status: "Prête".into(),
                                output: None,
                                bytes: None,
                                dimensions: None,
                                warning: None,
                                thumbnail: None,
                                inspected: false,
                            });
                            self.next_id += 1;
                        }
                    }
                    // Jump to what was just added, as dropping a file on the web does.
                    if first_new < self.entries.len() {
                        self.select(first_new);
                    }
                    // Reveal the image list as soon as there is a choice to make.
                    if self.entries.len() > 1 {
                        self.drawer = true;
                    }
                    self.set_message(if self.entries.len() == 1 {
                        "1 image dans la liste".into()
                    } else {
                        format!("{} images dans la liste", self.entries.len())
                    });
                }
                Event::ImportError(error) => self.set_message(error),
                Event::Inspected {
                    id,
                    thumbnail,
                    dimensions,
                    bytes,
                    warning,
                } => {
                    if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
                        entry.bytes = Some(bytes);
                        entry.dimensions = Some(dimensions);
                        entry.warning = warning;
                        entry.thumbnail = Some(ctx.load_texture(
                            format!("thumbnail-{id}"),
                            egui::ColorImage::from_rgba_unmultiplied(
                                [thumbnail.width() as usize, thumbnail.height() as usize],
                                thumbnail.as_raw(),
                            ),
                            TextureOptions::LINEAR,
                        ));
                        self.thumbnail_lru.push_back(id);
                    }
                    while self.thumbnail_lru.len() > 128 {
                        if let Some(id) = self.thumbnail_lru.pop_front()
                            && let Some(e) = self.entries.iter_mut().find(|e| e.id == id)
                        {
                            e.thumbnail = None;
                        }
                    }
                }
                Event::InspectError { id, error } => {
                    if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
                        e.status = format!("Erreur : {error}");
                    }
                }
                Event::Preview {
                    generation,
                    sides,
                    warning,
                } => {
                    if generation == self.generation {
                        self.preview_pending = false;
                        self.previews = [None, None];
                        self.preview_errors = [None, None];
                        for (i, side) in sides.into_iter().enumerate() {
                            match side {
                                Ok(side) => {
                                    self.previews[i] =
                                        Some(Preview::upload(ctx, side, i, self.aliasing))
                                }
                                Err(error) => self.preview_errors[i] = Some(error),
                            }
                        }
                        if let Some(warning) = warning {
                            self.set_message(warning);
                        }
                    }
                }
                Event::PreviewError { generation, error } => {
                    if generation == self.generation {
                        self.preview_pending = false;
                        self.preview_errors = [Some(error.clone()), Some(error)];
                    }
                }
                Event::Stage { id, stage } => {
                    if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
                        e.status = stage;
                    }
                }
                Event::Finished { id, result } => {
                    self.done += 1;
                    self.batch_ids.remove(&id);
                    if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
                        match result {
                            Ok((path, bytes)) => {
                                e.status = format!("Terminée · {}", size(bytes));
                                e.output = Some(path);
                            }
                            Err(error) => e.status = format!("Erreur : {error}"),
                        }
                    }
                }
                Event::BatchDone { cancelled } => {
                    self.batch = false;
                    for e in &mut self.entries {
                        if self.batch_ids.contains(&e.id) {
                            e.status = "Annulée".into();
                        }
                    }
                    self.batch_ids.clear();
                    self.set_message(if cancelled {
                        "Lot annulé. Les fichiers déjà écrits sont conservés.".into()
                    } else {
                        format!("Lot terminé : {} fichiers traités.", self.done)
                    });
                    self.changed();
                }
            }
        }
        // The results bubble needs the source size, so inspect the selected
        // image even when the image list is hidden.
        if !self.batch
            && let Some(i) = self.selected
            && !self.entries[i].inspected
        {
            self.entries[i].inspected = true;
            let (id, path) = (self.entries[i].id, self.entries[i].path.clone());
            self.command(Command::Inspect { id, path });
        }
        if !self.batch
            && self
                .dirty
                .is_some_and(|t| t.elapsed() >= Duration::from_millis(300))
        {
            self.dirty = None;
            if let Some(i) = self.selected {
                self.command(Command::Preview {
                    generation: self.generation,
                    path: self.entries[i].path.clone(),
                    settings: Box::new(self.resolved()),
                });
            } else {
                self.preview_pending = false;
            }
        }
        if self.dirty.is_some() || self.preview_pending || self.batch {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    /// Commits an edited copy of the settings to the right place.
    fn apply(&mut self, before: Settings, after: Settings) {
        if after == before {
            return;
        }
        if self.individual {
            if let Some(i) = self.selected {
                self.entries[i].overrides.record(&before, &after);
            }
        } else {
            self.common = after;
        }
        self.changed();
    }
    fn start(&mut self, all: bool) {
        let Some(directory) = self.output.clone() else {
            // Ask for the folder, and resume this export once it is chosen.
            self.pending_export = Some(all);
            self.dialog(2, &self.ctx.clone());
            return;
        };
        let mut tasks = Vec::new();
        for (i, e) in self.entries.iter().enumerate() {
            if !all && self.selected != Some(i) {
                continue;
            }
            match e.overrides.resolve(&self.common) {
                Ok(settings) => tasks.push(Task {
                    id: e.id,
                    path: e.path.clone(),
                    settings,
                }),
                Err(error) => {
                    self.set_message(error.to_string());
                    return;
                }
            }
        }
        if tasks.is_empty() {
            return;
        }
        if tasks
            .iter()
            .any(|t| t.settings.sides[t.settings.export_side].format.is_none())
        {
            self.set_message(
                "Choisissez un format pour le côté exporté (l’image originale ne s’exporte pas).",
            );
            return;
        }
        self.batch_ids = tasks.iter().map(|t| t.id).collect();
        self.total = tasks.len();
        self.done = 0;
        self.batch = true;
        self.cancel = Arc::new(AtomicBool::new(false));
        for e in &mut self.entries {
            if self.batch_ids.contains(&e.id) {
                e.status = "En attente".into();
                e.output = None;
            }
        }
        self.command(Command::Batch {
            tasks,
            directory,
            cancel: self.cancel.clone(),
        });
    }

    // ------------------------------------------------------------ viewport

    fn checker_texture(&mut self, ctx: &egui::Context) -> TextureHandle {
        self.checker
            .get_or_insert_with(|| {
                // `base.css` tiles a 20px square of 2.5% black over white.
                let faint = Color32::from_rgb(249, 249, 249);
                ctx.load_texture(
                    "page-checker",
                    egui::ColorImage {
                        size: [2, 2],
                        pixels: vec![Color32::WHITE, faint, faint, Color32::WHITE],
                        source_size: vec2(2., 2.),
                    },
                    TextureOptions::NEAREST_REPEAT,
                )
            })
            .clone()
    }
    fn viewport_input(&mut self, ui: &egui::Ui, rect: Rect, response: &egui::Response) {
        let split_x = rect.left() + rect.width() * self.split;
        let grip = Pos2::new(split_x, rect.center().y);
        // Generous hit area: a wide band around the bar plus the whole grip.
        let on_handle = |p: Pos2| {
            (p.x - split_x).abs() <= SPLIT_TOUCH / 2. || p.distance(grip) <= SPLIT_GRIP + 4.
        };
        if response.drag_started() {
            // Use where the button went down, not where the drag was detected.
            let origin = ui.input(|i| i.pointer.press_origin());
            self.drag_split = origin
                .or(response.interact_pointer_pos())
                .is_some_and(on_handle);
        }
        let hovering = response.hover_pos().is_some_and(on_handle);
        if hovering || (self.drag_split && response.dragged()) {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
        if response.dragged() {
            if self.drag_split {
                if let Some(p) = response.interact_pointer_pos() {
                    self.split = ((p.x - rect.left()) / rect.width()).clamp(0.03, 0.97);
                }
            } else {
                self.pan += ui.input(|i| i.pointer.delta());
            }
        }
        if response.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if scroll != 0. {
                self.zoom = (self.zoom * (scroll * 0.002).exp()).clamp(0.02, 64.);
            }
        }
        if response.double_clicked() {
            self.fit();
        }
    }
    fn fit(&mut self) {
        self.zoom = 1.;
        self.pan = Vec2::ZERO;
    }
    fn paint_viewport(&mut self, ui: &egui::Ui, rect: Rect) {
        let checker = self.checker_texture(ui.ctx());
        let painter = ui.painter();
        painter.image(
            checker.id(),
            rect,
            Rect::from_min_size(Pos2::ZERO, rect.size() / 20.),
            Color32::WHITE,
        );
        if !self.alt_background {
            // `.output::before` — black at 80% over the page background.
            painter.rect_filled(rect, 0., Color32::from_black_alpha(204));
        }
        let max_w = self
            .previews
            .iter()
            .flatten()
            .map(|p| p.dimensions.0)
            .max()
            .unwrap_or(1) as f32;
        let max_h = self
            .previews
            .iter()
            .flatten()
            .map(|p| p.dimensions.1)
            .max()
            .unwrap_or(1) as f32;
        let fit = (rect.width() / max_w).min(rect.height() / max_h) * 0.86;
        self.fit_scale = fit;
        // A common pixel scale and pan keep the two sides spatially synchronized.
        let scale = fit * self.zoom;
        let split_x = rect.left() + rect.width() * self.split;
        for i in 0..2 {
            let clip = if i == 0 {
                Rect::from_min_max(rect.min, Pos2::new(split_x, rect.bottom()))
            } else {
                Rect::from_min_max(Pos2::new(split_x, rect.top()), rect.max)
            };
            if let Some(preview) = &self.previews[i] {
                // Like the web, each side is contained in the same box so a
                // resized output stays comparable with the original.
                let (w, h) = (preview.dimensions.0 as f32, preview.dimensions.1 as f32);
                // Like the web, each side is contained in the same box so a
                // resized output stays comparable with the original.
                let pixel = (max_w / w).min(max_h / h) * scale;
                let size = vec2(w, h) * pixel;
                let origin = rect.center() + self.pan - size / 2.;
                let p = painter.with_clip_rect(clip);
                for tile in &preview.tiles {
                    p.image(
                        tile.texture.id(),
                        Rect::from_min_size(origin + tile.offset * pixel, tile.size * pixel),
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                        Color32::WHITE,
                    );
                }
            }
        }
        // The two-up handle: a dark translucent bar and a black round grip
        // with the pink (left) and blue (right) arrows.
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(split_x - SPLIT_BAR / 2., rect.top()),
                Pos2::new(split_x + SPLIT_BAR / 2., rect.bottom()),
            ),
            0.,
            Color32::from_black_alpha(150),
        );
        let grip = Pos2::new(split_x, rect.center().y);
        painter.circle_filled(grip, SPLIT_GRIP, theme::BLACK);
        for (dir, color) in [(-1., theme::PINK), (1., theme::BLUE)] {
            painter.add(Shape::convex_polygon(
                vec![
                    grip + vec2(19. * dir, 0.),
                    grip + vec2(6. * dir, -11.),
                    grip + vec2(6. * dir, 11.),
                ],
                color,
                Stroke::NONE,
            ));
        }
    }
    /// `Output/controls` — the floating button groups under the viewport.
    fn viewport_controls(&mut self, ui: &mut egui::Ui, rotation: &mut u16) -> bool {
        let mut rotated = false;
        // Anchor the bar a clear margin above the window edge, level with
        // the results bubbles; a narrow viewport wraps it onto two rows.
        let area = ui.max_rect();
        let total = theme::BUTTON_SIZE * 5. + theme::ZOOM_WIDTH + 6.;
        let height = if area.width() >= total + 12. {
            theme::BUTTON_SIZE
        } else {
            theme::BUTTON_SIZE * 2. + 6.
        };
        let row = Rect::from_min_max(
            Pos2::new(area.left(), area.bottom() - CONTROLS_MARGIN - height),
            Pos2::new(area.right(), area.bottom() - CONTROLS_MARGIN),
        );
        let mut bar = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .id_salt("viewport-controls")
                .layout(Layout::top_down(Align::Center)),
        );
        bar.scope(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(6., 6.);
                let zoom_group = theme::BUTTON_SIZE * 2. + theme::ZOOM_WIDTH;
                let view_group = theme::BUTTON_SIZE * 3.;
                let total = zoom_group + 6. + view_group;
                ui.add_space(((ui.available_width() - total) / 2.).max(0.));
                // Each group is allocated whole so it never wraps mid-way.
                ui.allocate_ui_with_layout(
                    vec2(zoom_group, theme::BUTTON_SIZE),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 0.;
                        if theme::group_button(ui, Some(Icon::Minus), None, true, false, false)
                            .clicked()
                        {
                            self.zoom = (self.zoom / 1.25).clamp(0.02, 64.);
                        }
                        let percent = (self.fit_scale * self.zoom * 100.).round() as i32;
                        let zoom = theme::group_button(
                            ui,
                            None,
                            Some(&format!("{percent} %")),
                            false,
                            false,
                            false,
                        );
                        if zoom.clicked() {
                            self.fit();
                        }
                        zoom.on_hover_text("Cliquer pour ajuster à la fenêtre");
                        if theme::group_button(ui, Some(Icon::Plus), None, false, true, false)
                            .clicked()
                        {
                            self.zoom = (self.zoom * 1.25).clamp(0.02, 64.);
                        }
                    },
                );
                ui.allocate_ui_with_layout(
                    vec2(view_group, theme::BUTTON_SIZE),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 0.;
                        if theme::group_button(ui, Some(Icon::Rotate), None, true, false, false)
                            .on_hover_text("Pivoter")
                            .clicked()
                        {
                            *rotation = (*rotation + 90) % 360;
                            rotated = true;
                        }
                        let icon = if self.aliasing {
                            Icon::AliasingActive
                        } else {
                            Icon::Aliasing
                        };
                        if theme::group_button(ui, Some(icon), None, false, false, self.aliasing)
                            .on_hover_text("Lissage de l\u{2019}aperçu")
                            .clicked()
                        {
                            self.aliasing = !self.aliasing;
                            let ctx = ui.ctx().clone();
                            for (i, preview) in self.previews.iter_mut().enumerate() {
                                if let Some(preview) = preview {
                                    preview.retile(&ctx, i, self.aliasing);
                                }
                            }
                        }
                        let icon = if self.alt_background {
                            Icon::BackgroundActive
                        } else {
                            Icon::Background
                        };
                        if theme::group_button(
                            ui,
                            Some(icon),
                            None,
                            false,
                            true,
                            self.alt_background,
                        )
                        .on_hover_text("Fond clair / sombre")
                        .clicked()
                        {
                            self.alt_background = !self.alt_background;
                        }
                    },
                );
            });
        });
        rotated
    }

    // -------------------------------------------------------------- panels

    fn side_panel(
        &self,
        ui: &mut egui::Ui,
        index: usize,
        settings: &mut Settings,
        svg: bool,
        source_name: &str,
        actions: &mut Vec<Action>,
    ) {
        let theme = Theme::of(index);
        let radius = theme.scroller_radius();
        let full = ui.max_rect();
        // Input size after rotation, for the resize presets and fields.
        let source = self
            .selected
            .and_then(|i| self.entries[i].dimensions)
            .map(|(w, h)| {
                if settings.rotation % 180 == 90 {
                    (h, w)
                } else {
                    (w, h)
                }
            });
        // `.options` is bottom-aligned and only as tall as its content, with
        // the results bubble sitting underneath it.
        const RESULTS_HEIGHT: f32 = 88.;
        const GAP: f32 = 6.;
        let room = (full.height() - RESULTS_HEIGHT - GAP).max(0.);
        let height_id = Id::new(("options-height", index));
        let measured = ui.data(|d| d.get_temp::<f32>(height_id)).unwrap_or(room);
        let height = measured.clamp(0., room);
        let scroller = Rect::from_min_max(
            Pos2::new(full.left(), full.bottom() - RESULTS_HEIGHT - GAP - height),
            Pos2::new(full.right(), full.bottom() - RESULTS_HEIGHT - GAP),
        );
        let results = Rect::from_min_max(
            Pos2::new(full.left(), full.bottom() - RESULTS_HEIGHT),
            full.max,
        );
        ui.painter().rect_filled(scroller, radius, theme::OFF_BLACK);

        let mut options = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(scroller)
                .id_salt(("options-ui", index))
                .layout(Layout::top_down(Align::Min)),
        );
        options.visuals_mut().selection.bg_fill = theme.main;
        options.spacing_mut().item_spacing.y = 0.;
        let mut content_height = measured;
        egui::ScrollArea::vertical()
            .id_salt(("options", index))
            .auto_shrink([false, false])
            .show(&mut options, |ui| {
                let used = ui.scope(|ui| {
                    ui.set_width(ui.available_width());
                    ui.spacing_mut().item_spacing.y = 0.;
                    let original = settings.sides[index].format.is_none();
                    if original {
                        // The web hides the edit block for the untouched image.
                        controls::compress(
                            ui,
                            &mut settings.sides[index],
                            theme,
                            source_name,
                            radius,
                        );
                    } else {
                        controls::edit(
                            ui,
                            &mut settings.sides[index],
                            svg,
                            theme,
                            radius,
                            source,
                            |ui| {
                                let color = theme.header_text;
                                if theme::title_icon_button(ui, Icon::Import, color, true)
                                    .on_hover_text("Importer des réglages")
                                    .clicked()
                                {
                                    actions.push(Action::LoadSide(index));
                                }
                                if theme::title_icon_button(ui, Icon::Save, color, true)
                                    .on_hover_text("Enregistrer les réglages")
                                    .clicked()
                                {
                                    actions.push(Action::SaveSide(index));
                                }
                                if theme::title_icon_button(ui, Icon::Swap, color, true)
                                    .on_hover_text("Copier vers l\u{2019}autre côté")
                                    .clicked()
                                {
                                    actions.push(Action::CopySide(index));
                                }
                            },
                        );
                        controls::compress(
                            ui,
                            &mut settings.sides[index],
                            theme,
                            source_name,
                            CornerRadius::ZERO,
                        );
                    }
                });
                content_height = used.response.rect.height();
            });
        ui.data_mut(|d| d.insert_temp(height_id, content_height));

        let mut bubble = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(results)
                .id_salt(("results-ui", index))
                .layout(Layout::top_down(Align::Min)),
        );
        bubble.visuals_mut().selection.bg_fill = theme.main;
        self.results(&mut bubble, index, theme, settings, actions);
    }
    /// `Compress/Results` — the size bubble with its percentage flag, next to
    /// the download blob. Geometry follows `Results/style.css`.
    fn results(
        &self,
        ui: &mut egui::Ui,
        index: usize,
        theme: Theme,
        settings: &Settings,
        actions: &mut Vec<Action>,
    ) {
        const REM: f32 = theme::REM;
        // `.download`: 63px plus 30px of overflow for the blobs.
        const BLOB: f32 = 93.;
        const FLAG_HEIGHT: f32 = theme::BIG_NUMBER + 1.4 * REM;
        const ARROW: f32 = 16.;
        const SPEECH: f32 = 2.1 * REM;
        const GAP: f32 = 0.9 * REM;

        let original = settings.sides[index].format.is_none();
        // Like `Results`, only react once a computation lasts a moment.
        let loading = !original
            && self.preview_pending
            && self.pending_since.elapsed() > Duration::from_millis(150);
        let fade = if loading { 0.45 } else { 1.0 };
        let out_bytes = self.previews[index].as_ref().map(|p| p.bytes);
        let source_bytes = self.selected.and_then(|i| self.entries[i].bytes);
        let (rect, _) = theme::block(ui, 88., Sense::hover());
        let painter = ui.painter().clone();
        let cy = rect.center().y;

        // Contents, measured first because the bubble is only as wide as them.
        let text = |t: &str, font: FontId, color: Color32| {
            painter.layout_no_wrap(t.to_owned(), font, color)
        };
        let (value, unit) = out_bytes
            .map(theme::pretty_bytes)
            .unwrap_or_else(|| ("…".into(), ""));
        let unit_color = if original {
            Color32::from_white_alpha(194)
        } else {
            theme.main
        };
        let value_g = text(
            &value,
            FontId::proportional(theme::BODY),
            theme::WHITE.gamma_multiply(fade),
        );
        let unit_g = text(
            unit,
            FontId::proportional(theme::BODY),
            unit_color.gamma_multiply(fade),
        );
        let size_w = value_g.size().x
            + if unit.is_empty() {
                0.
            } else {
                4. + unit_g.size().x
            };

        let percent = match (out_bytes, source_bytes) {
            (Some(out), Some(src)) if src > 0 => {
                let ratio = out as f64 / src as f64;
                let absolute = (ratio * 100.).round() as i64;
                Some((
                    ratio,
                    if ratio > 1. {
                        absolute - 100
                    } else {
                        100 - absolute
                    },
                ))
            }
            _ => None,
        };
        // `.size-direction` only shows when the size actually changed.
        let direction = percent.and_then(|(ratio, _)| (ratio != 1.).then_some(ratio < 1.));
        let fg = theme::WHITE.gamma_multiply(fade);
        let number_g = text(
            &percent.map_or(0, |(_, p)| p).to_string(),
            theme::numbers_font(theme::BIG_NUMBER),
            fg,
        );
        let percent_g = text(
            "%",
            FontId::proportional(theme::BODY),
            fg.gamma_multiply(0.76),
        );
        let direction_w = if direction.is_some() { 1.3 * REM } else { 0. };
        // `.percent-output` padding: 0.6rem on the arrow side, 1.1rem outside.
        let data_w = 0.6 * REM
            + direction_w
            + number_g.size().x
            + 0.2 * REM
            + percent_g.size().x
            + 1.1 * REM;
        let percent_w = ARROW + data_w;
        let box_w = SPEECH + size_w + GAP + percent_w + 1.;
        let box_h = FLAG_HEIGHT + 2.;

        // The blob sits on the window edge; the bubble overlaps it by 10px.
        let blob_rect = if theme.flip {
            Rect::from_center_size(Pos2::new(rect.right() - BLOB / 2., cy), Vec2::splat(BLOB))
        } else {
            Rect::from_center_size(Pos2::new(rect.left() + BLOB / 2., cy), Vec2::splat(BLOB))
        };
        let bubble = if theme.flip {
            let right = blob_rect.left() + 10.;
            Rect::from_min_max(
                Pos2::new(right - box_w, cy - box_h / 2.),
                Pos2::new(right, cy + box_h / 2.),
            )
        } else {
            let left = blob_rect.right() - 10.;
            Rect::from_min_max(
                Pos2::new(left, cy - box_h / 2.),
                Pos2::new(left + box_w, cy + box_h / 2.),
            )
        };
        // The tail lives in the first 11.4px of the speech padding.
        let body = if theme.flip {
            Rect::from_min_max(
                bubble.min,
                Pos2::new(bubble.right() - 11.4, bubble.bottom()),
            )
        } else {
            Rect::from_min_max(Pos2::new(bubble.left() + 11.4, bubble.top()), bubble.max)
        };
        theme::bubble_background(&painter, body, theme.flip);

        // Size read-out.
        let size_x = if theme.flip {
            bubble.left() + 1. + percent_w + GAP
        } else {
            bubble.left() + SPEECH
        };
        painter.galley(
            Pos2::new(size_x, cy - value_g.size().y / 2.),
            value_g.clone(),
            theme::WHITE,
        );
        if !unit.is_empty() {
            painter.galley(
                Pos2::new(size_x + value_g.size().x + 4., cy - unit_g.size().y / 2.),
                unit_g,
                theme::WHITE,
            );
        }

        // Percentage flag: the arrow points at the size, the data box beside it.
        let flag_top = cy - FLAG_HEIGHT / 2.;
        let flag_x = if theme.flip {
            bubble.left() + 1.
        } else {
            bubble.right() - 1. - percent_w
        };
        let (data, tip, base, radius) = if theme.flip {
            (
                Rect::from_min_size(Pos2::new(flag_x, flag_top), vec2(data_w, FLAG_HEIGHT)),
                flag_x + percent_w,
                flag_x + data_w,
                CornerRadius {
                    nw: 4,
                    sw: 4,
                    ne: 0,
                    se: 0,
                },
            )
        } else {
            (
                Rect::from_min_size(
                    Pos2::new(flag_x + ARROW, flag_top),
                    vec2(data_w, FLAG_HEIGHT),
                ),
                flag_x,
                flag_x + ARROW,
                CornerRadius {
                    nw: 0,
                    sw: 0,
                    ne: 4,
                    se: 4,
                },
            )
        };
        if !original {
            let fill = theme.main.gamma_multiply(fade);
            let arrow = |dx: f32, color: Color32| {
                painter.add(Shape::convex_polygon(
                    vec![
                        Pos2::new(tip + dx, cy),
                        Pos2::new(base + dx, flag_top),
                        Pos2::new(base + dx, flag_top + FLAG_HEIGHT),
                    ],
                    color,
                    Stroke::NONE,
                ));
            };
            // `filter: drop-shadow(∓1px 0 0 rgba(0, 0, 0, 0.67))`.
            arrow(
                if theme.flip { 1. } else { -1. },
                Color32::from_black_alpha(171),
            );
            arrow(0., fill);
            painter.rect_filled(data, radius, fill);
        }
        let shadow = Color32::from_black_alpha(77);
        let mut x = data.left() + if theme.flip { 1.1 * REM } else { 0.6 * REM };
        if let Some(smaller) = direction {
            theme::size_direction(
                &painter,
                Pos2::new(x + 0.55 * REM, cy + 3.),
                smaller,
                fg.gamma_multiply(0.76),
            );
            x += direction_w;
        }
        let number_pos = Pos2::new(x, cy - number_g.size().y / 2.);
        painter.galley(
            number_pos + vec2(0., 2.),
            text(
                &percent.map_or(0, |(_, p)| p).to_string(),
                theme::numbers_font(theme::BIG_NUMBER),
                shadow,
            ),
            shadow,
        );
        painter.galley(number_pos, number_g.clone(), fg);
        painter.galley(
            Pos2::new(x + number_g.size().x + 0.2 * REM, number_pos.y + 4.),
            percent_g,
            fg,
        );

        // The download blob doubles as the "export this side" control.
        let response = ui.interact(
            blob_rect.shrink(14.),
            ui.id().with(("download", index)),
            Sense::click(),
        );
        let exporting = settings.export_side == index;
        let color = if original { theme::BLACK } else { theme.hot };
        let grow = if response.hovered() { 1.06 } else { 1. };
        theme::draw_icon(
            &painter,
            Rect::from_center_size(blob_rect.center(), blob_rect.size() * grow),
            Icon::DownloadBlobs,
            color.gamma_multiply(if exporting || original { 1. } else { 0.6 }),
        );
        if loading {
            // `.download-disable`: the icon gives way to the spinner.
            theme::spinner(ui, blob_rect.center() + vec2(1., 0.), 13., theme::WHITE);
        } else {
            theme::draw_icon(
                &painter,
                Rect::from_center_size(blob_rect.center() + vec2(2., 2.), Vec2::splat(27.)),
                Icon::Download,
                theme::WHITE.gamma_multiply(if exporting || original { 1. } else { 0.8 }),
            );
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        let response = if original {
            response.on_hover_text("L’image originale ne s’exporte pas")
        } else if exporting {
            response.on_hover_text("Exporter ce côté dans le dossier de sortie")
        } else {
            response.on_hover_text("Choisir ce côté pour l’export, puis exporter")
        };
        if response.clicked() && !original {
            actions.push(Action::Export(index));
        }
        if let Some(error) = &self.preview_errors[index] {
            theme::truncated_text(
                &painter,
                Pos2::new(rect.left() + 8., rect.top()),
                error,
                FontId::proportional(theme::SMALL),
                theme::HOT_PINK,
                rect.width() - 16.,
                1,
            );
        }
    }

    // ------------------------------------------------- image list (desktop)

    /// The desktop-only image list, output folder and batch controls. It
    /// overlays the left options panel so the editor keeps the web layout.
    fn files_drawer(&mut self, ui: &mut egui::Ui) {
        if !self.drawer {
            return;
        }
        let ctx = &ui.ctx().clone();
        let width = DRAWER_WIDTH;
        let height = ui.available_height();
        let mut select = None;
        let mut remove = None;
        let mut inspect = Vec::new();
        let mut clear = false;
        egui::Panel::left("files-drawer")
            .exact_size(width)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                egui::Frame::NONE
                    .fill(theme::OFF_BLACK)
                    .corner_radius(CornerRadius {
                        nw: 0,
                        sw: 0,
                        ne: theme::RADIUS,
                        se: theme::RADIUS,
                    })
                    .show(ui, |ui| {
                        ui.set_min_size(vec2(width, height));
                        ui.set_max_size(vec2(width, height));
                        ui.spacing_mut().item_spacing.y = 0.;
                        let full = ui.max_rect();
                        // Room for the floating blob button in the corner.
                        ui.add_space(78.);
                        // Header, in the black "original image" title style.
                        theme::options_title(
                            ui,
                            &format!("Images · {}", self.entries.len()),
                            Theme::LEFT,
                            true,
                            CornerRadius {
                                nw: 0,
                                sw: 0,
                                ne: theme::RADIUS,
                                se: 0,
                            },
                            |ui| {
                                if theme::title_icon_button(
                                    ui,
                                    Icon::Trash,
                                    theme::WHITE,
                                    !self.batch && !self.entries.is_empty(),
                                )
                                .on_hover_text("Vider la liste")
                                .clicked()
                                    && !self.batch
                                {
                                    clear = true;
                                }
                                if theme::title_icon_button(ui, Icon::Folder, theme::WHITE, true)
                                    .on_hover_text("Ajouter un dossier")
                                    .clicked()
                                {
                                    self.dialog(1, ctx);
                                }
                                if theme::title_icon_button(ui, Icon::AddImage, theme::WHITE, true)
                                    .on_hover_text("Ajouter des images")
                                    .clicked()
                                {
                                    self.dialog(0, ctx);
                                }
                            },
                        );
                        theme::row_toggle(ui, "Inclure les sous-dossiers", |ui| {
                            theme::checkbox(ui, &mut self.recursive, Theme::LEFT);
                        });
                        theme::rule(ui);
                        // Which settings the panels on the right are editing.
                        theme::one_cell(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.;
                                let half = (ui.available_width() - 6.) / 2.;
                                let kind = |active: bool| {
                                    if active {
                                        theme::ButtonKind::Accent(theme::PINK)
                                    } else {
                                        theme::ButtonKind::Plain
                                    }
                                };
                                ui.scope(|ui| {
                                    ui.set_width(half);
                                    if theme::button(ui, "Communs", kind(!self.individual), true)
                                        .on_hover_text(
                                            "Les réglages s’appliquent à toutes les images",
                                        )
                                        .clicked()
                                    {
                                        self.individual = false;
                                    }
                                });
                                ui.scope(|ui| {
                                    ui.set_width(half);
                                    ui.add_enabled_ui(self.selected.is_some(), |ui| {
                                        if theme::button(
                                            ui,
                                            "Cette image",
                                            kind(self.individual),
                                            true,
                                        )
                                        .on_hover_text("Exceptions propres à l’image sélectionnée")
                                        .clicked()
                                        {
                                            self.individual = true;
                                        }
                                    });
                                });
                            });
                        });
                        if self.individual {
                            theme::one_cell(ui, |ui| {
                                if theme::button(
                                    ui,
                                    "Revenir aux réglages communs",
                                    theme::ButtonKind::Plain,
                                    true,
                                )
                                .clicked()
                                    && let Some(i) = self.selected
                                {
                                    self.entries[i].overrides = Overrides::default();
                                    self.individual = false;
                                    self.changed();
                                }
                            });
                        }
                        theme::rule(ui);

                        let footer_height = if self.batch { 178. } else { 152. };
                        let top = ui.cursor().top();
                        let list_rect = Rect::from_min_max(
                            Pos2::new(full.left(), top),
                            Pos2::new(full.right(), full.bottom() - footer_height),
                        );
                        let footer_rect = Rect::from_min_max(
                            Pos2::new(full.left(), full.bottom() - footer_height),
                            full.max,
                        );
                        let mut list = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(list_rect)
                                .id_salt("files-list")
                                .layout(Layout::top_down(Align::Min)),
                        );
                        let selected = self.selected;
                        let batch = self.batch;
                        egui::ScrollArea::vertical()
                            .id_salt("files-scroll")
                            .auto_shrink([false, false])
                            .show_rows(&mut list, 68., self.entries.len(), |ui, rows| {
                                for i in rows {
                                    let e = &self.entries[i];
                                    let (rect, response) = theme::block(ui, 68., Sense::click());
                                    let response =
                                        response.on_hover_text(e.path.display().to_string());
                                    let active = selected == Some(i);
                                    if active {
                                        ui.painter().rect_filled(
                                            rect,
                                            0.,
                                            Color32::from_white_alpha(18),
                                        );
                                        ui.painter().rect_filled(
                                            Rect::from_min_size(
                                                rect.left_top(),
                                                vec2(3., rect.height()),
                                            ),
                                            0.,
                                            theme::PINK,
                                        );
                                    } else if response.hovered() {
                                        ui.painter().rect_filled(
                                            rect,
                                            0.,
                                            Color32::from_white_alpha(10),
                                        );
                                    }
                                    let thumb = Rect::from_min_size(
                                        Pos2::new(rect.left() + 12., rect.top() + 13.),
                                        Vec2::splat(42.),
                                    );
                                    ui.painter().rect_filled(thumb, 3., theme::BLACK);
                                    if let Some(texture) = &e.thumbnail {
                                        ui.painter().image(
                                            texture.id(),
                                            thumb,
                                            Rect::from_min_max(Pos2::ZERO, Pos2::new(1., 1.)),
                                            Color32::WHITE,
                                        );
                                    }
                                    let name =
                                        e.path.file_name().unwrap_or_default().to_string_lossy();
                                    let text_left = thumb.right() + 10.;
                                    let max_w = rect.right() - text_left - 34.;
                                    theme::truncated_text(
                                        ui.painter(),
                                        Pos2::new(text_left, rect.top() + 8.),
                                        &name,
                                        FontId::proportional(theme::BODY),
                                        theme::WHITE,
                                        max_w,
                                        2,
                                    );
                                    let mut status = e.status.clone();
                                    if !e.overrides.0.is_empty() {
                                        status.push_str(" · réglages individuels");
                                    }
                                    theme::truncated_text(
                                        ui.painter(),
                                        Pos2::new(text_left, rect.bottom() - 21.),
                                        &status,
                                        FontId::proportional(theme::SMALL),
                                        theme::LESS_LIGHT_GRAY,
                                        max_w,
                                        1,
                                    );
                                    if e.warning.is_some() {
                                        theme::draw_icon(
                                            ui.painter(),
                                            Rect::from_center_size(
                                                Pos2::new(rect.right() - 40., rect.top() + 17.),
                                                Vec2::splat(14.),
                                            ),
                                            Icon::Warning,
                                            Color32::from_rgb(255, 200, 80),
                                        );
                                    }
                                    let close = Rect::from_center_size(
                                        Pos2::new(rect.right() - 18., rect.center().y),
                                        Vec2::splat(20.),
                                    );
                                    let close_response = ui.interact(
                                        close,
                                        ui.id().with(("remove", e.id)),
                                        Sense::click(),
                                    );
                                    if !batch {
                                        theme::draw_icon(
                                            ui.painter(),
                                            close.shrink(3.),
                                            Icon::Remove,
                                            if close_response.hovered() {
                                                theme::HOT_PINK
                                            } else {
                                                theme::LESS_LIGHT_GRAY
                                            },
                                        );
                                        if close_response.clicked() {
                                            remove = Some(i);
                                        }
                                    }
                                    if response.clicked() && !close_response.clicked() {
                                        select = Some(i);
                                    }
                                    if !e.inspected && !batch {
                                        inspect.push((e.id, e.path.clone()));
                                    }
                                }
                            });
                        if self.entries.is_empty() {
                            ui.painter().text(
                                list_rect.center(),
                                Align2::CENTER_CENTER,
                                "Déposez des images ici",
                                FontId::proportional(theme::BODY),
                                theme::LESS_LIGHT_GRAY,
                            );
                        }

                        let mut footer = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(footer_rect)
                                .id_salt("files-footer")
                                .layout(Layout::top_down(Align::Min)),
                        );
                        footer.painter().rect_filled(
                            footer_rect,
                            CornerRadius {
                                nw: 0,
                                ne: 0,
                                sw: 0,
                                se: theme::RADIUS,
                            },
                            theme::DARK_GRAY,
                        );
                        self.drawer_footer(&mut footer);
                    });
            });
        for (id, path) in inspect {
            if let Some(e) = self.entries.iter_mut().find(|e| e.id == id) {
                e.inspected = true;
            }
            self.command(Command::Inspect { id, path });
        }
        if clear {
            self.entries.clear();
            self.selected = None;
            self.previews = [None, None];
            self.thumbnail_lru.clear();
            self.changed();
        }
        if let Some(i) = remove {
            self.entries.remove(i);
            self.selected = None;
            if !self.entries.is_empty() {
                self.select(i.min(self.entries.len() - 1));
            } else {
                self.previews = [None, None];
                self.changed();
            }
        } else if let Some(i) = select {
            self.select(i);
        }
    }
    fn drawer_footer(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        theme::one_cell(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 6.;
            if theme::button(ui, "Dossier de sortie…", theme::ButtonKind::Plain, true).clicked() {
                self.dialog(2, &ctx);
            }
            let path = self
                .output
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "Aucun dossier sélectionné".into());
            ui.add(
                egui::Label::new(
                    egui::RichText::new(path)
                        .size(theme::SMALL)
                        .color(theme::LESS_LIGHT_GRAY),
                )
                .truncate(),
            );
            if self.batch {
                let (rect, _) = theme::block(ui, 8., Sense::hover());
                let track = Rect::from_min_size(
                    Pos2::new(rect.left(), rect.center().y - 3.),
                    vec2(rect.width(), 6.),
                );
                ui.painter()
                    .rect_filled(track, CornerRadius::same(3), theme::BLACK);
                let ratio = self.done as f32 / self.total.max(1) as f32;
                ui.painter().rect_filled(
                    Rect::from_min_size(track.min, vec2(track.width() * ratio, 6.)),
                    CornerRadius::same(3),
                    theme::PINK,
                );
                ui.label(
                    egui::RichText::new(format!(
                        "{} / {} fichiers · réglages figés au lancement",
                        self.done, self.total
                    ))
                    .size(theme::SMALL),
                );
                if theme::button(ui, "Annuler le lot", theme::ButtonKind::Plain, true).clicked() {
                    self.cancel.store(true, Ordering::Relaxed);
                    self.set_message("Annulation demandée ; attente du codec courant…");
                }
            } else {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.;
                    let half = (ui.available_width() - 6.) / 2.;
                    ui.scope(|ui| {
                        ui.set_width(half);
                        ui.add_enabled_ui(self.selected.is_some(), |ui| {
                            if theme::button(ui, "Exporter", theme::ButtonKind::Plain, true)
                                .on_hover_text(
                                    "Écrit l’image sélectionnée dans le dossier de sortie",
                                )
                                .clicked()
                            {
                                self.start(false);
                            }
                        });
                    });
                    ui.scope(|ui| {
                        ui.set_width(half);
                        ui.add_enabled_ui(!self.entries.is_empty(), |ui| {
                            if theme::button(
                                ui,
                                "Convertir le lot",
                                theme::ButtonKind::Accent(theme::HOT_PINK),
                                true,
                            )
                            .clicked()
                            {
                                self.start(true);
                            }
                        });
                    });
                });
            }
        });
    }

    /// `shared/custom-els/snack-bar` — transient status, bottom-left.
    fn snackbar(&self, ctx: &egui::Context, screen: Rect) {
        let warning = self.selected.and_then(|i| self.entries[i].warning.clone());
        let message = if self.message_at.elapsed() < Duration::from_secs(8) {
            self.message.as_str()
        } else {
            ""
        };
        if message.is_empty() && warning.is_none() {
            return;
        }
        let left = screen.left()
            + theme::OPTIONS_WIDTH
            + if self.drawer { DRAWER_WIDTH } else { 0. }
            + 12.;
        egui::Area::new(Id::new("snackbar"))
            .order(egui::Order::Foreground)
            .fixed_pos(Pos2::new(left, screen.bottom() - 64.))
            .show(ctx, |ui| {
                egui::Frame::NONE
                    .fill(theme::CONTROL_BG)
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_max_width(420.);
                        ui.spacing_mut().item_spacing.y = 3.;
                        if !message.is_empty() {
                            ui.label(egui::RichText::new(message).size(theme::BODY));
                        }
                        if let Some(warning) = warning {
                            ui.label(
                                egui::RichText::new(warning)
                                    .size(theme::SMALL)
                                    .color(Color32::from_rgb(255, 200, 80)),
                            );
                        }
                    });
            });
    }
    /// The blob button in the top-left corner (`.back` in the web app).
    fn blob_button(&mut self, ctx: &egui::Context, screen: Rect) {
        egui::Area::new(Id::new("drawer-blob"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.left_top() + vec2(14., 14.))
            .show(ctx, |ui| {
                let icon = if self.drawer { Icon::Close } else { Icon::Menu };
                if theme::blob_button(ui, 58., icon, theme::HOT_PINK)
                    .on_hover_text("Liste des images, dossier de sortie et conversion par lot")
                    .clicked()
                {
                    self.drawer = !self.drawer;
                }
            });
    }

    // --------------------------------------------------------------- pages

    /// The landing page, echoing the web intro: a blob over the light page.
    fn intro(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let rect = ui.max_rect();
        let checker = self.checker_texture(&ctx);
        ui.painter().image(
            checker.id(),
            rect,
            Rect::from_min_size(Pos2::ZERO, rect.size() / 20.),
            Color32::WHITE,
        );
        let blob_size = (rect.width() * 0.36).clamp(300., 460.);
        let center = Pos2::new(rect.center().x, rect.center().y + 30.);
        let time = ui.input(|i| i.time) as f32;
        theme::blob_animated(
            ui.painter(),
            Rect::from_center_size(center, Vec2::splat(blob_size)),
            theme::HOT_PINK.gamma_multiply(0.42),
            time,
        );
        // ~30 fps is plenty for the slow morph and keeps the idle cost low.
        ctx.request_repaint_after(Duration::from_millis(33));
        // `.logo`: the web wordmark, 189px wide.
        theme::draw_icon(
            ui.painter(),
            Rect::from_center_size(
                Pos2::new(rect.center().x, center.y - blob_size / 2. - 110.),
                vec2(189., 90.),
            ),
            Icon::Logo,
            Color32::WHITE,
        );
        ui.painter().text(
            Pos2::new(rect.center().x, center.y - blob_size / 2. - 56.),
            Align2::CENTER_CENTER,
            "Compression d’images locale · hors ligne",
            FontId::proportional(theme::BODY),
            theme::DIM_TEXT,
        );
        theme::draw_icon(
            ui.painter(),
            Rect::from_center_size(center - vec2(0., 62.), Vec2::splat(64.)),
            Icon::AddImage,
            theme::WHITE,
        );
        theme::strong_text(
            ui.painter(),
            center - vec2(0., 14.),
            Align2::CENTER_CENTER,
            "Déposez vos images ici",
            FontId::proportional(theme::BODY * 1.25),
            theme::WHITE,
        );
        let labels = ["Sélectionner des images", "Sélectionner un dossier"];
        let buttons_width: f32 = labels
            .iter()
            .map(|t| {
                ui.painter()
                    .layout_no_wrap((*t).into(), FontId::proportional(theme::BODY), theme::WHITE)
                    .size()
                    .x
                    + 36.
            })
            .sum::<f32>()
            + 10.;
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(Rect::from_center_size(
                    center + vec2(0., 52.),
                    vec2(buttons_width.max(260.), 90.),
                ))
                .id_salt("intro-actions")
                .layout(Layout::top_down(Align::Center)),
        );
        child.spacing_mut().item_spacing.y = 12.;
        child.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.;
            for (kind, label) in labels.into_iter().enumerate() {
                if theme::pill_button(ui, label, theme::WHITE, theme::HOT_PINK).clicked() {
                    self.dialog(kind as u8, &ctx);
                }
            }
        });
        child.horizontal(|ui| {
            let label_width = 150.;
            ui.add_space(((ui.available_width() - label_width - 25.) / 2.).max(0.));
            let intro_theme = Theme {
                main: theme::HOT_PINK,
                ..Theme::LEFT
            };
            theme::checkbox(ui, &mut self.recursive, intro_theme);
            ui.label(
                egui::RichText::new("Inclure les sous-dossiers")
                    .size(theme::BODY)
                    .color(theme::WHITE),
            );
        });
        if !self.message.is_empty() {
            ui.painter().text(
                Pos2::new(rect.center().x, rect.bottom() - 24.),
                Align2::CENTER_CENTER,
                &self.message,
                FontId::proportional(theme::BODY),
                theme::DIM_TEXT,
            );
        }
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let screen = ui.max_rect();
        // Registered before the panels so they take pointer priority.
        let response = ui.interact(screen, Id::new("viewport"), Sense::click_and_drag());
        self.viewport_input(ui, screen, &response);
        self.paint_viewport(ui, screen);

        let mut settings = if self.individual {
            self.resolved()
        } else {
            self.common.clone()
        };
        let before = settings.clone();
        let mut actions = Vec::new();
        let svg = self.selected.is_some_and(|i| {
            self.entries[i]
                .path
                .extension()
                .is_some_and(|s| s.eq_ignore_ascii_case("svg"))
        });
        let source_name = self
            .selected
            .map(|i| {
                self.entries[i]
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            })
            .unwrap_or_default();

        self.files_drawer(ui);
        egui::Panel::left("options-1")
            .exact_size(theme::OPTIONS_WIDTH)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                self.side_panel(ui, 0, &mut settings, svg, &source_name, &mut actions);
            });
        egui::Panel::right("options-2")
            .exact_size(theme::OPTIONS_WIDTH)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                self.side_panel(ui, 1, &mut settings, svg, &source_name, &mut actions);
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                self.viewport_controls(ui, &mut settings.rotation);
            });

        let mut export = false;
        for action in actions {
            match action {
                Action::Export(index) => {
                    settings.export_side = index;
                    export = true;
                }
                Action::CopySide(index) => {
                    let side = settings.sides[index].clone();
                    settings.sides[1 - index] = side;
                }
                Action::SaveSide(index) => self.dialog(3 + index as u8, &ctx),
                Action::LoadSide(index) => self.dialog(5 + index as u8, &ctx),
            }
        }
        self.apply(before, settings);
        if export {
            self.start(false);
        }
        self.blob_button(&ctx, screen);
        self.snackbar(&ctx, screen);
    }
}
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.events(&ctx);
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        if !dropped.is_empty() {
            self.command(Command::Import {
                paths: dropped,
                recursive: self.recursive,
            });
        }
        if self.entries.is_empty() {
            self.intro(ui);
        } else {
            self.editor(ui);
        }
        // The dashed drop outline of `.drop-valid`.
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            let rect = ui.max_rect().shrink(10.);
            ui.painter().extend(Shape::dashed_line(
                &[
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                    rect.left_top(),
                ],
                Stroke::new(2., theme::PINK),
                8.,
                6.,
            ));
        }
    }
}
impl Drop for App {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
fn main() -> eframe::Result<()> {
    // winit has no file drag-and-drop on native Wayland, only on X11, so
    // prefer XWayland when it is available. `SQUOOSH_WAYLAND=1` opts out.
    if std::env::var_os("DISPLAY").is_some()
        && std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var_os("SQUOOSH_WAYLAND").is_none()
    {
        // SAFETY: nothing else runs yet, so no thread reads the environment.
        unsafe { std::env::remove_var("WAYLAND_DISPLAY") };
    }
    let paths: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
    if paths.iter().any(|p| p == std::path::Path::new("--version")) {
        println!("Squoosh Desktop {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).ok();
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1480., 920.])
        .with_min_inner_size([1050., 650.])
        .with_app_id("squoosh-desktop");
    if let Some(icon) = icon {
        viewport = viewport.with_icon(icon);
    }
    let native = eframe::NativeOptions {
        viewport,
        #[cfg(windows)]
        renderer: eframe::Renderer::Wgpu,
        #[cfg(not(windows))]
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Squoosh Desktop",
        native,
        Box::new(move |cc| Ok(Box::new(App::new(cc, paths)))),
    )
}
