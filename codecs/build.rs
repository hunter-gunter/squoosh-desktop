use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-changed=src/native.c");
    println!("cargo:rerun-if-changed=options.json");
    println!("cargo:rerun-if-env-changed=SQUOOSH_NATIVE_PREFIX");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let specs: serde_json::Value =
        serde_json::from_str(&fs::read_to_string("options.json").unwrap()).unwrap();
    let mut header = String::from("#include <stdint.h>\n");
    for codec in ["jpeg", "webp", "avif"] {
        header.push_str("typedef struct {\n");
        for field in specs[codec].as_array().unwrap() {
            header.push_str(&format!("int32_t {};\n", field["key"].as_str().unwrap()));
        }
        header.push_str(&format!("}} {}_options;\n", codec));
    }
    fs::write(out.join("options.h"), header).unwrap();
    let mut build = cc::Build::new();
    build
        .file("src/native.c")
        .include(&out)
        .flag_if_supported("-std=c11")
        // MSVC reads sources in the ANSI code page unless told otherwise,
        // which would garble the French error messages.
        .flag_if_supported("/utf-8");
    for path in
        env::split_paths(&env::var_os("DEP_JPEG_INCLUDE").expect("MozJPEG include directories"))
    {
        build.include(path);
    }
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        link_msvc_prefix(&mut build);
    } else {
        for (lib, min) in [("libwebp", "1.2"), ("libavif", "1.2"), ("lcms2", "2.12")] {
            for path in pkg_config::Config::new()
                .atleast_version(min)
                .probe(lib)
                .unwrap()
                .include_paths
            {
                build.include(path);
            }
        }
    }
    build.compile("squoosh_native");
}
/// Windows links the static libraries of a vcpkg `x64-windows-static` tree
/// (see `vcpkg.json`), so the executable needs no third-party DLL.
fn link_msvc_prefix(build: &mut cc::Build) {
    let prefix = PathBuf::from(env::var_os("SQUOOSH_NATIVE_PREFIX").expect(
        "SQUOOSH_NATIVE_PREFIX doit désigner vcpkg_installed/x64-windows-static (voir scripts/windows.ps1)",
    ));
    let lib = prefix.join("lib");
    // After MozJPEG's directories: the tree also holds libjpeg-turbo's
    // `jpeglib.h` (a libyuv dependency), which must not shadow MozJPEG's.
    build.include(prefix.join("include"));
    println!("cargo:rustc-link-search=native={}", lib.display());
    // An explicit list: libjpeg-turbo is never linked, so every `jpeg_*`
    // symbol resolves to MozJPEG. libyuv's MJPEG path, the only user of
    // libjpeg-turbo, is not reachable from libavif.
    for (names, required) in [
        (&["avif"][..], true),
        (&["aom"], true),
        (&["yuv"], false),
        (&["libwebp", "webp"], true),
        (&["libsharpyuv", "sharpyuv"], false),
        (&["lcms2", "liblcms2"], true),
    ] {
        match names.iter().find(|n| lib.join(format!("{n}.lib")).exists()) {
            Some(name) => println!("cargo:rustc-link-lib=static={name}"),
            None => assert!(!required, "{}.lib absent de {}", names[0], lib.display()),
        }
    }
}
