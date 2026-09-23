use crate::{
    pipeline::{self, Source},
    settings::Settings,
};
use anyhow::{Context, Result, ensure};
use crossbeam_channel::{Receiver, Sender};
use image::RgbaImage;
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Task {
    pub id: u64,
    pub path: PathBuf,
    pub settings: Settings,
}
pub struct PreviewSide {
    pub image: RgbaImage,
    pub bytes: u64,
    pub dimensions: (u32, u32),
}
pub enum Event {
    Imported(Vec<PathBuf>),
    ImportError(String),
    Inspected {
        id: u64,
        thumbnail: RgbaImage,
        dimensions: (u32, u32),
        bytes: u64,
        warning: Option<String>,
    },
    InspectError {
        id: u64,
        error: String,
    },
    Preview {
        generation: u64,
        sides: [Result<PreviewSide, String>; 2],
        warning: Option<String>,
    },
    PreviewError {
        generation: u64,
        error: String,
    },
    Stage {
        id: u64,
        stage: String,
    },
    Finished {
        id: u64,
        result: Result<(PathBuf, u64), String>,
    },
    BatchDone {
        cancelled: bool,
    },
}
pub enum Command {
    Import {
        paths: Vec<PathBuf>,
        recursive: bool,
    },
    Inspect {
        id: u64,
        path: PathBuf,
    },
    Preview {
        generation: u64,
        path: PathBuf,
        settings: Box<Settings>,
    },
    Batch {
        tasks: Vec<Task>,
        directory: PathBuf,
        cancel: Arc<AtomicBool>,
    },
    Stop,
}
pub struct Worker {
    pub commands: Sender<Command>,
    pub events: Receiver<Event>,
    pub latest_preview: Arc<std::sync::atomic::AtomicU64>,
}
fn collect(paths: Vec<PathBuf>, recursive: bool) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut directories = HashSet::new();
    let mut pending: Vec<(PathBuf, bool)> = paths.into_iter().map(|p| (p, true)).collect();
    while let Some((path, explicit)) = pending.pop() {
        if path.is_dir() {
            let canonical = dunce::canonicalize(&path)?;
            if !directories.insert(canonical) {
                continue;
            }
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                if kind.is_dir() {
                    if recursive {
                        pending.push((entry.path(), false));
                    }
                } else if !kind.is_symlink() {
                    pending.push((entry.path(), false));
                }
            }
        } else if explicit || pipeline::supported(&path) {
            let canonical = dunce::canonicalize(&path).unwrap_or(path);
            if seen.insert(canonical.clone()) {
                out.push(canonical);
            }
        }
        ensure!(
            out.len() + pending.len() <= 100_000,
            "Limite de 100 000 fichiers dépassée"
        );
    }
    out.sort();
    Ok(out)
}
fn display_side(source: &Source, settings: &Settings, index: usize) -> Result<PreviewSide> {
    let (image, bytes) = if settings.sides[index].format.is_none() {
        (
            pipeline::rotate(&source.image, settings.rotation),
            source.bytes,
        )
    } else {
        let c = pipeline::convert(source, settings, index, &AtomicBool::new(false), |_| {})?;
        (c.image, c.bytes.len() as u64)
    };
    let dimensions = image.dimensions();
    // Keep native pixels for useful 1:1 inspection, subject to the source pixel limit.
    Ok(PreviewSide {
        image,
        bytes,
        dimensions,
    })
}
/// Antivirus scanners on Windows briefly lock freshly written files, which
/// makes the rename fail with a sharing violation: retry for about a second.
fn persist_noclobber(
    mut temp: tempfile::NamedTempFile,
    target: &Path,
) -> Result<fs::File, tempfile::PersistError> {
    for _ in 0..20 {
        match temp.persist_noclobber(target) {
            Err(e) if cfg!(windows) && matches!(e.error.raw_os_error(), Some(5 | 32)) => {
                temp = e.file;
                thread::sleep(Duration::from_millis(50));
            }
            result => return result,
        }
    }
    temp.persist_noclobber(target)
}
pub fn write_output(
    directory: &Path,
    input: &Path,
    extension: &str,
    bytes: &[u8],
    cancel: &AtomicBool,
) -> Result<PathBuf> {
    ensure!(directory.is_dir(), "Le dossier de sortie n’existe pas");
    let mut temp = tempfile::NamedTempFile::new_in(directory)
        .context("Dossier de sortie non accessible en écriture")?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    let stem = input.file_stem().unwrap_or(std::ffi::OsStr::new("image"));
    for suffix in 0..100_000 {
        pipeline::check_cancel(cancel)?;
        let mut name = stem.to_os_string();
        if suffix > 0 {
            name.push(format!("_{suffix}"));
        }
        name.push(format!(".{extension}"));
        let target = directory.join(name);
        // Also avoid replacing the input through lexical/canonical aliases.
        if target == input {
            continue;
        }
        match persist_noclobber(temp, &target) {
            Ok(_) => return Ok(target),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                temp = error.file
            }
            Err(error) => return Err(error.error.into()),
        }
    }
    anyhow::bail!("Trop de noms de sortie en conflit")
}
impl Worker {
    pub fn new(wake: impl Fn() + Send + 'static) -> Self {
        let (tx, rx) = crossbeam_channel::unbounded();
        let (events_tx, events) = crossbeam_channel::bounded(4);
        let latest_preview = Arc::new(std::sync::atomic::AtomicU64::new(0));
        let latest = latest_preview.clone();
        thread::Builder::new()
            .name("squoosh-conversion".into())
            .spawn(move || {
                let send = |event| {
                    let ok = events_tx.send(event).is_ok();
                    wake();
                    ok
                };
                while let Ok(command) = rx.recv() {
                    let outcome =
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match command {
                            Command::Stop => false,
                            Command::Import { paths, recursive } => {
                                match collect(paths, recursive) {
                                    Ok(paths) => {
                                        send(Event::Imported(paths));
                                    }
                                    Err(e) => {
                                        send(Event::ImportError(format!("{e:#}")));
                                    }
                                }
                                true
                            }
                            Command::Inspect { id, path } => {
                                match pipeline::load(&path) {
                                    Ok(source) => {
                                        let dimensions = source.image.dimensions();
                                        send(Event::Inspected {
                                            id,
                                            thumbnail: image::imageops::thumbnail(
                                                &source.image,
                                                64,
                                                64,
                                            ),
                                            dimensions,
                                            bytes: source.bytes,
                                            warning: source.warning,
                                        });
                                    }
                                    Err(e) => {
                                        send(Event::InspectError {
                                            id,
                                            error: format!("{e:#}"),
                                        });
                                    }
                                }
                                true
                            }
                            Command::Preview {
                                generation,
                                path,
                                settings,
                            } => {
                                if latest.load(Ordering::Relaxed) != generation {
                                    return true;
                                }
                                match pipeline::load(&path) {
                                    Ok(source) => {
                                        let left = display_side(&source, &settings, 0)
                                            .map_err(|e| format!("{e:#}"));
                                        if latest.load(Ordering::Relaxed) != generation {
                                            return true;
                                        }
                                        let right = display_side(&source, &settings, 1)
                                            .map_err(|e| format!("{e:#}"));
                                        if latest.load(Ordering::Relaxed) == generation {
                                            send(Event::Preview {
                                                generation,
                                                sides: [left, right],
                                                warning: source.warning,
                                            });
                                        }
                                    }
                                    Err(e) => {
                                        send(Event::PreviewError {
                                            generation,
                                            error: format!("{e:#}"),
                                        });
                                    }
                                }
                                true
                            }
                            Command::Batch {
                                tasks,
                                directory,
                                cancel,
                            } => {
                                for task in tasks {
                                    if cancel.load(Ordering::Relaxed) {
                                        break;
                                    }
                                    let result = (|| -> Result<(PathBuf, u64)> {
                                        send(Event::Stage {
                                            id: task.id,
                                            stage: "Lecture".into(),
                                        });
                                        let source = pipeline::load(&task.path)?;
                                        let result = pipeline::convert(
                                            &source,
                                            &task.settings,
                                            task.settings.export_side,
                                            &cancel,
                                            |stage| {
                                                send(Event::Stage {
                                                    id: task.id,
                                                    stage: stage.into(),
                                                });
                                            },
                                        )?;
                                        send(Event::Stage {
                                            id: task.id,
                                            stage: "Écriture".into(),
                                        });
                                        let output = write_output(
                                            &directory,
                                            &task.path,
                                            result.format.extension(),
                                            &result.bytes,
                                            &cancel,
                                        )?;
                                        Ok((output, result.bytes.len() as u64))
                                    })()
                                    .map_err(|e| format!("{e:#}"));
                                    if !send(Event::Finished {
                                        id: task.id,
                                        result,
                                    }) {
                                        return false;
                                    }
                                }
                                send(Event::BatchDone {
                                    cancelled: cancel.load(Ordering::Relaxed),
                                });
                                true
                            }
                        }));
                    match outcome {
                        Ok(true) => {}
                        Ok(false) => break,
                        Err(_) => {
                            send(Event::ImportError(
                                "Erreur interne du traitement ; le moteur a été réinitialisé"
                                    .into(),
                            ));
                            send(Event::BatchDone { cancelled: true });
                        }
                    }
                }
            })
            .expect("conversion worker");
        Self {
            commands: tx,
            events,
            latest_preview,
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Stop);
    }
}
