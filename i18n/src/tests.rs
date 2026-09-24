use super::*;
use std::{collections::BTreeSet, fs, path::Path};

#[test]
fn locales() {
    assert_eq!(Lang::from_locale("fr_FR.UTF-8"), Some(Lang::Fr));
    assert_eq!(Lang::from_locale("fr-CA"), Some(Lang::Fr));
    assert_eq!(Lang::from_locale("FR"), Some(Lang::Fr));
    assert_eq!(Lang::from_locale("en-US"), Some(Lang::En));
    assert_eq!(Lang::from_locale("pt_PT"), Some(Lang::PtBr));
    assert_eq!(Lang::from_locale("zh_CN.UTF-8"), Some(Lang::ZhCn));
    assert_eq!(Lang::from_locale("zh-Hans"), Some(Lang::ZhCn));
    assert_eq!(Lang::from_locale("zh-Hant-HK"), Some(Lang::ZhTw));
    assert_eq!(Lang::from_locale("zh_TW"), Some(Lang::ZhTw));
    assert_eq!(Lang::from_locale("ja_JP"), Some(Lang::Ja));
    assert_eq!(Lang::from_locale("ar_EG"), None);
    assert_eq!(Lang::from_locale("fry"), None);
    assert_eq!(Lang::from_locale("C"), None);
    assert_eq!(Lang::from_locale(""), None);
}

#[test]
fn placeholders() {
    let f = |t: &str| format(t, &[(None, &1), (None, &"b")]);
    assert_eq!(f("{} and {}"), "1 and b");
    assert_eq!(f("{1} before {0}"), "b before 1");
    assert_eq!(f("{{}} {}%"), "{} 1%");
    assert_eq!(f("{missing} {"), "{missing} {");
    assert_eq!(
        format("Error: {error}", &[(Some("error"), &"disk full")]),
        "Error: disk full"
    );
    assert_eq!(tr!("{} images in the list", 3), "3 images in the list");
}

/// Reads the string literal that follows `start` (after optional spaces).
fn literal(source: &str) -> Option<String> {
    let rest = source.trim_start().strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = rest.chars();
    while let Some(c) = chars.next() {
        match c {
            '"' => return Some(out),
            '\\' => match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                'n' => out.push('\n'),
                'u' => {
                    let hex: String = chars.by_ref().skip(1).take_while(|&c| c != '}').collect();
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                _ => return None,
            },
            c => out.push(c),
        }
    }
    None
}

/// Every English message of the application: `t("…")`, `tr!("…")` and
/// `n("…")` in the sources, plus the labels of the codec registry.
fn english_keys() -> BTreeSet<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let mut keys = BTreeSet::new();
    for dir in ["app/src", "core/src", "codecs/src"] {
        for entry in fs::read_dir(root.join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            let source = fs::read_to_string(&path).unwrap();
            for marker in ["t(", "tr!(", "n("] {
                for (i, _) in source.match_indices(marker) {
                    let before = source[..i].chars().next_back();
                    if before.is_some_and(|c| c.is_alphanumeric() || c == '_') {
                        continue;
                    }
                    if let Some(key) = literal(&source[i + marker.len()..]) {
                        keys.insert(key);
                    }
                }
            }
        }
    }
    let registry: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("codecs/options.json")).unwrap())
            .unwrap();
    for spec in registry
        .as_object()
        .unwrap()
        .values()
        .flat_map(|v| v.as_array().unwrap())
    {
        keys.insert(spec["label"].as_str().unwrap().to_owned());
        for choice in spec["choices"].as_array().unwrap() {
            keys.insert(choice.as_str().unwrap().to_owned());
        }
    }
    keys
}

/// The placeholders of a template, with `{}` numbered like `{0}`, `{1}`….
fn holes(template: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut next = 0;
    let mut rest = template.replace("{{", "").replace("}}", "");
    while let Some(start) = rest.find('{') {
        let Some(end) = rest[start..].find('}') else {
            break;
        };
        let inner = &rest[start + 1..start + end];
        out.insert(if inner.is_empty() {
            next += 1;
            (next - 1).to_string()
        } else {
            inner.to_owned()
        });
        rest = rest[start + end + 1..].to_owned();
    }
    out
}

/// Technical names that stay as they are in every language; a catalog may
/// still translate them.
const VERBATIM: &[&str] = &[
    "4:0:0",
    "4:2:0",
    "4:2:2",
    "4:4:4",
    "Ahumada et al",
    "ImageMagick",
    "JPEG Annex K",
    "Klein et al",
    "PSNR",
    "Peterson et al",
    "RGB",
    "SSIM",
    "Sharp YUV",
    "Watson et al",
    "YCbCr",
];

#[test]
fn catalogs() {
    let all = english_keys();
    let keys: BTreeSet<&String> = all
        .iter()
        .filter(|k| !VERBATIM.contains(&k.as_str()))
        .collect();
    assert!(
        keys.len() > 150,
        "the source scan found only {} keys",
        keys.len()
    );
    let mut problems = Vec::new();
    for &lang in &Lang::ALL[1..] {
        let catalog: HashMap<String, String> = serde_json::from_str(lang.source()).unwrap();
        for &key in &keys {
            match catalog.get(key) {
                None => problems.push(format!("{}: missing {key:?}", lang.code())),
                Some(v) if v.trim().is_empty() => {
                    problems.push(format!("{}: empty {key:?}", lang.code()))
                }
                Some(v) if holes(v) != holes(key) => {
                    problems.push(format!("{}: placeholders of {key:?} in {v:?}", lang.code()))
                }
                _ => {}
            }
        }
        for key in catalog.keys().filter(|k| !all.contains(*k)) {
            problems.push(format!("{}: unused {key:?}", lang.code()));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
