fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=windows.manifest");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    // Windows gives the main thread 1 MiB of stack, Linux 8 MiB: keep the
    // Linux headroom for deep SVG documents and egui layouts.
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        println!("cargo:rustc-link-arg-bins=/STACK:8388608");
    }
    // The manifest takes a four-part numeric version; winresource fills the
    // file and product versions from CARGO_PKG_VERSION itself.
    let version = ["MAJOR", "MINOR", "PATCH"]
        .map(|part| std::env::var(format!("CARGO_PKG_VERSION_{part}")).unwrap())
        .join(".");
    let manifest = std::fs::read_to_string("windows.manifest")
        .unwrap()
        .replace("@VERSION@", &format!("{version}.0"));
    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico")
        .set_manifest(&manifest)
        .set("ProductName", "Squoosh Desktop")
        .set("FileDescription", "Squoosh Desktop")
        .set("LegalCopyright", "GPL-3.0-or-later");
    res.compile().expect("ressources Windows");
}
