//! User-facing languages. Messages are written in English in the code, and
//! that English text is the key of every translation catalog in `locales/`:
//!
//! ```
//! use squoosh_i18n::{t, tr};
//! let title = t("Add images");
//! let count = tr!("{} images in the list", 3);
//! let error = tr!("Error: {error}", error = "disk full");
//! ```
//!
//! Placeholders are `{}` (the next argument), `{0}`, `{1}`… (by position, so
//! a translation can reorder them) and `{name}` (named arguments). A missing
//! translation falls back to English; the `catalogs` test keeps every
//! catalog complete.
use std::{
    collections::HashMap,
    fmt::{Display, Write},
    sync::{
        OnceLock,
        atomic::{AtomicU8, Ordering},
    },
};

macro_rules! languages {
    ($($variant:ident $code:literal $name:literal $english:literal),* $(,)?) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Lang {
            $($variant),*
        }
        impl Lang {
            pub const ALL: &[Self] = &[$(Self::$variant),*];
            /// BCP 47 tag, also the catalog file name.
            pub fn code(self) -> &'static str {
                match self {
                    $(Self::$variant => $code),*
                }
            }
            /// The language's own name, as shown in the language picker.
            pub fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name),*
                }
            }
            pub fn english_name(self) -> &'static str {
                match self {
                    $(Self::$variant => $english),*
                }
            }
            fn source(self) -> &'static str {
                // English is the source language: its catalog is empty.
                match self {
                    $(Self::$variant => include_str!(concat!("../locales/", $code, ".json"))),*
                }
            }
        }
    };
}
// Arabic, Hebrew, Persian and the Indic scripts are left out: egui neither
// shapes complex scripts nor lays out right-to-left text.
languages! {
    En "en" "English" "English",
    Fr "fr" "Français" "French",
    De "de" "Deutsch" "German",
    Es "es" "Español" "Spanish",
    It "it" "Italiano" "Italian",
    PtBr "pt-BR" "Português (Brasil)" "Portuguese (Brazil)",
    Nl "nl" "Nederlands" "Dutch",
    Pl "pl" "Polski" "Polish",
    Ru "ru" "Русский" "Russian",
    Uk "uk" "Українська" "Ukrainian",
    Tr "tr" "Türkçe" "Turkish",
    Id "id" "Bahasa Indonesia" "Indonesian",
    Vi "vi" "Tiếng Việt" "Vietnamese",
    ZhCn "zh-CN" "简体中文" "Chinese (Simplified)",
    ZhTw "zh-TW" "繁體中文" "Chinese (Traditional)",
    Ja "ja" "日本語" "Japanese",
    Ko "ko" "한국어" "Korean",
}

impl Lang {
    /// Parses a locale such as `fr`, `fr_FR.UTF-8`, `pt-BR` or `zh-Hant-HK`.
    pub fn from_locale(locale: &str) -> Option<Self> {
        let locale = locale.split(['.', '@']).next()?.replace('_', "-");
        let mut parts = locale.split('-').map(str::to_ascii_lowercase);
        let language = parts.next()?;
        let rest: Vec<String> = parts.collect();
        let has = |tag: &str| rest.iter().any(|p| p == tag);
        Some(match language.as_str() {
            "zh" if ["hant", "tw", "hk", "mo"].into_iter().any(has) => Self::ZhTw,
            "zh" => Self::ZhCn,
            "pt" => Self::PtBr,
            code => *Self::ALL.iter().find(|l| l.code() == code)?,
        })
    }
    /// The language forced by `SQUOOSH_LANG`, if any.
    pub fn from_env() -> Option<Self> {
        Self::from_locale(&std::env::var("SQUOOSH_LANG").ok()?)
    }
    /// The first supported language of the system, English otherwise.
    pub fn system() -> Self {
        sys_locale::get_locales()
            .find_map(|l| Self::from_locale(&l))
            .unwrap_or(Self::En)
    }
    /// Every translated message, e.g. to check that a font covers them.
    pub fn texts(self) -> impl Iterator<Item = &'static str> {
        self.catalog().values().copied()
    }
    fn catalog(self) -> &'static HashMap<&'static str, &'static str> {
        static CATALOGS: OnceLock<Vec<OnceLock<HashMap<&'static str, &'static str>>>> =
            OnceLock::new();
        let slots = CATALOGS.get_or_init(|| Self::ALL.iter().map(|_| OnceLock::new()).collect());
        slots[self as usize].get_or_init(|| {
            let parsed: HashMap<String, String> = serde_json::from_str(self.source())
                .unwrap_or_else(|e| panic!("locales/{}.json: {e}", self.code()));
            parsed
                .into_iter()
                .filter(|(_, v)| !v.is_empty())
                .map(|(k, v)| (&*k.leak(), &*v.leak()))
                .collect()
        })
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

/// The language used by every crate of the application.
pub fn lang() -> Lang {
    Lang::ALL
        .get(CURRENT.load(Ordering::Relaxed) as usize)
        .copied()
        .unwrap_or(Lang::En)
}
pub fn set_lang(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

/// Translates an English message into the current language.
pub fn t(english: &str) -> &str {
    match lang() {
        Lang::En => english,
        lang => lang.catalog().get(english).copied().unwrap_or(english),
    }
}

/// Marks an English message for translation without translating it yet,
/// for tables that are translated later with [`t`].
pub const fn n(english: &'static str) -> &'static str {
    english
}

/// Fills the placeholders of a translated template; see the crate docs.
pub fn format(template: &str, args: &[(Option<&str>, &dyn Display)]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut next = 0;
    let mut rest = template;
    while let Some(start) = rest.find(['{', '}']) {
        out.push_str(&rest[..start]);
        let tail = &rest[start..];
        if tail.starts_with("{{") || tail.starts_with("}}") {
            out.push_str(&tail[..1]);
            rest = &tail[2..];
            continue;
        }
        let Some(end) = tail.find('}').filter(|_| tail.starts_with('{')) else {
            out.push_str(&tail[..1]);
            rest = &tail[1..];
            continue;
        };
        let inner = &tail[1..end];
        let arg = if inner.is_empty() {
            next += 1;
            args.get(next - 1)
        } else if let Ok(i) = inner.parse::<usize>() {
            args.get(i)
        } else {
            args.iter().find(|(name, _)| *name == Some(inner))
        };
        match arg {
            Some((_, value)) => {
                let _ = write!(out, "{value}");
            }
            None => out.push_str(&tail[..=end]),
        }
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

/// `format!` over a translated template: positional or named arguments.
#[macro_export]
macro_rules! tr {
    ($english:literal $(,)?) => {
        $crate::t($english).to_owned()
    };
    ($english:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::format(
            $crate::t($english),
            &[$((Some(stringify!($name)), &$value as &dyn ::std::fmt::Display)),+],
        )
    };
    ($english:literal, $($value:expr),+ $(,)?) => {
        $crate::format(
            $crate::t($english),
            &[$((None, &$value as &dyn ::std::fmt::Display)),+],
        )
    };
}

#[cfg(test)]
mod tests;
