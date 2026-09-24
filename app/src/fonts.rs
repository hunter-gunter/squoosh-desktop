//! System fonts for the scripts that egui's built-in font lacks: Chinese,
//! Japanese, Korean and the Vietnamese diacritics. Nothing is bundled; a
//! language is offered when the system has a font covering its catalog.
use crate::theme;
use eframe::egui;
use skrifa::MetadataProvider;
use squoosh_i18n::Lang;
use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

/// Family names tried first, in order, before scanning every system font.
fn preferred(lang: Lang) -> &'static [&'static str] {
    match lang {
        Lang::ZhCn => &[
            "Noto Sans CJK SC",
            "Source Han Sans SC",
            "Noto Sans SC",
            "Microsoft YaHei UI",
            "Microsoft YaHei",
            "PingFang SC",
            "WenQuanYi Micro Hei",
        ],
        Lang::ZhTw => &[
            "Noto Sans CJK TC",
            "Source Han Sans TC",
            "Noto Sans TC",
            "Microsoft JhengHei UI",
            "Microsoft JhengHei",
            "PingFang TC",
            "IBM Plex Sans TC",
        ],
        Lang::Ja => &[
            "Noto Sans CJK JP",
            "Source Han Sans JP",
            "Noto Sans JP",
            "Yu Gothic UI",
            "Yu Gothic",
            "Meiryo UI",
            "Meiryo",
            "Hiragino Sans",
            "IPAGothic",
            "IBM Plex Sans JP",
        ],
        Lang::Ko => &[
            "Noto Sans CJK KR",
            "Source Han Sans KR",
            "Noto Sans KR",
            "Malgun Gothic",
            "Apple SD Gothic Neo",
            "NanumGothic",
            "IBM Plex Sans KR",
        ],
        Lang::Vi => &[
            "Noto Sans",
            "Segoe UI",
            "DejaVu Sans",
            "Liberation Sans",
            "Arial",
        ],
        _ => &[],
    }
}

#[derive(Clone, Copy)]
struct Face {
    data: &'static [u8],
    index: u32,
}

#[derive(Default)]
pub struct Fonts {
    db: Option<fontdb::Database>,
    files: HashMap<fontdb::ID, &'static [u8]>,
    faces: HashMap<Lang, Option<Face>>,
}
impl Fonts {
    /// Whether the language can be displayed, loading its font if needed.
    pub fn available(&mut self, lang: Lang) -> bool {
        preferred(lang).is_empty() || self.face(lang).is_some()
    }
    fn face(&mut self, lang: Lang) -> Option<Face> {
        if let Some(face) = self.faces.get(&lang) {
            return *face;
        }
        // The characters the language needs beyond egui's own font.
        let needed: BTreeSet<char> = std::iter::once(lang.name())
            .chain(lang.texts())
            .flat_map(str::chars)
            .filter(|c| !c.is_ascii())
            .collect();
        let db = self.db.get_or_insert_with(|| {
            let mut db = fontdb::Database::new();
            db.load_system_fonts();
            db
        });
        let named = |family: &str| {
            db.faces()
                .filter(|f| f.families.iter().any(|(name, _)| name == family))
                .min_by_key(|f| (f.style != fontdb::Style::Normal, f.weight.0.abs_diff(400)))
                .map(|f| f.id)
        };
        let mut ids: Vec<fontdb::ID> = preferred(lang).iter().filter_map(|f| named(f)).collect();
        ids.extend(
            db.faces()
                .filter(|f| f.style == fontdb::Style::Normal && f.weight.0 == 400)
                .map(|f| f.id),
        );
        let covers = |data: &[u8], index: u32| {
            let Ok(font) = skrifa::FontRef::from_index(data, index) else {
                return false;
            };
            let charmap = font.charmap();
            let missing = needed.iter().filter(|&&c| charmap.map(c).is_none()).count();
            // Tolerate a stray symbol, which egui's fallbacks may still cover.
            missing * 100 <= needed.len()
        };
        // Fonts are memory-mapped while checked; only the chosen one is kept.
        let chosen = ids
            .into_iter()
            .find(|&id| db.with_face_data(id, covers).unwrap_or(false));
        let found = chosen.and_then(|id| self.load(id));
        self.faces.insert(lang, found);
        found
    }
    fn load(&mut self, id: fontdb::ID) -> Option<Face> {
        let db = self.db.as_ref()?;
        let (_, index) = db.face_source(id)?;
        let data = match self.files.get(&id) {
            Some(data) => *data,
            None => {
                // Fonts live for the whole session, shared by every face of a file.
                let data: &'static [u8] = db.with_face_data(id, |d, _| d.to_vec())?.leak();
                self.files.insert(id, data);
                data
            }
        };
        Some(Face { data, index })
    }
    /// Installs egui's fonts with `lang`'s system font as the first fallback,
    /// and with `all`, the fonts of every other language too (for the names
    /// in the language picker).
    pub fn install(&mut self, ctx: &egui::Context, lang: Lang, all: bool) {
        let mut order = vec![lang];
        if all {
            order.extend(Lang::ALL.iter().copied().filter(|&l| l != lang));
        }
        let mut fonts = theme::font_definitions();
        let mut position = 1;
        for lang in order {
            if preferred(lang).is_empty() {
                continue;
            }
            let Some(face) = self.face(lang) else {
                continue;
            };
            let name = format!("system-{}", lang.code());
            let mut data = egui::FontData::from_static(face.data);
            data.index = face.index;
            fonts.font_data.insert(name.clone(), Arc::new(data));
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                let list = fonts.families.entry(family).or_default();
                list.insert(position.min(list.len()), name.clone());
            }
            position += 1;
        }
        ctx.set_fonts(fonts);
    }
}
