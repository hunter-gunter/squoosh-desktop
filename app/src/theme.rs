//! Visual system ported from the Squoosh web UI: palette, type scale and the
//! bespoke controls (toggles, ranges, selects, speech bubbles, blob buttons).
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Layout, Pos2, Rect, Response, Sense, Shape,
    Stroke, StrokeKind, TextStyle, Ui, Vec2, vec2,
};

// colors.css
pub const PINK: Color32 = Color32::from_rgb(255, 51, 133);
pub const HOT_PINK: Color32 = Color32::from_rgb(255, 0, 102);
pub const WHITE: Color32 = Color32::WHITE;
pub const BLACK: Color32 = Color32::BLACK;
pub const OFF_BLACK: Color32 = Color32::from_rgb(29, 29, 29);
pub const BLUE: Color32 = Color32::from_rgb(95, 180, 228);
pub const DEEP_BLUE: Color32 = Color32::from_rgb(0, 153, 255);
pub const LIGHT_BLUE: Color32 = Color32::from_rgb(118, 200, 255);
pub const LESS_LIGHT_GRAY: Color32 = Color32::from_rgb(188, 188, 188);
pub const MEDIUM_LIGHT_GRAY: Color32 = Color32::from_rgb(209, 209, 209);
pub const DARK_GRAY: Color32 = Color32::from_rgb(51, 51, 51);
pub const DARK_TEXT: Color32 = Color32::from_rgb(20, 38, 48);
pub const DIM_TEXT: Color32 = Color32::from_rgb(52, 58, 62);

/// `rgba(29, 29, 29, 0.92)` — the viewport control bar.
pub const CONTROL_BG: Color32 = Color32::from_rgba_premultiplied(27, 27, 27, 235);
pub const CONTROL_BG_HOVER: Color32 = Color32::from_rgba_premultiplied(46, 46, 46, 235);
pub const CONTROL_BG_ACTIVE: Color32 = Color32::from_rgba_premultiplied(66, 66, 66, 235);
pub const CONTROL_BORDER: Color32 = Color32::from_rgba_premultiplied(0, 0, 0, 171);
/// `rgba(30, 31, 29, 0.69)` — the results speech bubble.
pub const BUBBLE_BG: Color32 = Color32::from_rgba_premultiplied(21, 21, 20, 176);

/// The web sets `html { font: 12px/1.3 … }`, so 1rem is 12px.
pub const REM: f32 = 12.0;
pub const BODY: f32 = REM * 1.2;
pub const SMALL: f32 = REM;
pub const TITLE: f32 = REM * 1.4;
pub const BIG_NUMBER: f32 = REM * 2.6;
/// `--horizontal-padding` of the options panels.
pub const PAD_X: f32 = 15.0;
pub const PAD_Y: f32 = 10.0;
/// `--options-radius`
pub const RADIUS: u8 = 7;
pub const OPTIONS_WIDTH: f32 = 300.0;

/// Per-side accent colors (`.options-1-theme` / `.options-2-theme`).
#[derive(Clone, Copy, PartialEq)]
pub struct Theme {
    pub main: Color32,
    pub hot: Color32,
    pub header_text: Color32,
    /// The right-hand panel mirrors every asymmetric piece of chrome.
    pub flip: bool,
}
impl Theme {
    pub const LEFT: Self = Self {
        main: PINK,
        hot: HOT_PINK,
        header_text: WHITE,
        flip: false,
    };
    pub const RIGHT: Self = Self {
        main: BLUE,
        hot: DEEP_BLUE,
        header_text: DARK_TEXT,
        flip: true,
    };
    pub fn of(index: usize) -> Self {
        if index == 0 { Self::LEFT } else { Self::RIGHT }
    }
    /// Corner radius of the options scroller: rounded away from the window edge.
    pub fn scroller_radius(self) -> CornerRadius {
        if self.flip {
            CornerRadius {
                nw: RADIUS,
                ne: 0,
                sw: RADIUS,
                se: 0,
            }
        } else {
            CornerRadius {
                nw: 0,
                ne: RADIUS,
                sw: 0,
                se: RADIUS,
            }
        }
    }
}

pub fn install(ctx: &egui::Context) {
    let mut style = (*ctx.style_of(egui::Theme::Dark)).clone();
    style.visuals = egui::Visuals::dark();
    style.visuals.panel_fill = Color32::TRANSPARENT;
    style.visuals.window_fill = OFF_BLACK;
    style.visuals.extreme_bg_color = BLACK;
    style.visuals.override_text_color = Some(WHITE);
    style.visuals.window_corner_radius = CornerRadius::same(4);
    style.visuals.menu_corner_radius = CornerRadius::same(4);
    style.visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 12,
        spread: 0,
        color: Color32::from_black_alpha(140),
    };
    style.visuals.window_shadow = style.visuals.popup_shadow;
    style.visuals.selection.bg_fill = PINK;
    style.visuals.selection.stroke = Stroke::new(1.0, WHITE);

    for widget in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        widget.corner_radius = CornerRadius::same(4);
        widget.bg_stroke = Stroke::NONE;
        widget.fg_stroke = Stroke::new(1.0, WHITE);
        widget.expansion = 0.0;
    }
    style.visuals.widgets.noninteractive.bg_fill = BLACK;
    style.visuals.widgets.noninteractive.weak_bg_fill = BLACK;
    style.visuals.widgets.inactive.bg_fill = BLACK;
    style.visuals.widgets.inactive.weak_bg_fill = BLACK;
    style.visuals.widgets.hovered.bg_fill = DARK_GRAY;
    style.visuals.widgets.hovered.weak_bg_fill = DARK_GRAY;
    style.visuals.widgets.active.bg_fill = DARK_GRAY;
    style.visuals.widgets.active.weak_bg_fill = DARK_GRAY;
    style.visuals.widgets.open.bg_fill = BLACK;
    style.visuals.widgets.open.weak_bg_fill = BLACK;

    style.text_styles = [
        (TextStyle::Small, FontId::proportional(SMALL)),
        (TextStyle::Body, FontId::proportional(BODY)),
        (TextStyle::Button, FontId::proportional(BODY)),
        (TextStyle::Heading, FontId::proportional(TITLE)),
        (TextStyle::Monospace, FontId::monospace(BODY)),
    ]
    .into();
    style.spacing.item_spacing = vec2(8.0, 8.0);
    style.spacing.button_padding = vec2(8.0, 6.0);
    style.spacing.interact_size = vec2(24.0, 24.0);
    style.spacing.combo_width = 60.0;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating = true;
    style.spacing.menu_margin = egui::Margin::same(4);
    style.interaction.selectable_labels = false;

    // The results bubble uses Roboto Mono for its digits, like the web.
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "RobotoMonoNumbers".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/roboto-mono-numbers.ttf"
        ))),
    );
    let mut numbers = vec!["RobotoMonoNumbers".to_owned()];
    numbers.extend(fonts.families[&egui::FontFamily::Monospace].iter().cloned());
    fonts
        .families
        .insert(egui::FontFamily::Name("numbers".into()), numbers);
    ctx.set_fonts(fonts);

    ctx.set_style_of(egui::Theme::Dark, style.clone());
    ctx.set_style_of(egui::Theme::Light, style);
    ctx.set_theme(egui::Theme::Dark);
}

/// egui ships no bold face, so faux-bold by over-painting with a small offset.
pub fn strong_text(
    painter: &egui::Painter,
    pos: Pos2,
    anchor: Align2,
    text: &str,
    font: FontId,
    color: Color32,
) -> Rect {
    let rect = painter.text(pos, anchor, text, font.clone(), color);
    painter.text(pos + vec2(0.45, 0.0), anchor, text, font, color);
    rect
}

// ---------------------------------------------------------------- icons

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Icon {
    Plus,
    Minus,
    Rotate,
    Aliasing,
    AliasingActive,
    Background,
    BackgroundActive,
    Swap,
    Save,
    Import,
    Download,
    DownloadBlobs,
    Checked,
    Unchecked,
    Arrow,
    BackBlob,
    Close,
    Menu,
    AddImage,
    Folder,
    Trash,
    Remove,
    Warning,
    Logo,
}

/// Draws `icon` inside `rect` using the web app's own SVG artwork.
pub fn draw_icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    crate::icons::paint(painter.ctx(), painter, rect, icon, color, 0.0);
}

// ---------------------------------------------------------------- controls

/// Full-width row, ignoring item spacing, like a CSS block element.
pub fn block(ui: &mut Ui, height: f32, sense: Sense) -> (Rect, Response) {
    let width = ui.available_width();
    ui.allocate_exact_size(vec2(width, height), sense)
}

/// `.options-title` — the sticky, theme-colored heading of a panel section.
pub fn options_title(
    ui: &mut Ui,
    text: &str,
    theme: Theme,
    original: bool,
    radius: CornerRadius,
    buttons: impl FnOnce(&mut Ui),
) {
    options_title_indented(ui, text, theme, original, radius, 0.0, buttons);
}

/// As [`options_title`], with extra space before the heading.
pub fn options_title_indented(
    ui: &mut Ui,
    text: &str,
    theme: Theme,
    original: bool,
    radius: CornerRadius,
    indent: f32,
    buttons: impl FnOnce(&mut Ui),
) {
    let height = TITLE * 1.3 + PAD_Y * 2.0;
    let (rect, _) = block(ui, height, Sense::hover());
    let (fill, fg) = if original {
        (BLACK, WHITE)
    } else {
        (theme.main, theme.header_text)
    };
    let radius = CornerRadius {
        nw: radius.nw,
        ne: radius.ne,
        sw: 0,
        se: 0,
    };
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, OFF_BLACK),
    );
    strong_text(
        ui.painter(),
        rect.left_center() + vec2(PAD_X + indent, 0.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(TITLE),
        fg,
    );
    let buttons_rect = Rect::from_min_max(
        Pos2::new(rect.right() - 150.0, rect.top()),
        rect.right_bottom() - vec2(PAD_X - 2.0, 0.0),
    );
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(buttons_rect)
            .layout(Layout::right_to_left(Align::Center)),
    );
    child.spacing_mut().item_spacing.x = 1.1 * REM;
    child.visuals_mut().override_text_color = Some(fg);
    buttons(&mut child);
}

/// `.title-button` — a bare icon button tinted with the header text color.
pub fn title_icon_button(ui: &mut Ui, icon: Icon, color: Color32, enabled: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(20.0), Sense::click());
    let color = if enabled {
        if response.hovered() {
            color
        } else {
            color.gamma_multiply(0.86)
        }
    } else {
        color.gamma_multiply(0.5)
    };
    draw_icon(ui.painter(), rect, icon, color);
    if response.hovered() && enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// `.section-enabler` — a dark bar with the section name and its toggle.
pub fn section_enabler(ui: &mut Ui, text: &str, on: &mut bool, theme: Theme) -> Response {
    let height = BODY * 1.3 + 30.0;
    let (rect, response) = block(ui, height, Sense::click());
    ui.painter().rect_filled(rect, 0.0, DARK_GRAY);
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, OFF_BLACK),
    );
    ui.painter().text(
        rect.left_center() + vec2(PAD_X, 0.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(BODY),
        WHITE,
    );
    let knob = Rect::from_center_size(
        Pos2::new(rect.right() - PAD_X - 22.0, rect.center().y),
        vec2(44.0, 20.0),
    );
    let mut toggled = response.clicked();
    let inner = ui.interact(knob, response.id.with("knob"), Sense::click());
    toggled |= inner.clicked();
    if toggled {
        *on = !*on;
    }
    paint_toggle(ui, knob, *on, theme, response.id);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let mut response = response;
    if toggled {
        response.mark_changed();
    }
    response
}

fn paint_toggle(ui: &Ui, rect: Rect, on: bool, theme: Theme, id: egui::Id) {
    let how = ui.ctx().animate_bool_responsive(id, on);
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(10), BLACK);
    let center = Pos2::new(
        egui::lerp((rect.left() + 10.0)..=(rect.right() - 10.0), how),
        rect.center().y,
    );
    painter.circle_filled(center, 7.0, LESS_LIGHT_GRAY);
    if how > 0.0 {
        painter.circle_filled(center, 7.0, theme.main.gamma_multiply(how));
    }
}

/// `Options/Checkbox` — the square, theme-filled checkbox.
pub fn checkbox(ui: &mut Ui, checked: &mut bool, theme: Theme) -> Response {
    let (rect, mut response) = ui.allocate_exact_size(Vec2::splat(17.0), Sense::click());
    let enabled = ui.is_enabled();
    if response.clicked() && enabled {
        *checked = !*checked;
        response.mark_changed();
    }
    // `Options/Checkbox`: filled theme glyph when checked, outline otherwise.
    let (icon, color) = match (*checked, enabled) {
        (true, true) => (Icon::Checked, theme.main),
        (true, false) => (Icon::Checked, DARK_GRAY),
        (false, true) => (Icon::Unchecked, WHITE),
        (false, false) => (Icon::Unchecked, DARK_GRAY),
    };
    draw_icon(ui.painter(), rect, icon, color);
    if response.hovered() && enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// `.option-text-first` — `87px` label column, then the control.
pub fn row_text_first<R>(ui: &mut Ui, label: &str, content: impl FnOnce(&mut Ui) -> R) -> R {
    let mut out = None;
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = vec2(0.7 * REM, 0.0);
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            vec2(width, 0.0),
            Layout::left_to_right(Align::Center),
            |ui| {
                ui.add_space(PAD_X);
                ui.add_space(0.0);
                let (label_rect, _) = ui.allocate_exact_size(vec2(87.0, 26.0), Sense::hover());
                ui.painter().text(
                    label_rect.left_center(),
                    Align2::LEFT_CENTER,
                    label,
                    FontId::proportional(BODY),
                    ui.visuals().text_color(),
                );
                let remaining = ui.available_width() - PAD_X;
                out = Some(
                    ui.allocate_ui_with_layout(
                        vec2(remaining.max(10.0), 0.0),
                        Layout::left_to_right(Align::Center),
                        content,
                    )
                    .inner,
                );
            },
        );
    });
    out.expect("content ran")
}

/// `.option-toggle` — label filling the row, control pinned to the end.
pub fn row_toggle<R>(ui: &mut Ui, label: &str, content: impl FnOnce(&mut Ui) -> R) -> R {
    let mut out = None;
    ui.scope(|ui| {
        let width = ui.available_width();
        ui.allocate_ui_with_layout(
            vec2(width, 0.0),
            Layout::right_to_left(Align::Center),
            |ui| {
                ui.add_space(PAD_X);
                out = Some(content(ui));
                ui.add_space(0.7 * REM);
                ui.allocate_ui_with_layout(
                    vec2(ui.available_width() - PAD_X, 26.0),
                    Layout::left_to_right(Align::Center),
                    |ui| {
                        ui.label(egui::RichText::new(label).size(BODY));
                    },
                );
            },
        );
    });
    out.expect("content ran")
}

/// `.option-one-cell` — a single full-width cell with the panel padding.
pub fn one_cell<R>(ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
    let width = ui.available_width();
    ui.scope(|ui| {
        egui::Frame::NONE
            .inner_margin(egui::Margin {
                left: PAD_X as i8,
                right: PAD_X as i8,
                top: PAD_Y as i8,
                bottom: PAD_Y as i8,
            })
            .show(ui, |ui| {
                ui.set_width(width - PAD_X * 2.0);
                content(ui)
            })
            .inner
    })
    .inner
}

/// `.options-section` — the off-black body behind a group of options.
pub fn section<R>(ui: &mut Ui, content: impl FnOnce(&mut Ui) -> R) -> R {
    let start = ui.cursor().top();
    let bg = ui.painter().add(Shape::Noop);
    let out = content(ui);
    let end = ui.cursor().top();
    let rect = Rect::from_min_max(
        Pos2::new(ui.max_rect().left(), start),
        Pos2::new(ui.max_rect().right(), end),
    );
    ui.painter()
        .set(bg, egui::epaint::RectShape::filled(rect, 0.0, OFF_BLACK));
    out
}

/// `range-input` — a 2px track with a filled lead-in and a round thumb.
pub fn range(
    ui: &mut Ui,
    label: &str,
    value: &mut f64,
    bounds: std::ops::RangeInclusive<f64>,
    step: f64,
    decimals: usize,
    theme: Theme,
) -> bool {
    let (min, max) = (*bounds.start(), *bounds.end());
    let enabled = ui.is_enabled();
    let mut changed = false;
    let width = ui.available_width();
    ui.allocate_ui_with_layout(
        vec2(width, 0.0),
        Layout::left_to_right(Align::Center),
        |ui| {
            ui.label(egui::RichText::new(label).size(BODY));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.visuals_mut().widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                ui.visuals_mut().widgets.hovered.weak_bg_fill = Color32::TRANSPARENT;
                ui.visuals_mut().widgets.active.weak_bg_fill = Color32::TRANSPARENT;
                let response = ui.add(
                    egui::DragValue::new(value)
                        .range(min..=max)
                        .speed(step.max(0.01))
                        .max_decimals(decimals)
                        .min_decimals(decimals),
                );
                changed |= response.changed();
                // The dotted, theme-colored underline of `.text-input`.
                let r = response.rect;
                ui.painter().extend(Shape::dashed_line(
                    &[
                        Pos2::new(r.left(), r.bottom()),
                        Pos2::new(r.right(), r.bottom()),
                    ],
                    Stroke::new(1.0, theme.main),
                    2.0,
                    2.0,
                ));
            });
        },
    );
    let (rect, response) = ui.allocate_exact_size(vec2(width, 18.0), Sense::click_and_drag());
    if enabled
        && (response.dragged() || response.clicked())
        && let Some(pos) = response.interact_pointer_pos()
    {
        let t = ((pos.x - rect.left() - 6.0) / (rect.width() - 12.0)).clamp(0.0, 1.0) as f64;
        let mut new = min + t * (max - min);
        if step > 0.0 {
            new = (new / step).round() * step;
        }
        if new != *value {
            *value = new;
            changed = true;
        }
    }
    let t = if max > min {
        ((*value - min) / (max - min)).clamp(0.0, 1.0) as f32
    } else {
        0.0
    };
    let (track, thumb) = if enabled {
        (MEDIUM_LIGHT_GRAY, theme.main)
    } else {
        (DARK_GRAY, LESS_LIGHT_GRAY)
    };
    let painter = ui.painter();
    let line = Rect::from_min_size(
        Pos2::new(rect.left(), rect.top() + 8.0),
        vec2(rect.width(), 2.0),
    );
    painter.rect_filled(line, CornerRadius::same(1), track);
    if t > 0.0 {
        painter.rect_filled(
            Rect::from_min_size(line.min, vec2(line.width() * t, 2.0)),
            CornerRadius::same(1),
            thumb,
        );
    }
    let center = Pos2::new(
        rect.left() + 6.0 + t * (rect.width() - 12.0),
        rect.top() + 9.0,
    );
    painter.circle_filled(center, 6.0, thumb);
    if response.hovered() && enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    changed
}

/// `Options/Select` — a flat dark dropdown with a white chevron.
pub fn select<R>(
    ui: &mut Ui,
    id_salt: impl std::hash::Hash + std::fmt::Debug,
    selected: &str,
    large: bool,
    contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let width = ui.available_width();
    let (fill, padding) = if large {
        (DARK_GRAY, vec2(10.0, 10.0))
    } else {
        (BLACK, vec2(10.0, 7.0))
    };
    let mut out = None;
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = padding;
        ui.spacing_mut().icon_width = 10.0;
        ui.spacing_mut().icon_spacing = 8.0;
        {
            let widgets = &mut ui.visuals_mut().widgets;
            for w in [
                &mut widgets.inactive,
                &mut widgets.hovered,
                &mut widgets.active,
                &mut widgets.open,
            ] {
                w.weak_bg_fill = fill;
                w.bg_fill = fill;
            }
        }
        ui.visuals_mut().widgets.hovered.weak_bg_fill = fill.gamma_multiply(1.35);
        let inner = egui::ComboBox::from_id_salt(id_salt)
            .icon(|ui, rect, visuals, _open| {
                let glyph = Rect::from_center_size(
                    Pos2::new(rect.right() - 5.0, rect.center().y),
                    Vec2::splat(10.0),
                );
                draw_icon(ui.painter(), glyph, Icon::Arrow, visuals.fg_stroke.color);
            })
            .selected_text(egui::RichText::new(selected).size(BODY))
            // Long labels (e.g. file names) are cut with "…" instead of
            // pushing the dropdown out of its panel.
            .truncate()
            .width(width - padding.x * 2.0 - 24.0)
            .height(420.0)
            .show_ui(ui, |ui| {
                ui.spacing_mut().button_padding = vec2(8.0, 6.0);
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                contents(ui)
            });
        inner.response.on_hover_text(selected);
        out = inner.inner;
    });
    out
}

/// `.text-field` — the black rounded numeric field used by Resize.
pub fn text_field(ui: &mut Ui, value: &mut u32, max: u32) -> Response {
    let width = ui.available_width();
    ui.scope(|ui| {
        ui.visuals_mut().widgets.inactive.weak_bg_fill = BLACK;
        ui.visuals_mut().widgets.hovered.weak_bg_fill = DARK_GRAY;
        ui.visuals_mut().widgets.active.weak_bg_fill = DARK_GRAY;
        ui.spacing_mut().button_padding = vec2(10.0, 6.0);
        ui.add_sized(
            vec2(width, 26.0),
            egui::DragValue::new(value).range(1..=max).speed(1.0),
        )
    })
    .inner
}

// ---------------------------------------------------------------- buttons

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonKind {
    /// `.button` from the viewport controls.
    Plain,
    /// Filled with the side accent color.
    Accent(Color32),
}

/// A Squoosh-style flat button, used by the desktop-only panels.
pub fn button(ui: &mut Ui, text: &str, kind: ButtonKind, full_width: bool) -> Response {
    let font = FontId::proportional(BODY);
    let galley = ui.painter().layout_no_wrap(text.into(), font, WHITE);
    let width = if full_width {
        ui.available_width()
    } else {
        galley.size().x + 24.0
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), Sense::click());
    let enabled = ui.is_enabled();
    let fill = match kind {
        _ if !enabled => DARK_GRAY.gamma_multiply(0.6),
        ButtonKind::Accent(color) if response.is_pointer_button_down_on() => {
            color.gamma_multiply(0.8)
        }
        ButtonKind::Accent(color) if response.hovered() => color.gamma_multiply(1.15),
        ButtonKind::Accent(color) => color,
        ButtonKind::Plain if response.is_pointer_button_down_on() => CONTROL_BG_ACTIVE,
        ButtonKind::Plain if response.hovered() => CONTROL_BG_HOVER,
        ButtonKind::Plain => CONTROL_BG,
    };
    ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
    let fg = match kind {
        ButtonKind::Accent(color) if color == BLUE || color == LIGHT_BLUE => DARK_TEXT,
        _ => WHITE,
    };
    ui.painter().galley(
        rect.center() - galley.size() / 2.0,
        galley,
        if enabled { fg } else { LESS_LIGHT_GRAY },
    );
    if response.hovered() && enabled {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// Height and width of a square `.button` in the viewport control bar.
pub const BUTTON_SIZE: f32 = 39.0;
/// `.zoom { width: 7rem }`
pub const ZOOM_WIDTH: f32 = REM * 7.0;

/// One segment of `.button-group`: joined pill buttons over the viewport.
pub fn group_button(
    ui: &mut Ui,
    icon: Option<Icon>,
    text: Option<&str>,
    first: bool,
    last: bool,
    active: bool,
) -> Response {
    let font = FontId::proportional(BODY);
    let galley = text.map(|t| ui.painter().layout_no_wrap(t.into(), font, WHITE));
    // `.button` is square; `.zoom` is a fixed 7rem field.
    let width = if galley.is_some() {
        ZOOM_WIDTH
    } else {
        BUTTON_SIZE
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, BUTTON_SIZE), Sense::click());
    let fill = if active {
        CONTROL_BG_ACTIVE
    } else if response.hovered() {
        CONTROL_BG_HOVER
    } else {
        CONTROL_BG
    };
    let radius = CornerRadius {
        nw: if first { 6 } else { 0 },
        sw: if first { 6 } else { 0 },
        ne: if last { 6 } else { 0 },
        se: if last { 6 } else { 0 },
    };
    ui.painter().rect_filled(rect, radius, fill);
    ui.painter().rect_stroke(
        rect,
        radius,
        Stroke::new(1.0, CONTROL_BORDER),
        StrokeKind::Inside,
    );
    if let Some(galley) = galley {
        ui.painter()
            .galley(rect.center() - galley.size() / 2.0, galley, WHITE);
    } else if let Some(icon) = icon {
        draw_icon(
            ui.painter(),
            Rect::from_center_size(rect.center(), Vec2::splat(22.0)),
            icon,
            WHITE,
        );
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// The top-left blob button (`.back` in the web app): its blob, then the glyph.
pub fn blob_button(ui: &mut Ui, size: f32, icon: Icon, color: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let grow = if response.hovered() { 1.06 } else { 1.0 };
    let rect = Rect::from_center_size(rect.center(), rect.size() * grow);
    draw_icon(
        ui.painter(),
        rect,
        Icon::BackBlob,
        color.gamma_multiply(0.77),
    );
    draw_icon(ui.painter(), rect, icon, WHITE);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// `.bubble::before` — rounded body with a small tail on the `flip` side.
/// `rect` is the body; the tail sticks 11px out of it.
pub fn bubble_background(painter: &egui::Painter, rect: Rect, flip: bool) {
    let border = Color32::from_black_alpha(59);
    let y = rect.center().y + 3.0;
    let (tip, base) = if flip {
        (rect.right() + 11.4, rect.right() - 0.5)
    } else {
        (rect.left() - 11.4, rect.left() + 0.5)
    };
    let tail = vec![
        Pos2::new(base, y - 7.5),
        Pos2::new(tip, y),
        Pos2::new(base, y),
    ];
    painter.rect_filled(rect, CornerRadius::same(5), BUBBLE_BG);
    painter.rect_stroke(
        rect,
        CornerRadius::same(5),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
    painter.add(Shape::convex_polygon(tail.clone(), BUBBLE_BG, Stroke::NONE));
    painter.add(Shape::line(tail, Stroke::new(1.0, border)));
}

/// Formats bytes the way `Results/pretty-bytes.ts` does.
pub fn pretty_bytes(bytes: u64) -> (String, &'static str) {
    // Decimal units and three significant digits (`toPrecision(3)`).
    const UNITS: [&str; 5] = ["B", "kB", "MB", "GB", "TB"];
    if bytes < 1 {
        return ("0".into(), UNITS[0]);
    }
    let exponent = ((bytes as f64).log10() / 3.0).floor().min(4.0) as i32;
    let value = bytes as f64 / 1000f64.powi(exponent);
    let text = if exponent == 0 || value >= 100.0 {
        format!("{value:.0}")
    } else if value >= 10.0 {
        format!("{value:.1}")
    } else {
        format!("{value:.2}")
    };
    (text, UNITS[exponent as usize])
}

/// `.option-reveal` — the row that folds the advanced settings open.
pub fn revealer(ui: &mut Ui, text: &str, open: &mut bool) -> Response {
    let height = BODY * 1.3 + PAD_Y * 2.0;
    let (rect, mut response) = block(ui, height, Sense::click());
    if response.clicked() {
        *open = !*open;
        response.mark_changed();
    }
    let painter = ui.painter();
    if response.hovered() {
        painter.rect_filled(rect, 0.0, Color32::from_white_alpha(34));
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    painter.line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(1.0, Color32::from_white_alpha(68)),
    );
    // A chevron that rotates from "pointing right" to "pointing down".
    let how = ui.ctx().animate_bool_responsive(response.id, *open);
    let angle = (1.0 - how) * -std::f32::consts::FRAC_PI_2;
    let center = Pos2::new(rect.left() + PAD_X + 5.0, rect.center().y);
    crate::icons::paint(
        ui.ctx(),
        painter,
        Rect::from_center_size(center, Vec2::splat(10.0)),
        Icon::Arrow,
        WHITE,
        angle,
    );
    painter.text(
        Pos2::new(rect.left() + PAD_X + 16.0 + REM, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(BODY),
        WHITE,
    );
    response
}

/// A thin separator matching the 1px `--off-black` rules between sections.
pub fn rule(ui: &mut Ui) {
    let (rect, _) = block(ui, 1.0, Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, Color32::from_white_alpha(28));
}

/// The bold `↓` / `↑` of `.size-direction`, drawn because the bundled font
/// has no arrows. `center` is the middle of the glyph.
pub fn size_direction(painter: &egui::Painter, center: Pos2, down: bool, color: Color32) {
    let dir = if down { 1.0 } else { -1.0 };
    let shadow = Color32::from_black_alpha(77);
    for (offset, color) in [(vec2(0.0, 2.0), shadow), (Vec2::ZERO, color)] {
        let c = center + offset;
        painter.rect_filled(
            Rect::from_center_size(c - vec2(0.0, 2.5 * dir), vec2(3.0, 9.0)),
            0.0,
            color,
        );
        painter.add(Shape::convex_polygon(
            vec![
                c + vec2(0.0, 8.0 * dir),
                c + vec2(-5.5, 1.5 * dir),
                c + vec2(5.5, 1.5 * dir),
            ],
            color,
            Stroke::NONE,
        ));
    }
}

/// `shared/custom-els/loading-spinner` — a rotating arc.
pub fn spinner(ui: &Ui, center: Pos2, radius: f32, color: Color32) {
    let t = ui.input(|i| i.time) as f32;
    // The arc both spins and breathes, like the web element.
    let start = t * 5.0;
    let sweep = std::f32::consts::PI * (1.0 + 0.5 * (t * 2.2).sin());
    let points: Vec<Pos2> = (0..=32)
        .map(|i| {
            let a = start + sweep * i as f32 / 32.0;
            center + vec2(a.cos(), a.sin()) * radius
        })
        .collect();
    ui.painter()
        .add(Shape::line(points, Stroke::new(radius * 0.28, color)));
    ui.ctx().request_repaint();
}

/// The intro blob: layered lobes that slowly morph and turn, after
/// `Intro/blob-anim`.
pub fn blob_animated(painter: &egui::Painter, rect: Rect, color: Color32, time: f32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) / 2.0;
    for (layer, (scale, speed, phase)) in
        [(1.0, 0.18, 0.0_f32), (0.93, -0.23, 2.1), (0.84, 0.29, 4.0)]
            .into_iter()
            .enumerate()
    {
        let turn = time * speed;
        let morph = time * (0.6 + layer as f32 * 0.17);
        let points: Vec<Pos2> = (0..72)
            .map(|i| {
                let a = std::f32::consts::TAU * i as f32 / 72.0;
                let wobble = 1.0
                    + 0.075 * (3.0 * a + phase + morph).sin()
                    + 0.045 * (5.0 * a - phase * 1.7 - morph * 1.3).sin()
                    + 0.025 * (2.0 * a + morph * 0.7).sin();
                let (sn, cs) = (a + turn).sin_cos();
                c + vec2(cs, sn) * r * scale * wobble
            })
            .collect();
        painter.add(Shape::convex_polygon(points, color, Stroke::NONE));
    }
}

/// A rounded pill button, used over the pink intro blob.
pub fn pill_button(ui: &mut Ui, text: &str, fill: Color32, fg: Color32) -> Response {
    let font = FontId::proportional(BODY);
    let galley = ui.painter().layout_no_wrap(text.into(), font, fg);
    let size = vec2(galley.size().x + 36.0, 38.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let how = ui
        .ctx()
        .animate_bool_responsive(response.id, response.hovered());
    let rect = rect.expand(how * 1.5);
    ui.painter().rect_filled(
        rect.translate(vec2(0.0, 2.0)),
        CornerRadius::same(19),
        Color32::from_black_alpha(40),
    );
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(19),
        if response.is_pointer_button_down_on() {
            fill.gamma_multiply(0.9)
        } else {
            fill
        },
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, fg);
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    response
}

/// Text wrapped on at most `max_rows` lines and cut with an ellipsis, so it
/// never spills past `max_width`.
pub fn truncated_text(
    painter: &egui::Painter,
    pos: Pos2,
    text: &str,
    font: FontId,
    color: Color32,
    max_width: f32,
    max_rows: usize,
) -> Rect {
    let mut job = egui::text::LayoutJob::simple_singleline(text.into(), font, color);
    job.wrap = egui::text::TextWrapping {
        max_rows,
        break_anywhere: max_rows == 1,
        ..egui::text::TextWrapping::truncate_at_width(max_width.max(10.0))
    };
    let galley = painter.layout_job(job);
    let rect = Rect::from_min_size(pos, galley.size());
    painter.galley(pos, galley, color);
    rect
}

/// Roboto Mono (digits only, as shipped by the web app), for big numbers.
pub fn numbers_font(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("numbers".into()))
}
