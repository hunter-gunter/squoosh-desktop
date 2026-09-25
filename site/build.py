#!/usr/bin/env python3
"""Builds the Squoosh Desktop website: one static page per language.

    python3 site/build.py [--out DIR] [--version X.Y.Z] [--base-url URL]

English is served at the root, every other language under /<code>/.
The texts live in site/locales/<code>.json; en.json lists every key.
Uses the standard library only, so it runs as is on GitHub Actions.
"""

import argparse
import html
import json
import re
import shutil
from datetime import date
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
REPO = "https://github.com/hunter-gunter/squoosh-desktop"
BASE_URL = "https://hunter-gunter.github.io/squoosh-desktop/"

# Same order and codes as the app's language menu (i18n/locales).
LANGS = ["en", "fr", "de", "es", "it", "pt-BR", "nl", "pl", "ru", "uk",
         "tr", "id", "vi", "zh-CN", "zh-TW", "ja", "ko"]
OG_LOCALES = {"en": "en_US", "fr": "fr_FR", "de": "de_DE", "es": "es_ES", "it": "it_IT",
              "pt-BR": "pt_BR", "nl": "nl_NL", "pl": "pl_PL", "ru": "ru_RU", "uk": "uk_UA",
              "tr": "tr_TR", "id": "id_ID", "vi": "vi_VN", "zh-CN": "zh_CN", "zh-TW": "zh_TW",
              "ja": "ja_JP", "ko": "ko_KR"}
CYRILLIC = {"ru", "uk"}
CJK = {"zh-CN", "zh-TW", "ja", "ko"}
CODECS = ["Rust", "MozJPEG", "OxiPNG", "WebP", "AVIF", "Lanczos3", "Mitchell", "Catmull-Rom",
          "HQX", "Dithering", "Lossless", "Batch"]
# FAQ entries: q1/a1 … q8/a8 in every locale.
FAQ = range(1, 9)
# Comparison rows c1 … c6: (squoosh.app, Squoosh Desktop).
COMPARE = [(True, True), (False, True), (False, True), (False, True), (False, True), (True, True)]

# Screenshot widths made by site/screenshots.sh, and the width each one takes on the page.
WIDTHS = (740, 1110, 1480)
HERO_SIZES = "(max-width: 960px) calc(100vw - 38px), min(620px, 52vw)"
SHOT_SIZES = "(max-width: 640px) calc(100vw - 38px), (max-width: 1192px) calc(50vw - 38px), 560px"
BATCH_SIZES = "(max-width: 1192px) calc(100vw - 38px), 1160px"

ICON_DOWNLOAD = ('<svg class="arrow" viewBox="0 0 24 24" aria-hidden="true"><path fill="none" stroke="currentColor" '
                 'stroke-width="3" stroke-linecap="square" d="M12 3v12m-6-6 6 6 6-6M4 21h16"/></svg>')
ICON_GITHUB = ('<svg viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="M12 .5a11.5 11.5 0 0 0-3.64 '
               '22.41c.58.1.79-.25.79-.56v-2c-3.2.7-3.88-1.37-3.88-1.37-.52-1.33-1.28-1.69-1.28-1.69-1.05-.72.08-.7.08-.7 '
               '1.16.08 1.77 1.19 1.77 1.19 1.03 1.77 2.7 1.26 3.36.96.1-.75.4-1.26.73-1.55-2.55-.29-5.24-1.28-5.24-5.69 '
               '0-1.26.45-2.28 1.19-3.09-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.17 1.18a11 11 0 0 1 5.77 0c2.2-1.49 '
               '3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.81 1.19 1.83 1.19 3.09 0 4.42-2.69 5.39-5.26 5.68.41.36.78 '
               '1.06.78 2.14v3.17c0 .31.21.67.8.56A11.5 11.5 0 0 0 12 .5Z"/></svg>')
ICON_GLOBE = ('<svg viewBox="0 0 24 24" aria-hidden="true"><g fill="none" stroke="currentColor" stroke-width="2.4">'
              '<circle cx="12" cy="12" r="9.5"/><path d="M2.5 12h19M12 2.5c-5 5.6-5 13.4 0 19m0-19c5 5.6 5 13.4 0 19"/>'
              '</g></svg>')


def icon(name, cls="os"):
    """Inline logo from site/icons/ (Simple Icons, CC0)."""
    path = re.search(r' d="([^"]+)"', (SITE / "icons" / f"{name}.svg").read_text("utf-8")).group(1)
    return f'<svg class="{cls}" viewBox="0 0 24 24" aria-hidden="true"><path fill="currentColor" d="{path}"/></svg>'


def srcset(prefix, name, ext):
    return ", ".join(f"{prefix}assets/screenshots/{name}-{w}.{ext} {w}w" for w in WIDTHS)


def picture(prefix, name, alt, sizes, hero=False):
    """AVIF with a WebP fallback, in every width of WIDTHS."""
    loading = 'fetchpriority="high"' if hero else 'loading="lazy" decoding="async"'
    return (f'<picture><source type="image/avif" srcset="{srcset(prefix, name, "avif")}" sizes="{sizes}">'
            f'<img src="{prefix}assets/screenshots/{name}-{WIDTHS[-1]}.webp" srcset="{srcset(prefix, name, "webp")}" '
            f'sizes="{sizes}" width="1480" height="920" {loading} alt="{e(alt)}"></picture>')


def load_locales():
    locales = {code: json.loads((SITE / "locales" / f"{code}.json").read_text("utf-8")) for code in LANGS}
    keys = set(locales["en"])
    for code, texts in locales.items():
        missing, extra = keys - set(texts), set(texts) - keys
        if missing or extra:
            raise SystemExit(f"locales/{code}.json: missing {sorted(missing)}, unknown {sorted(extra)}")
    return locales


def project_version():
    text = (ROOT / "Cargo.toml").read_text("utf-8")
    section = re.search(r"^\[workspace\.package\]$(.*?)(?=^\[|\Z)", text, re.S | re.M).group(1)
    return re.search(r'^version\s*=\s*"([^"]+)"', section, re.M).group(1)


def slug(code):
    return "" if code == "en" else code.lower() + "/"


def e(text):
    return html.escape(text, quote=True)


def styles(prefix):
    css = (SITE / "assets" / "fonts.css").read_text("utf-8") + (SITE / "assets" / "style.css").read_text("utf-8")
    css = css.replace("url(fonts/", f"url({prefix}assets/fonts/")
    # Minification: comments, then whitespace around punctuation (never around
    # "-" or "+", which calc() needs).
    css = re.sub(r"/\*.*?\*/", "", css, flags=re.S)
    css = re.sub(r"\s+", " ", css)
    css = re.sub(r" ?([{}:;,>]) ?", r"\1", css)
    return css.replace(";}", "}").strip()


def downloads(version):
    url = f"{REPO}/releases/download/v{version}"
    return {
        "windows": f"{url}/squoosh-desktop-{version}-windows-x64-setup.exe",
        "zip": f"{url}/squoosh-desktop-{version}-windows-x64.zip",
        "appimage": f"{url}/squoosh-desktop-{version}-x86_64.AppImage",
        "deb": f"{url}/squoosh-desktop_{version}_amd64.deb",
        "arch": "https://aur.archlinux.org/packages/squoosh-desktop",
        "releases": f"{REPO}/releases",
    }


def json_ld(t, code, locales, version, base_url, dl):
    page = base_url + slug(code)
    app = {
        "@context": "https://schema.org",
        "@type": "SoftwareApplication",
        "@id": page + "#app",
        "name": "Squoosh Desktop",
        "alternateName": "Squoosh for Windows and Linux",
        "description": t["description"],
        "url": page,
        "applicationCategory": "MultimediaApplication",
        "applicationSubCategory": "Image compressor",
        "operatingSystem": "Windows 10, Windows 11, Linux",
        "processorRequirements": "x86_64",
        "softwareVersion": version,
        "downloadUrl": dl["releases"],
        "installUrl": dl["releases"],
        "image": base_url + "assets/icon-512.png",
        "screenshot": [base_url + f"assets/screenshots/{name}-{WIDTHS[-1]}.webp" for name in ("editor", "welcome", "batch")],
        "featureList": [t[f"f{i}_title"] + ": " + t[f"f{i}_text"] for i in range(1, 5)]
                       + [t[f"b{i}_title"] + ": " + t[f"b{i}_text"] for i in range(1, 7)]
                       + [t[f"r{i}_title"] + ": " + t[f"r{i}_text"] for i in range(1, 5)],
        "keywords": "Squoosh, Rust, image compressor, AVIF, WebP, MozJPEG, OxiPNG, offline, batch",
        "inLanguage": LANGS,
        "isAccessibleForFree": True,
        "offers": {"@type": "Offer", "price": "0", "priceCurrency": "USD"},
        "license": "https://www.gnu.org/licenses/gpl-3.0.html",
        "sameAs": [REPO],
        "isBasedOn": {"@type": "SoftwareApplication", "name": "Squoosh", "url": "https://squoosh.app"},
    }
    # Declares the language the app is written in, which SoftwareApplication cannot.
    source = {
        "@context": "https://schema.org",
        "@type": "SoftwareSourceCode",
        "name": "Squoosh Desktop",
        "description": t["rust_intro"],
        "codeRepository": REPO,
        "programmingLanguage": {"@type": "ComputerLanguage", "name": "Rust", "url": "https://www.rust-lang.org"},
        "runtimePlatform": ["Windows", "Linux"],
        "license": "https://www.gnu.org/licenses/gpl-3.0.html",
        "targetProduct": {"@id": page + "#app"},
    }
    faq = {
        "@context": "https://schema.org",
        "@type": "FAQPage",
        "inLanguage": code,
        "mainEntity": [{"@type": "Question", "name": t[f"q{i}"],
                        "acceptedAnswer": {"@type": "Answer", "text": t[f"a{i}"]}} for i in FAQ],
    }
    return "\n".join(
        '<script type="application/ld+json">' + json.dumps(data, ensure_ascii=False).replace("</", "<\\/") + "</script>"
        for data in (app, source, faq))


def render(code, locales, version, base_url):
    t = locales[code]
    prefix = "" if code == "en" else "../"
    page = base_url + slug(code)
    dl = downloads(version)

    alternates = "\n".join(f'<link rel="alternate" hreflang="{c}" href="{base_url + slug(c)}">' for c in LANGS)
    alternates += f'\n<link rel="alternate" hreflang="x-default" href="{base_url}">'
    og_alternates = "\n".join(f'<meta property="og:locale:alternate" content="{OG_LOCALES[c]}">'
                              for c in LANGS if c != code)
    preload = ["unbounded-latin"] + (["onest-cyrillic", "unbounded-cyrillic"] if code in CYRILLIC else
                                     [] if code in CJK else ["onest-latin"])
    preloads = "\n".join(f'<link rel="preload" href="{prefix}assets/fonts/{name}.woff2" as="font" '
                         f'type="font/woff2" crossorigin>' for name in preload)

    current = ' aria-current="page"'
    lang_links = "\n".join(
        f'<li><a href="{(prefix + slug(c)) or "./"}" hreflang="{c}" lang="{c}"'
        f'{current if c == code else ""}>{e(locales[c]["lang_name"])}</a></li>' for c in LANGS)

    codecs = "".join(f"<span>{name}</span>" for name in CODECS)
    cards = "\n".join(
        f'<li><span class="num" aria-hidden="true">0{i}</span><h3>{e(t[f"f{i}_title"])}</h3>'
        f'<p>{e(t[f"f{i}_text"])}</p></li>' for i in range(1, 5))
    faq = "\n".join(
        f'<details{" open" if i == 1 else ""}><summary>{e(t[f"q{i}"])}</summary><p>{e(t[f"a{i}"])}</p></details>'
        for i in FAQ)
    checks = "\n".join(
        f'<li><h3>{e(t[f"b{i}_title"])}</h3><p>{e(t[f"b{i}_text"])}</p></li>' for i in range(1, 7))

    def mark(yes):
        sign, label = ("✓", t["compare_yes"]) if yes else ("—", t["compare_no"])
        return f'<span class="{"yes" if yes else "no"}" aria-hidden="true">{sign}</span><span class="sr">{e(label)}</span>'
    compare = "\n".join(
        f'<tr><th scope="row">{e(t[f"c{i}"])}</th><td>{mark(web)}</td><td class="us">{mark(desktop)}</td></tr>'
        for i, (web, desktop) in enumerate(COMPARE, 1))
    stats = "\n".join(
        f'<li><b>{e(t[f"r{i}_value"])}</b><h3>{e(t[f"r{i}_title"])}</h3><p>{e(t[f"r{i}_text"])}</p></li>'
        for i in range(1, 5))
    based_on = e(t["footer_based"]).replace(
        "{squoosh}", '<a href="https://github.com/GoogleChromeLabs/squoosh">Squoosh</a>')

    return f"""<!doctype html>
<html lang="{code}">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{e(t["title"])}</title>
<meta name="description" content="{e(t["description"])}">
<meta name="robots" content="index, follow, max-image-preview:large">
<meta name="theme-color" content="#ff3d8b">
<meta name="color-scheme" content="light">
<link rel="canonical" href="{page}">
{alternates}
<link rel="icon" href="{prefix}assets/favicon-32.png" sizes="32x32" type="image/png">
<link rel="icon" href="{prefix}assets/icon-192.png" sizes="192x192" type="image/png">
<link rel="apple-touch-icon" href="{prefix}assets/apple-touch-icon.png">
<meta property="og:type" content="website">
<meta property="og:site_name" content="Squoosh Desktop">
<meta property="og:title" content="{e(t["title"])}">
<meta property="og:description" content="{e(t["description"])}">
<meta property="og:url" content="{page}">
<meta property="og:image" content="{base_url}assets/og.png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:alt" content="{e(t["hero_alt"])}">
<meta property="og:locale" content="{OG_LOCALES[code]}">
{og_alternates}
<meta name="twitter:card" content="summary_large_image">
{preloads}
<link rel="preload" as="image" type="image/avif" imagesrcset="{srcset(prefix, "editor", "avif")}" imagesizes="{HERO_SIZES}" fetchpriority="high">
<style>
{styles(prefix)}
</style>
{json_ld(t, code, locales, version, base_url, dl)}
</head>
<body>
<a class="skip" href="#main">{e(t["skip"])}</a>
<header class="wrap top">
  <a class="logo" href="{prefix or './'}" aria-label="Squoosh Desktop">
    <img src="{prefix}assets/icon-128.webp" alt="" width="48" height="48"><span>Squoosh Desktop</span>
  </a>
  <nav class="nav" aria-label="{e(t["language"])}">
    <details class="lang">
      <summary class="chip">{ICON_GLOBE}<span>{e(t["lang_name"])}</span></summary>
      <ul>
{lang_links}
      </ul>
    </details>
    <a class="chip" href="{REPO}">{ICON_GITHUB}<span>GitHub</span></a>
  </nav>
</header>

<main id="main">
  <div class="wrap hero">
    <div>
      <h1><span class="brand">Squoosh Desktop</span> <span class="sub">{e(t["tagline"])}</span></h1>
      <p class="lead">{e(t["intro"])}</p>
      <div class="cta">
        <a class="btn btn-win" href="{dl["windows"]}">{icon("windows")}<span>{e(t["dl_windows"])}<small>Windows 10 · 11 · .exe</small></span>{ICON_DOWNLOAD}</a>
        <a class="btn btn-linux" href="{dl["appimage"]}">{icon("linux")}<span>{e(t["dl_linux"])}<small>AppImage · x86_64</small></span>{ICON_DOWNLOAD}</a>
      </div>
      <p class="also"><span>{e(t["also"])}</span>
        <a href="{dl["deb"]}">{icon("debian")}Debian .deb</a>
        <a href="{dl["arch"]}">{icon("archlinux")}Arch Linux · AUR</a>
        <a href="{dl["zip"]}">{icon("windows")}{e(t["portable"])}</a>
        <a href="{dl["releases"]}">{ICON_GITHUB}{e(t["all_releases"])}</a>
      </p>
    </div>
    <div class="shot">
      <div class="window">
        {picture(prefix, "editor", t["hero_alt"], HERO_SIZES, hero=True)}
      </div>
      <a class="rust-sticker" href="#rust"><span>{icon("rust")}{e(t["rust_badge"])}</span></a>
    </div>
  </div>

  <div class="band" aria-hidden="true"><div>{codecs}{codecs}</div></div>

  <section class="wrap" aria-labelledby="features">
    <h2 id="features">{e(t["features_title"])}</h2>
    <ul class="cards">
{cards}
    </ul>
  </section>

  <section class="wrap" aria-labelledby="batch">
    <div class="section-head">
      <h2 id="batch">{e(t["batch_title"])}</h2>
      <p>{e(t["batch_intro"])}</p>
    </div>
    <div class="batch">
      <figure class="window">
        {picture(prefix, "batch", t["batch_alt"], BATCH_SIZES)}
        <figcaption>{e(t["batch_caption"])}</figcaption>
      </figure>
      <ul class="checks">
{checks}
      </ul>
    </div>
  </section>

  <section class="wrap" aria-labelledby="compare">
    <h2 id="compare">{e(t["compare_title"])}</h2>
    <table class="compare">
      <thead><tr><th scope="col">{e(t["compare_feature"])}</th><th scope="col">squoosh.app</th><th scope="col" class="us">Squoosh Desktop</th></tr></thead>
      <tbody>
{compare}
      </tbody>
    </table>
  </section>

  <section class="rust" aria-labelledby="rust">
    <div class="wrap">
      <div class="rust-head">
        {icon("rust", "rust-logo")}
        <div>
          <h2 id="rust">{e(t["rust_title"])}</h2>
          <p>{e(t["rust_intro"])}</p>
        </div>
      </div>
      <ul class="stats">
{stats}
      </ul>
    </div>
  </section>

  <section class="wrap" aria-labelledby="screenshots">
    <h2 id="screenshots">{e(t["shots_title"])}</h2>
    <div class="shots">
      <figure>
        {picture(prefix, "welcome", t["welcome_alt"], SHOT_SIZES)}
        <figcaption>{e(t["welcome_caption"])}</figcaption>
      </figure>
      <figure>
        {picture(prefix, "editor-ja", t["langs_alt"], SHOT_SIZES)}
        <figcaption>{e(t["langs_caption"])}</figcaption>
      </figure>
    </div>
  </section>

  <section class="wrap" aria-labelledby="faq">
    <h2 id="faq">{e(t["faq_title"])}</h2>
    <div class="faq">
{faq}
    </div>
  </section>
</main>

<footer>
  <div class="wrap">
    <div>
      <p><strong>Squoosh Desktop</strong> v{e(version)}</p>
      <p>{based_on}</p>
      <p>{e(t["footer_license"])} <a href="{REPO}">{e(t["source"])}</a></p>
    </div>
    <nav aria-label="{e(t["language"])}">
      <ul class="langs">
{lang_links}
      </ul>
    </nav>
  </div>
</footer>
</body>
</html>
"""


def sitemap(base_url, today):
    links = "".join(f'\n    <xhtml:link rel="alternate" hreflang="{c}" href="{base_url + slug(c)}"/>' for c in LANGS)
    links += f'\n    <xhtml:link rel="alternate" hreflang="x-default" href="{base_url}"/>'
    urls = "".join(f"\n  <url>\n    <loc>{base_url + slug(c)}</loc>\n    <lastmod>{today}</lastmod>{links}\n  </url>"
                   for c in LANGS)
    return ('<?xml version="1.0" encoding="UTF-8"?>\n'
            '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" '
            f'xmlns:xhtml="http://www.w3.org/1999/xhtml">{urls}\n</urlset>\n')


def llms_txt(t, locales, version, base_url, dl):
    faq = "\n".join(f"- **{t[f'q{i}']}** {t[f'a{i}']}" for i in FAQ)
    features = "\n".join(f"- **{t[f'f{i}_title']}**: {t[f'f{i}_text']}" for i in range(1, 5))
    rust = "\n".join(f"- **{t[f'r{i}_title']}**: {t[f'r{i}_text']}" for i in range(1, 5))
    batch = "\n".join(f"- **{t[f'b{i}_title']}**: {t[f'b{i}_text']}" for i in range(1, 7))
    yes_no = {True: t["compare_yes"], False: t["compare_no"]}
    compare = "\n".join(f"| {t[f'c{i}']} | {yes_no[web]} | {yes_no[desktop]} |"
                        for i, (web, desktop) in enumerate(COMPARE, 1))
    languages = ", ".join(f"{locales[c]['lang_name']} ({base_url + slug(c)})" for c in LANGS)
    return f"""# Squoosh Desktop

> {t["intro"]}

Squoosh Desktop is a native Rust port of Squoosh (squoosh.app, by Google Chrome Labs) for Windows 10/11 and Linux (x86_64). Latest version: {version}. License: GPL-3.0-or-later.

## Features

{features}

## {t["batch_title"]}

{t["batch_intro"]}

{batch}

## {t["compare_title"]}

| {t["compare_feature"]} | squoosh.app | Squoosh Desktop |
|---|---|---|
{compare}

## Why Rust

{t["rust_intro"]} The MozJPEG, libwebp and libavif codecs are native C libraries; the pipeline, OxiPNG, resizing, palette reduction and the interface are Rust.

{rust}

## FAQ

{faq}

## Links

- [Download the latest release]({dl["releases"]}/latest)
- [Windows installer]({dl["windows"]})
- [Linux AppImage]({dl["appimage"]})
- [Debian package]({dl["deb"]})
- [Source code and documentation]({REPO})
- [Changelog]({REPO}/blob/main/CHANGELOG.md)

## Languages

The website and the app are available in: {languages}.
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--out", type=Path, default=ROOT / "target" / "site")
    parser.add_argument("--version", help="release shown on the page (default: Cargo.toml version)")
    parser.add_argument("--base-url", default=BASE_URL)
    args = parser.parse_args()
    version = (args.version or project_version()).removeprefix("v")
    base_url = args.base_url.rstrip("/") + "/"
    locales = load_locales()

    out = args.out
    if out.exists():
        shutil.rmtree(out)
    shutil.copytree(SITE / "assets", out / "assets", ignore=shutil.ignore_patterns("*.css", "og.html"))
    # Files served as is at the site root, e.g. search engine verification files.
    shutil.copytree(SITE / "static", out, dirs_exist_ok=True)
    for code in LANGS:
        target = out / slug(code) / "index.html"
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(render(code, locales, version, base_url), "utf-8")

    today = date.today().isoformat()
    (out / "sitemap.xml").write_text(sitemap(base_url, today), "utf-8")
    (out / "robots.txt").write_text(f"User-agent: *\nAllow: /\n\nSitemap: {base_url}sitemap.xml\n", "utf-8")
    (out / "llms.txt").write_text(llms_txt(locales["en"], locales, version, base_url, downloads(version)), "utf-8")
    (out / ".nojekyll").write_text("", "utf-8")
    print(f"Site {version}: {len(LANGS)} pages in {out}")


if __name__ == "__main__":
    main()
