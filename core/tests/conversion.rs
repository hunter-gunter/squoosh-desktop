use image::{DynamicImage, ImageEncoder, Rgba, RgbaImage};
use squoosh_codecs::{Format, defaults};
use squoosh_core::{
    jobs::{Command, Event, Task, Worker, write_output},
    pipeline::{self, Source},
    settings::{Filter, Overrides, Settings},
};
use std::{
    fs,
    io::Cursor,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

fn pixels() -> RgbaImage {
    RgbaImage::from_fn(32, 24, |x, y| {
        Rgba([
            (x * 7) as u8,
            (y * 9) as u8,
            ((x + y) * 4) as u8,
            if x < 8 {
                0
            } else if x < 16 {
                128
            } else {
                255
            },
        ])
    })
}
fn source() -> Source {
    Source {
        image: pixels(),
        bytes: 2000,
        warning: None,
        svg: None,
    }
}
fn settings(format: Format) -> Settings {
    let mut settings = Settings::default();
    settings.sides[1].format = Some(format);
    settings
}
fn run(source: &Source, settings: &Settings) -> pipeline::Converted {
    pipeline::convert(source, settings, 1, &AtomicBool::new(false), |_| {}).unwrap()
}

#[test]
fn all_input_output_combinations() {
    let dir = tempfile::tempdir().unwrap();
    for format in Format::ALL {
        fs::write(
            dir.path().join(format!("input.{}", format.extension())),
            squoosh_codecs::encode(&pixels(), format, &defaults(format)).unwrap(),
        )
        .unwrap();
    }
    for (ext, format) in [
        ("bmp", image::ImageFormat::Bmp),
        ("tiff", image::ImageFormat::Tiff),
        ("gif", image::ImageFormat::Gif),
    ] {
        let mut data = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(pixels())
            .write_to(&mut data, format)
            .unwrap();
        fs::write(dir.path().join(format!("input.{ext}")), data.into_inner()).unwrap();
    }
    fs::write(dir.path().join("input.svg"),r#"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="24"><rect width="32" height="24" fill="red" opacity="0.5"/></svg>"#).unwrap();
    for entry in fs::read_dir(dir.path()).unwrap() {
        let path = entry.unwrap().path();
        let source = pipeline::load(&path).unwrap_or_else(|e| panic!("{}: {e:#}", path.display()));
        for format in Format::ALL {
            let result = run(&source, &settings(format));
            assert_eq!(
                result.image.dimensions(),
                (32, 24),
                "{} -> {format:?}",
                path.display()
            );
            assert!(!result.bytes.is_empty());
        }
    }
}
#[test]
fn png_and_lossless_modes_preserve_pixels() {
    for format in [Format::Png, Format::Webp, Format::Avif] {
        let source = source();
        let mut s = settings(format);
        let opts = s.sides[1].options.get_mut(format.key()).unwrap();
        if format == Format::Webp {
            opts.insert("lossless".into(), 1);
            opts.insert("exact".into(), 1);
        }
        if format == Format::Avif {
            opts.insert("quality".into(), 100);
            opts.insert("qualityAlpha".into(), 100);
            opts.insert("subsample".into(), 3);
        }
        let result = run(&source, &s);
        assert_eq!(result.image, source.image, "{format:?}");
    }
}
#[test]
fn jpeg_background_and_arithmetic_and_advanced_options() {
    let mut s = settings(Format::Jpeg);
    s.sides[1].background = [12, 85, 190];
    for (key, value) in [
        ("arithmetic", 1),
        ("progressive", 0),
        ("baseline", 1),
        ("color_space", 1),
        ("color_space", 2),
        ("quant_table", 8),
        ("trellis_multipass", 1),
        ("trellis_opt_zero", 1),
        ("trellis_opt_table", 1),
        ("trellis_loops", 2),
        ("auto_subsample", 0),
        ("chroma_subsample", 4),
        ("separate_chroma_quality", 1),
        ("chroma_quality", 95),
    ] {
        let mut case = s.clone();
        case.sides[1]
            .options
            .get_mut("jpeg")
            .unwrap()
            .insert(key.into(), value);
        let result = run(&source(), &case);
        assert!(result.image.pixels().all(|p| p[3] == 255));
    }
    s.sides[1]
        .options
        .get_mut("jpeg")
        .unwrap()
        .insert("quality".into(), 100);
    let image = run(&source(), &s).image;
    let p = image.get_pixel(2, 12);
    for (actual, expected) in p.0[..3].iter().zip([12_i32, 85, 190]) {
        assert!((i32::from(*actual) - expected).abs() < 8);
    }
}

#[test]
fn webp_and_avif_advanced_options_reach_native_codecs() {
    let webp_cases = [
        ("target_size", 2_000),
        ("target_PSNR", 35),
        ("method", 6),
        ("sns_strength", 80),
        ("filter_strength", 20),
        ("filter_sharpness", 7),
        ("filter_type", 0),
        ("partitions", 3),
        ("segments", 1),
        ("pass", 3),
        ("preprocessing", 1),
        ("autofilter", 1),
        ("partition_limit", 40),
        ("alpha_compression", 0),
        ("alpha_filtering", 2),
        ("alpha_quality", 60),
        ("exact", 1),
        ("image_hint", 3),
        ("emulate_jpeg_size", 1),
        ("thread_level", 1),
        ("low_memory", 1),
        ("use_sharp_yuv", 1),
    ];
    for (key, value) in webp_cases {
        let mut case = settings(Format::Webp);
        case.sides[1]
            .options
            .get_mut("webp")
            .unwrap()
            .insert(key.into(), value);
        let result = run(&source(), &case);
        assert_eq!(result.image.dimensions(), (32, 24), "WebP {key}");
    }

    let avif_cases = [
        ("qualityAlpha", 70),
        ("denoiseLevel", 10),
        ("tileColsLog2", 1),
        ("tileRowsLog2", 1),
        ("speed", 10),
        ("subsample", 2),
        ("chromaDeltaQ", 1),
        ("sharpness", 7),
        ("tune", 1),
        ("enableSharpYUV", 1),
    ];
    for (key, value) in avif_cases {
        let mut case = settings(Format::Avif);
        case.sides[1]
            .options
            .get_mut("avif")
            .unwrap()
            .insert(key.into(), value);
        let result = run(&source(), &case);
        assert_eq!(result.image.dimensions(), (32, 24), "AVIF {key}");
    }
}
#[test]
fn resize_rotation_filters_and_palette() {
    for filter in Filter::ALL.into_iter().filter(|f| *f != Filter::Vector) {
        let mut s = settings(Format::Png);
        s.rotation = 90;
        s.sides[1].resize.enabled = true;
        s.sides[1].resize.width = 36;
        s.sides[1].resize.filter = filter;
        assert_eq!(
            run(&source(), &s).image.dimensions(),
            (36, 48),
            "{filter:?}"
        );
    }
    let mut s = settings(Format::Png);
    s.sides[1].resize.enabled = true;
    s.sides[1].resize.width = 20;
    s.sides[1].resize.height = 20;
    s.sides[1].resize.lock_ratio = false;
    s.sides[1].resize.crop = true;
    assert_eq!(run(&source(), &s).image.dimensions(), (20, 20));
    s.sides[1].palette.enabled = true;
    s.sides[1].palette.colors = 8;
    let image = run(&source(), &s).image;
    assert!(
        image
            .pixels()
            .map(|p| p.0)
            .collect::<std::collections::HashSet<_>>()
            .len()
            <= 8
    );
    s.sides[1].palette.zx = true;
    let image = run(&source(), &s).image;
    for y in (0..image.height()).step_by(8) {
        for x in (0..image.width()).step_by(8) {
            let block = image::imageops::crop_imm(
                &image,
                x,
                y,
                8.min(image.width() - x),
                8.min(image.height() - y),
            )
            .to_image();
            assert!(
                block
                    .pixels()
                    .map(|p| p.0)
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    <= 2
            );
        }
    }
}
#[test]
fn vector_svg_resolution_and_external_resources() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("image.svg");
    fs::write(&path,r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="20" height="10" fill="blue"/></svg>"#).unwrap();
    let source = pipeline::load(&path).unwrap();
    let mut s = settings(Format::Png);
    s.rotation = 90;
    s.sides[1].resize.enabled = true;
    s.sides[1].resize.width = 100;
    s.sides[1].resize.filter = Filter::Vector;
    let result = run(&source, &s);
    assert_eq!(result.image.dimensions(), (100, 200));
    assert_eq!(result.image.get_pixel(50, 100).0, [0, 0, 255, 255]);
}
#[test]
fn vector_filter_falls_back_on_raster_sources() {
    let mut s = settings(Format::Png);
    s.sides[1].resize.enabled = true;
    s.sides[1].resize.width = 16;
    s.sides[1].resize.filter = Filter::Vector;
    let result = run(&source(), &s);
    assert_eq!(result.image.dimensions(), (16, 12));
}
#[test]
fn icc_and_exif_orientation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.png");
    let profile = lcms2::Profile::new_srgb().icc().unwrap();
    let mut bytes = Vec::new();
    let mut encoder = image::codecs::png::PngEncoder::new(&mut bytes);
    encoder.set_icc_profile(profile).unwrap();
    encoder
        .write_image(pixels().as_raw(), 32, 24, image::ExtendedColorType::Rgba8)
        .unwrap();
    fs::write(&path, bytes).unwrap();
    assert_eq!(pipeline::load(&path).unwrap().image, pixels());
    let jpeg = squoosh_codecs::encode(&pixels(), Format::Jpeg, &defaults(Format::Jpeg)).unwrap();
    // EXIF little-endian TIFF, one SHORT Orientation=6 entry.
    let exif = b"Exif\0\0II\x2a\0\x08\0\0\0\x01\0\x12\x01\x03\0\x01\0\0\0\x06\0\0\0\0\0\0\0";
    let mut oriented = vec![0xff, 0xd8, 0xff, 0xe1];
    oriented.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
    oriented.extend_from_slice(exif);
    oriented.extend_from_slice(&jpeg[2..]);
    let path = dir.path().join("oriented.jpg");
    fs::write(&path, oriented).unwrap();
    assert_eq!(pipeline::load(&path).unwrap().image.dimensions(), (24, 32));
}
#[test]
fn field_overrides_preserve_inheritance_and_snapshot() {
    let mut common = Settings::default();
    let mut individual = common.clone();
    individual.sides[1]
        .options
        .get_mut("jpeg")
        .unwrap()
        .insert("quality".into(), 91);
    let mut overrides = Overrides::default();
    overrides.record(&common, &individual);
    common.rotation = 90;
    common.sides[1]
        .options
        .get_mut("jpeg")
        .unwrap()
        .insert("quality".into(), 40);
    let snapshot = overrides.resolve(&common).unwrap();
    assert_eq!(snapshot.rotation, 90);
    assert_eq!(snapshot.sides[1].options["jpeg"]["quality"], 91);
    common.rotation = 180;
    assert_eq!(snapshot.rotation, 90);
    assert_eq!(Overrides::default().resolve(&common).unwrap(), common);
}
#[test]
fn output_collisions_cancel_and_error_paths() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("same.png");
    fs::write(&input, b"original").unwrap();
    let cancel = AtomicBool::new(false);
    let a = write_output(dir.path(), &input, "png", b"first", &cancel).unwrap();
    let b = write_output(dir.path(), &input, "png", b"second", &cancel).unwrap();
    assert_ne!(a, b);
    assert_eq!(fs::read(&input).unwrap(), b"original");
    cancel.store(true, Ordering::Relaxed);
    assert!(write_output(dir.path(), &input, "png", b"cancelled", &cancel).is_err());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 3);
    assert!(
        write_output(
            &dir.path().join("missing"),
            &input,
            "png",
            b"x",
            &AtomicBool::new(false)
        )
        .is_err()
    );
    assert!(pipeline::load(&input).is_err());
    let mut invalid = settings(Format::Webp);
    invalid.sides[1]
        .options
        .get_mut("webp")
        .unwrap()
        .insert("quality".into(), 101);
    assert!(pipeline::convert(&source(), &invalid, 1, &AtomicBool::new(false), |_| {}).is_err());
}
#[test]
fn batch_100_images_isolates_error_and_freezes_settings() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.png");
    pixels().save(&input).unwrap();
    let output = dir.path().join("output");
    fs::create_dir(&output).unwrap();
    let corrupt = dir.path().join("bad.png");
    fs::write(&corrupt, b"broken").unwrap();
    let worker = Worker::new(|| {});
    let mut s = settings(Format::Png);
    s.sides[1]
        .options
        .get_mut("png")
        .unwrap()
        .insert("level".into(), 0);
    let tasks = (0..100)
        .map(|id| Task {
            id,
            path: if id == 50 {
                corrupt.clone()
            } else {
                input.clone()
            },
            settings: s.clone(),
        })
        .collect();
    worker
        .commands
        .send(Command::Batch {
            tasks,
            directory: output.clone(),
            cancel: Arc::new(AtomicBool::new(false)),
        })
        .unwrap();
    s.sides[1].format = Some(Format::Jpeg);
    let (mut successes, mut errors) = (0, 0);
    loop {
        match worker.events.recv_timeout(Duration::from_secs(60)).unwrap() {
            Event::Finished { result, .. } => {
                if result.is_ok() {
                    successes += 1;
                } else {
                    errors += 1;
                }
            }
            Event::BatchDone { cancelled } => {
                assert!(!cancelled);
                break;
            }
            _ => {}
        }
    }
    assert_eq!((successes, errors), (99, 1));
    assert_eq!(fs::read_dir(output).unwrap().count(), 99);
}
#[test]
fn stale_previews_and_cancelled_batch() {
    let worker = Worker::new(|| {});
    worker.latest_preview.store(2, Ordering::Relaxed);
    worker
        .commands
        .send(Command::Preview {
            generation: 1,
            path: "does-not-exist.png".into(),
            settings: Box::new(Settings::default()),
        })
        .unwrap();
    worker
        .commands
        .send(Command::Batch {
            tasks: vec![],
            directory: "missing".into(),
            cancel: Arc::new(AtomicBool::new(true)),
        })
        .unwrap();
    assert!(matches!(
        worker.events.recv_timeout(Duration::from_secs(10)).unwrap(),
        Event::BatchDone { cancelled: true }
    ));
}
#[test]
fn import_deduplicates_and_respects_recursion() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("a.png"), b"a").unwrap();
    fs::create_dir(dir.path().join("child")).unwrap();
    fs::write(dir.path().join("child/b.png"), b"b").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(dir.path(), dir.path().join("child/loop")).unwrap();
    // Symbolic links need Developer Mode or elevation on Windows.
    #[cfg(windows)]
    let _ = std::os::windows::fs::symlink_dir(dir.path(), dir.path().join("child/loop"));
    let worker = Worker::new(|| {});
    for (recursive, count) in [(false, 1), (true, 2)] {
        worker
            .commands
            .send(Command::Import {
                paths: vec![dir.path().into(), dir.path().join("a.png")],
                recursive,
            })
            .unwrap();
        match worker.events.recv_timeout(Duration::from_secs(10)).unwrap() {
            Event::Imported(files) => assert_eq!(files.len(), count),
            _ => panic!("import error"),
        }
    }
}
