//! The web app's SVG icons, rasterised with resvg and cached as textures so
//! the desktop UI shows exactly the same artwork. Glyphs are drawn in white
//! and tinted when painted.
use crate::theme::Icon;
use eframe::egui::{self, Color32, ColorImage, Rect, TextureHandle, TextureOptions, vec2};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

/// Rasterised icons, keyed by icon and physical pixel size.
type Textures = HashMap<(Icon, [u32; 2]), TextureHandle>;

const FILL: &str = r##"xmlns="http://www.w3.org/2000/svg" fill="#fff""##;

/// SVG source for each icon; paths are copied from `lazy-app/icons` and the
/// other web components, desktop-only glyphs from Material Icons.
fn source(icon: Icon) -> String {
    let svg =
        |view_box: &str, body: &str| format!(r#"<svg {FILL} viewBox="{view_box}">{body}</svg>"#);
    let path = |d: &str| svg("0 0 24 24", &format!(r#"<path d="{d}"/>"#));
    let cog = |arrow: &str| {
        svg(
            "0 0 24 24",
            &format!(
                r##"<g fill="none" stroke="#fff" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="{arrow}"/><path d="M9 12a3 3 0 1 0 6 0a3 3 0 0 0-6 0"/></g>"##
            ),
        )
    };
    match icon {
        Icon::Minus => path("M19 13H5v-2h14v2z"),
        Icon::Plus => path("M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"),
        Icon::Rotate => path(
            "M15.6 5.5L11 1v3a8 8 0 0 0 0 16v-2a6 6 0 0 1 0-12v4l4.5-4.5zm4.3 5.5a8 8 0 0 0-1.6-3.9L17 8.5c.5.8.9 1.6 1 2.5h2zM13 17.9v2a8 8 0 0 0 3.9-1.6L15.5 17c-.8.5-1.6.9-2.5 1zm3.9-2.4l1.4 1.4A8 8 0 0 0 20 13h-2c-.1.9-.5 1.7-1 2.5z",
        ),
        Icon::Aliasing => svg(
            "0 0 24 24",
            r##"<circle cx="12" cy="12" r="8" fill="none" stroke="#fff" stroke-width="2"/>"##,
        ),
        Icon::AliasingActive => path(
            "M12 3h5v2h2v2h2v5h-2V9h-2V7h-2V5h-3V3M21 12v5h-2v2h-2v2h-5v-2h3v-2h2v-2h2v-3h2M12 21H7v-2H5v-2H3v-5h2v3h2v2h2v2h3v2M3 12V7h2V5h2V3h5v2H9v2H7v2H5v3H3",
        ),
        Icon::Background => path(
            "M3 13h2v-2H3v2zm0 4h2v-2H3v2zm2 4v-2H3c0 1.1.9 2 2 2zM3 9h2V7H3v2zm12 12h2v-2h-2v2zm4-18H9a2 2 0 0 0-2 2v10c0 1.1.9 2 2 2h10a2 2 0 0 0 2-2V5a2 2 0 0 0-2-2zm0 12H9V5h10v10zm-8 6h2v-2h-2v2zm-4 0h2v-2H7v2z",
        ),
        Icon::BackgroundActive => path(
            "M9 7H7v2h2V7zm0 4H7v2h2v-2zm0-8a2 2 0 0 0-2 2h2V3zm4 12h-2v2h2v-2zm6-12v2h2a2 2 0 0 0-2-2zm-6 0h-2v2h2V3zM9 17v-2H7c0 1.1.9 2 2 2zm10-4h2v-2h-2v2zm0-4h2V7h-2v2zm0 8a2 2 0 0 0 2-2h-2v2zM5 7H3v12c0 1.1.9 2 2 2h12v-2H5V7zm10-2h2V3h-2v2zm0 12h2v-2h-2v2z",
        ),
        Icon::Swap => svg(
            "0 0 18 14",
            r#"<path d="M5.5 3.6v6.8L2.1 7l3.4-3.4M7 0L0 7l7 7V0zm4 0v14l7-7-7-7z"/>"#,
        ),
        Icon::Save => cog(
            "M12.501 20.93c-.866.25-1.914-.166-2.176-1.247a1.724 1.724 0 0 0-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 0 0-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 0 0 1.066-2.573c-.94-1.543.826-3.31 2.37-2.37c1 .608 2.296.07 2.572-1.065c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.074.26 1.49 1.296 1.252 2.158M19 22v-6m3 3l-3-3l-3 3",
        ),
        Icon::Import => cog(
            "M12.52 20.924c-.87.262-1.93-.152-2.195-1.241a1.724 1.724 0 0 0-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 0 0-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 0 0 1.066-2.573c-.94-1.543.826-3.31 2.37-2.37c1 .608 2.296.07 2.572-1.065c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 0 0 2.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 0 0 1.065 2.572c1.088.264 1.502 1.323 1.242 2.192M19 16v6m3-3l-3 3l-3-3",
        ),
        Icon::Download => svg(
            "0 0 23.9 24.9",
            r#"<path d="M6.6 2.7h-4v13.2h2.7A2.7 2.7 0 018 18.6a2.7 2.7 0 002.6 2.6h2.7a2.7 2.7 0 002.6-2.6 2.7 2.7 0 012.7-2.7h2.6V2.7h-4a1.3 1.3 0 110-2.7h4A2.7 2.7 0 0124 2.7v18.5a2.7 2.7 0 01-2.7 2.7H2.7A2.7 2.7 0 010 21.2V2.7A2.7 2.7 0 012.7 0h4a1.3 1.3 0 010 2.7zm4 7.4V1.3a1.3 1.3 0 112.7 0v8.8L15 8.4a1.3 1.3 0 011.9 1.8l-4 4a1.3 1.3 0 01-1.9 0l-4-4A1.3 1.3 0 019 8.4z"/>"#,
        ),
        Icon::DownloadBlobs => svg(
            "0 0 89.6 86.9",
            r#"<path opacity=".7" d="M27.3 72c-8-4-15.6-12.3-16.9-21-1.2-8.7 4-17.8 10.5-26s14.4-15.6 24-16 21.2 6 28.6 16.5c7.4 10.5 10.8 25 6.6 34S64.1 71.8 54 73.6c-10.2 2-18.7 2.3-26.7-1.6z"/><path opacity=".7" d="M19.8 24.8c4.3-7.8 13-15 21.8-15.7 8.7-.8 17.5 4.8 25.4 11.8 7.8 6.9 14.8 15.2 14.7 24.9s-7.1 20.7-18 27.6c-10.8 6.8-25.5 9.5-34.2 4.8S18.1 61.6 16.7 51.4c-1.3-10.3-1.3-18.8 3-26.6z"/>"#,
        ),
        Icon::Checked => path(
            "M21.3 0H2.7A2.7 2.7 0 0 0 0 2.7v18.6A2.7 2.7 0 0 0 2.7 24h18.6a2.7 2.7 0 0 0 2.7-2.7V2.7A2.7 2.7 0 0 0 21.3 0zm-12 18.7L2.7 12l1.8-1.9L9.3 15 19.5 4.8l1.8 1.9z",
        ),
        Icon::Unchecked => path(
            "M21.3 2.7v18.6H2.7V2.7h18.6m0-2.7H2.7A2.7 2.7 0 0 0 0 2.7v18.6A2.7 2.7 0 0 0 2.7 24h18.6a2.7 2.7 0 0 0 2.7-2.7V2.7A2.7 2.7 0 0 0 21.3 0z",
        ),
        Icon::Arrow => svg(
            "0 -1.95 9.8 9.8",
            r#"<path d="M8.2.2a1 1 0 011.4 1.4l-4 4a1 1 0 01-1.4 0l-4-4A1 1 0 011.6.2l3.3 3.3L8.2.2z"/>"#,
        ),
        Icon::BackBlob => svg(
            "0 0 61 53.3",
            r#"<path d="M0 25.6c-.5-7.1 4.1-14.5 10-19.1S23.4.1 32.2 0c8.8 0 19 1.6 24.4 8s5.6 17.8 1.7 27a29.7 29.7 0 01-20.5 18c-8.4 1.5-17.3-2.6-24.5-8S.5 32.6.1 25.6z"/>"#,
        ),
        Icon::Close => svg(
            "0 0 61 53.3",
            r#"<path d="M41.6 17.1l-2-2.1-8.3 8.2-8.2-8.2-2 2 8.2 8.3-8.3 8.2 2.1 2 8.2-8.1 8.3 8.2 2-2-8.2-8.3z"/>"#,
        ),
        // Material "menu", placed like the back button's cross.
        Icon::Menu => svg(
            "0 0 61 53.3",
            r#"<path transform="translate(19.3 14.6) scale(1)" d="M3 18h18v-2H3v2zm0-5h18v-2H3v2zm0-7v2h18V6H3z"/>"#,
        ),
        Icon::AddImage => path(
            "M19 7v3h-2V7h-3V5h3V2h2v3h3v2h-3zm-3 4V8h-3V5H5a2 2 0 00-2 2v12c0 1.1.9 2 2 2h12a2 2 0 002-2v-8h-3zM5 19l3-4 2 3 3-4 4 5H5z",
        ),
        Icon::Folder => path(
            "M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z",
        ),
        Icon::Trash => {
            path("M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z")
        }
        Icon::Remove => path(
            "M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z",
        ),
        Icon::Warning => path("M1 21h22L12 2 1 21zm12-3h-2v-2h2v2zm0-4h-2v-4h2v4z"),
        Icon::Logo => include_str!("../assets/logo-with-text.svg").to_owned(),
    }
}

fn tree(icon: Icon) -> resvg::usvg::Tree {
    resvg::usvg::Tree::from_str(&source(icon), &resvg::usvg::Options::default())
        .expect("checked-in icon SVG")
}

/// Width / height of the icon's view box.
pub fn aspect(icon: Icon) -> f32 {
    static ASPECTS: LazyLock<Mutex<HashMap<Icon, f32>>> = LazyLock::new(Default::default);
    *ASPECTS.lock().unwrap().entry(icon).or_insert_with(|| {
        let size = tree(icon).size();
        size.width() / size.height()
    })
}

/// The icon rasterised at exactly `size` physical pixels.
pub fn texture(ctx: &egui::Context, icon: Icon, size: [u32; 2]) -> TextureHandle {
    static CACHE: LazyLock<Mutex<Textures>> = LazyLock::new(Default::default);
    CACHE
        .lock()
        .unwrap()
        .entry((icon, size))
        .or_insert_with(|| {
            let tree = tree(icon);
            let [w, h] = size.map(|v| v.max(1));
            let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).expect("icon size");
            let view = tree.size();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(
                    w as f32 / view.width(),
                    h as f32 / view.height(),
                ),
                &mut pixmap.as_mut(),
            );
            ctx.load_texture(
                format!("icon-{icon:?}-{w}x{h}"),
                ColorImage::from_rgba_premultiplied([w as usize, h as usize], pixmap.data()),
                TextureOptions::LINEAR,
            )
        })
        .clone()
}

/// The largest rect of the icon's aspect ratio centred in `rect`.
pub fn fit(icon: Icon, rect: Rect) -> Rect {
    let aspect = aspect(icon);
    let size = if rect.width() / rect.height() > aspect {
        vec2(rect.height() * aspect, rect.height())
    } else {
        vec2(rect.width(), rect.width() / aspect)
    };
    Rect::from_center_size(rect.center(), size)
}

/// Paints `icon` inside `rect`, tinted with `color`, optionally rotated.
pub fn paint(
    ui_ctx: &egui::Context,
    painter: &egui::Painter,
    rect: Rect,
    icon: Icon,
    color: Color32,
    angle: f32,
) {
    let rect = fit(icon, rect);
    let ppp = ui_ctx.pixels_per_point();
    let px = [
        (rect.width() * ppp).round() as u32,
        (rect.height() * ppp).round() as u32,
    ];
    let texture = texture(ui_ctx, icon, px);
    if angle == 0.0 {
        painter.image(
            texture.id(),
            rect,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            color,
        );
    } else {
        let mut mesh = egui::Mesh::with_texture(texture.id());
        mesh.add_rect_with_uv(
            rect,
            Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
            color,
        );
        mesh.rotate(egui::emath::Rot2::from_angle(angle), rect.center());
        painter.add(egui::Shape::mesh(mesh));
    }
}
