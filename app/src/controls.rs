//! The contents of an options panel, laid out like `Compress/Options` on the web:
//! an "Edit" block (resize + palette) followed by "Compress" (codec + settings).
use crate::theme::{self, Theme};
use eframe::egui::{self, Ui};
use squoosh_codecs::{Format, Options, specs};
use squoosh_core::{
    pipeline::target_dimensions,
    settings::{Filter, Side},
};

fn active(f: Format, key: &str, o: &Options) -> bool {
    match f {
        Format::Jpeg => match key {
            "progressive" => o["baseline"] == 0,
            "optimize_coding" => o["arithmetic"] == 0,
            "auto_subsample" | "separate_chroma_quality" => o["color_space"] == 3,
            "chroma_subsample" => o["color_space"] == 3 && o["auto_subsample"] == 0,
            "chroma_quality" => o["color_space"] == 3 && o["separate_chroma_quality"] != 0,
            "trellis_opt_zero" => o["trellis_multipass"] != 0,
            _ => true,
        },
        Format::Webp => match key {
            "use_delta_palette" => false,
            "near_lossless" | "image_hint" => o["lossless"] != 0,
            "filter_strength" => o["lossless"] == 0 && o["autofilter"] == 0,
            "sns_strength" | "filter_sharpness" | "filter_type" | "partitions" | "segments"
            | "pass" | "preprocessing" | "autofilter" | "partition_limit" | "use_sharp_yuv"
            | "alpha_compression" => o["lossless"] == 0,
            "alpha_quality" | "alpha_filtering" => {
                o["lossless"] == 0 && o["alpha_compression"] != 0
            }
            _ => true,
        },
        Format::Avif => match key {
            "subsample" | "qualityAlpha" | "denoiseLevel" | "chromaDeltaQ" | "sharpness"
            | "tune" => o["quality"] != 100,
            _ => true,
        },
        _ => true,
    }
}

/// The label shown in the codec dropdown, matching the web encoder names.
pub fn format_label(format: Option<Format>) -> &'static str {
    match format {
        None => "Image originale",
        Some(Format::Jpeg) => "MozJPEG",
        Some(Format::Png) => "OxiPNG",
        Some(Format::Webp) => "WebP",
        Some(Format::Avif) => "AVIF",
    }
}

fn option(ui: &mut Ui, f: Format, key: &str, o: &mut Options, theme: Theme) {
    let s = specs(f).iter().find(|s| s.key == key).unwrap();
    let enabled = active(f, key, o);
    let value = o.get_mut(key).unwrap();
    ui.push_id(key, |ui| {
        ui.add_enabled_ui(enabled, |ui| {
            if !s.choices.is_empty() {
                let selected = s.choices[(*value - s.min) as usize].clone();
                theme::row_text_first(ui, &s.label, |ui| {
                    theme::select(ui, "choice", &selected, false, |ui| {
                        for (i, label) in s.choices.iter().enumerate() {
                            ui.selectable_value(value, s.min + i as i32, label);
                        }
                    });
                });
            } else if s.min == 0 && s.max == 1 {
                let mut checked = *value != 0;
                theme::row_toggle(ui, &s.label, |ui| {
                    if theme::checkbox(ui, &mut checked, theme).changed() {
                        *value = i32::from(checked);
                    }
                });
            } else {
                // Some sliders read better inverted, exactly as the web does.
                let (mut displayed, max, label) = match (f, key) {
                    (Format::Avif, "speed") => (10 - *value, 10, "Effort (10 = lent)"),
                    (Format::Webp, "near_lossless") => (100 - *value, 100, "Perte légère"),
                    (Format::Webp, "filter_sharpness") => (7 - *value, 7, "Lissage du filtre"),
                    _ => (*value, s.max, s.label.as_str()),
                };
                theme::one_cell(ui, |ui| {
                    let mut v = displayed as f64;
                    if theme::range(ui, label, &mut v, s.min as f64..=max as f64, 1.0, 0, theme) {
                        displayed = v.round() as i32;
                        *value = match (f, key) {
                            (Format::Avif, "speed") => 10 - displayed,
                            (Format::Webp, "near_lossless") => 100 - displayed,
                            (Format::Webp, "filter_sharpness") => 7 - displayed,
                            _ => displayed,
                        };
                    }
                });
            }
        })
    });
}

/// `processors/resize/client` — the web presets.
const SIZE_PRESETS: [f64; 7] = [0.25, 0.3333, 0.5, 1.0, 2.0, 3.0, 4.0];

/// Resize, laid out like the web: method, preset, then real pixel sizes.
/// `source` is the size of the (rotated) input image, when known.
fn resize_options(
    ui: &mut Ui,
    side: &mut Side,
    is_svg: bool,
    theme: Theme,
    source: Option<(u32, u32)>,
) {
    let (sw, sh) = source.unwrap_or((side.resize.width.max(1), side.resize.height.max(1)));
    // Show exactly what the pipeline will produce, "0 = auto" included.
    let (dw, dh) = target_dimensions(sw, sh, side).unwrap_or((sw, sh));
    theme::section(ui, |ui| {
        let r = &mut side.resize;
        let filter = r.filter;
        theme::row_text_first(ui, "Méthode :", |ui| {
            theme::select(ui, "filter", filter.label(), false, |ui| {
                for f in Filter::ALL {
                    ui.add_enabled_ui(f != Filter::Vector || is_svg, |ui| {
                        ui.selectable_value(&mut r.filter, f, f.label());
                    });
                }
            });
        });
        let scaled = |p: f64| {
            (
                ((sw as f64 * p).round() as u32).max(1),
                ((sh as f64 * p).round() as u32).max(1),
            )
        };
        let current = SIZE_PRESETS.iter().position(|&p| scaled(p) == (dw, dh));
        let label = |p: f64| format!("{} %", (p * 100.).round());
        let selected = current.map_or("Personnalisé".to_owned(), |i| label(SIZE_PRESETS[i]));
        theme::row_text_first(ui, "Préréglage :", |ui| {
            theme::select(ui, "preset", &selected, false, |ui| {
                for (i, &p) in SIZE_PRESETS.iter().enumerate() {
                    if ui.selectable_label(current == Some(i), label(p)).clicked() {
                        (r.width, r.height) = scaled(p);
                    }
                }
                ui.add_enabled(
                    false,
                    egui::Button::selectable(current.is_none(), "Personnalisé"),
                );
            });
        });
        let mut width = dw;
        theme::row_text_first(ui, "Largeur :", |ui| {
            if theme::text_field(ui, &mut width, 32768).changed() {
                r.width = width;
                r.height = if r.lock_ratio {
                    ((width as f64 * sh as f64 / sw as f64).round() as u32).max(1)
                } else {
                    dh
                };
            }
        });
        let mut height = dh;
        theme::row_text_first(ui, "Hauteur :", |ui| {
            if theme::text_field(ui, &mut height, 32768).changed() {
                r.height = height;
                r.width = if r.lock_ratio {
                    ((height as f64 * sw as f64 / sh as f64).round() as u32).max(1)
                } else {
                    dw
                };
            }
        });
        theme::row_toggle(ui, "Prémultiplier l’alpha", |ui| {
            theme::checkbox(ui, &mut r.premultiply, theme);
        });
        theme::row_toggle(ui, "RGB linéaire", |ui| {
            theme::checkbox(ui, &mut r.linear_rgb, theme);
        });
        theme::row_toggle(ui, "Conserver les proportions", |ui| {
            if theme::checkbox(ui, &mut r.lock_ratio, theme).changed() && !r.lock_ratio {
                // Pin the current size so unlocking never makes it jump.
                (r.width, r.height) = (dw, dh);
            }
        });
        if !r.lock_ratio {
            let fit = if r.crop {
                "Recadrage central"
            } else {
                "Étirer"
            };
            theme::row_text_first(ui, "Ajustement :", |ui| {
                theme::select(ui, "fit", fit, false, |ui| {
                    ui.selectable_value(&mut r.crop, false, "Étirer");
                    ui.selectable_value(&mut r.crop, true, "Recadrage central");
                });
            });
        }
    });
}

fn palette_options(ui: &mut Ui, side: &mut Side, theme: Theme) {
    theme::section(ui, |ui| {
        let p = &mut side.palette;
        let selected = if p.zx { "ZX" } else { "Standard" };
        theme::row_text_first(ui, "Type", |ui| {
            theme::select(ui, "quant-type", selected, false, |ui| {
                ui.selectable_value(&mut p.zx, false, "Standard");
                ui.selectable_value(&mut p.zx, true, "ZX");
            });
        });
        if !p.zx {
            theme::one_cell(ui, |ui| {
                let mut colors = p.colors as f64;
                if theme::range(ui, "Couleurs", &mut colors, 2.0..=256.0, 1.0, 0, theme) {
                    p.colors = colors.round() as u32;
                }
            });
        }
        theme::one_cell(ui, |ui| {
            let mut dither = p.dither as f64;
            if theme::range(ui, "Tramage", &mut dither, 0.0..=1.0, 0.01, 2, theme) {
                p.dither = dither as f32;
            }
        });
    });
}

/// Draws the "Edit" half of a panel: resize and palette, hidden for the
/// original image exactly as the web app hides it.
pub fn edit(
    ui: &mut Ui,
    side: &mut Side,
    is_svg: bool,
    theme: Theme,
    top: egui::CornerRadius,
    source: Option<(u32, u32)>,
    buttons: impl FnOnce(&mut Ui),
) {
    theme::options_title(ui, "Édition", theme, false, top, buttons);
    theme::section_enabler(ui, "Redimensionner", &mut side.resize.enabled, theme);
    if side.resize.enabled {
        resize_options(ui, side, is_svg, theme, source);
    }
    theme::section_enabler(ui, "Réduire la palette", &mut side.palette.enabled, theme);
    if side.palette.enabled {
        palette_options(ui, side, theme);
    }
}

/// Draws the "Compress" half of a panel: codec picker and its settings.
pub fn compress(
    ui: &mut Ui,
    side: &mut Side,
    theme: Theme,
    source_name: &str,
    top: egui::CornerRadius,
) {
    let original = side.format.is_none();
    theme::options_title(ui, "Compression", theme, original, top, |_| {});
    let selected = if original {
        format!("Image originale ({source_name})")
    } else {
        format_label(side.format).to_owned()
    };
    theme::section(ui, |ui| {
        theme::one_cell(ui, |ui| {
            theme::select(ui, "format", &selected, true, |ui| {
                ui.selectable_value(
                    &mut side.format,
                    None,
                    format!("Image originale ({source_name})"),
                );
                for f in Format::ALL {
                    ui.selectable_value(&mut side.format, Some(f), format_label(Some(f)));
                }
            });
        });
    });
    let Some(format) = side.format else {
        return;
    };
    theme::section(ui, |ui| {
        let options = side.options.get_mut(format.key()).unwrap();
        match format {
            Format::Avif => {
                let mut lossless = options["quality"] == 100;
                theme::row_toggle(ui, "Sans perte", |ui| {
                    if theme::checkbox(ui, &mut lossless, theme).changed() {
                        options.insert("quality".into(), if lossless { 100 } else { 50 });
                        options.insert("qualityAlpha".into(), -1);
                        if lossless {
                            options.insert("subsample".into(), 3);
                        }
                    }
                });
                if !lossless {
                    option(ui, format, "quality", options, theme);
                }
                option(ui, format, "speed", options, theme);
            }
            Format::Webp => {
                option(ui, format, "lossless", options, theme);
                if options["lossless"] != 0 {
                    // The web exposes a single "effort" slider over these pairs.
                    const PRESETS: [(i32, i32); 10] = [
                        (0, 0),
                        (1, 20),
                        (2, 25),
                        (3, 30),
                        (3, 50),
                        (4, 50),
                        (4, 75),
                        (4, 90),
                        (5, 90),
                        (6, 100),
                    ];
                    let mut preset = PRESETS
                        .iter()
                        .position(|&(m, q)| m == options["method"] && q == options["quality"])
                        .unwrap_or(6) as f64;
                    theme::one_cell(ui, |ui| {
                        if theme::range(
                            ui,
                            "Effort sans perte",
                            &mut preset,
                            0.0..=9.0,
                            1.0,
                            0,
                            theme,
                        ) {
                            let (m, q) = PRESETS[preset.round() as usize];
                            options.insert("method".into(), m);
                            options.insert("quality".into(), q);
                        }
                    });
                    option(ui, format, "near_lossless", options, theme);
                } else {
                    option(ui, format, "quality", options, theme);
                    option(ui, format, "method", options, theme);
                }
            }
            Format::Jpeg => option(ui, format, "quality", options, theme),
            Format::Png => {
                option(ui, format, "level", options, theme);
                option(ui, format, "interlace", options, theme);
            }
        }

        let id = ui.id().with("advanced");
        let mut open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        theme::revealer(ui, "Réglages avancés", &mut open);
        ui.data_mut(|d| d.insert_temp(id, open));
        if open {
            for spec in specs(format) {
                if match format {
                    Format::Jpeg => spec.key == "quality",
                    Format::Png => true,
                    Format::Webp => ["quality", "method", "lossless", "near_lossless"]
                        .contains(&spec.key.as_str()),
                    Format::Avif => ["quality", "speed"].contains(&spec.key.as_str()),
                } {
                    continue;
                }
                option(ui, format, &spec.key, options, theme);
            }
            if format == Format::Jpeg {
                theme::row_text_first(ui, "Fond", |ui| {
                    ui.color_edit_button_srgb(&mut side.background);
                });
            }
        }
    });
}
