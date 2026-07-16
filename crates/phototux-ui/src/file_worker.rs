//! Background document file operations for the desktop session.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use phototux_engine::{CancelToken, DocumentGraph};
use phototux_io::{
    CompatibilityIssue, PtxDocument, Raster, RasterFormat, import_psd_path, load_ptx,
    save_ptx_atomic, write_autosave,
};

pub(crate) enum FileCommand {
    Open(PathBuf),
    OpenPtx(PathBuf),
    OpenPsd(PathBuf),
    SavePtx {
        path: PathBuf,
        graph: DocumentGraph,
    },
    Export {
        path: PathBuf,
        format: RasterFormat,
    },
    Autosave {
        graph: DocumentGraph,
        original: Option<PathBuf>,
    },
    Shutdown,
}

pub(crate) enum FileEvent {
    Opened {
        path: PathBuf,
        raster: Raster,
    },
    PtxOpened {
        path: PathBuf,
        document: PtxDocument,
    },
    PsdOpened {
        path: PathBuf,
        graph: DocumentGraph,
        raster: Raster,
        report: Vec<CompatibilityIssue>,
    },
    Saved {
        path: PathBuf,
    },
    Autosaved,
    Exported {
        path: PathBuf,
    },
    Failed {
        operation: &'static str,
        message: String,
    },
    Cancelled {
        operation: &'static str,
    },
}

pub(crate) struct FileWorker {
    commands: Option<Sender<FileCommand>>,
    events: Receiver<FileEvent>,
    join: Option<JoinHandle<()>>,
    start_error: Option<String>,
    cancel: CancelToken,
}

impl FileWorker {
    pub(crate) fn start() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let cancel = CancelToken::new();
        let cancel_worker = cancel.clone();
        match thread::Builder::new()
            .name("phototux-file-io".into())
            .spawn(move || worker_loop(command_rx, event_tx, cancel_worker))
        {
            Ok(join) => Self {
                commands: Some(command_tx),
                events: event_rx,
                join: Some(join),
                start_error: None,
                cancel,
            },
            Err(error) => Self {
                commands: None,
                events: event_rx,
                join: None,
                start_error: Some(format!("failed to spawn raster I/O worker: {error}")),
                cancel,
            },
        }
    }

    pub(crate) fn start_error(&self) -> Option<&str> {
        self.start_error.as_deref()
    }

    pub(crate) fn cancel_token(&self) -> &CancelToken {
        &self.cancel
    }

    pub(crate) fn send(&self, command: FileCommand) -> Result<(), String> {
        let Some(commands) = self.commands.as_ref() else {
            return Err(self
                .start_error
                .clone()
                .unwrap_or_else(|| "raster I/O worker unavailable".to_owned()));
        };
        self.cancel.reset();
        commands
            .send(command)
            .map_err(|_| "raster I/O worker stopped".to_owned())
    }

    pub(crate) fn poll_events(&self) -> Vec<FileEvent> {
        self.events.try_iter().collect()
    }
}

impl Drop for FileWorker {
    fn drop(&mut self) {
        if let Some(commands) = self.commands.take() {
            let _ = commands.send(FileCommand::Shutdown);
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn worker_loop(commands: Receiver<FileCommand>, events: Sender<FileEvent>, cancel: CancelToken) {
    while let Ok(command) = commands.recv() {
        let event = match command {
            FileCommand::Open(path) => match phototux_io::decode_path(&path) {
                Ok(raster) => FileEvent::Opened { path, raster },
                Err(error) => FileEvent::Failed {
                    operation: "Open",
                    message: error.to_string(),
                },
            },
            FileCommand::OpenPtx(path) => match load_ptx(&path) {
                Ok(document) => FileEvent::PtxOpened { path, document },
                Err(error) => FileEvent::Failed {
                    operation: "Open",
                    message: error.to_string(),
                },
            },
            FileCommand::OpenPsd(path) => match import_psd_path(&path) {
                Ok(imported) => {
                    let raster = match imported.flattened {
                        Some(raster) => Ok(raster),
                        None => placeholder_raster(),
                    };
                    match raster {
                        Ok(raster) => FileEvent::PsdOpened {
                            path,
                            graph: imported.graph,
                            raster,
                            report: imported.report,
                        },
                        Err(message) => FileEvent::Failed {
                            operation: "Open",
                            message,
                        },
                    }
                }
                Err(error) => FileEvent::Failed {
                    operation: "Open",
                    message: error.to_string(),
                },
            },
            FileCommand::SavePtx { path, graph } => save_document(path, graph, &cancel),
            FileCommand::Autosave { graph, original } => {
                autosave_document(graph, original, &cancel)
            }
            FileCommand::Export { path, format } => export_document(path, format, &cancel),
            FileCommand::Shutdown => break,
        };
        if events.send(event).is_err() {
            break;
        }
    }
}

fn save_document(path: PathBuf, graph: DocumentGraph, cancel: &CancelToken) -> FileEvent {
    if cancel.is_cancelled() {
        return FileEvent::Cancelled { operation: "Save" };
    }
    let result = (|| {
        let layers = phototux_canvas::read_all_layer_rgba().map_err(|e| e.to_string())?;
        if cancel.is_cancelled() {
            return Err("cancelled".to_owned());
        }
        let mut rasters = HashMap::new();
        for (id, width, height, pixels) in layers {
            let raster =
                Raster::new(width, height, pixels.into_boxed_slice()).map_err(|e| e.to_string())?;
            rasters.insert(id.0, raster);
        }
        let (mw, mh) = (graph.size.width, graph.size.height);
        let mut doc = PtxDocument::from_graph(graph, rasters);
        doc.masks = collect_mask_rasters(mw, mh)?;
        save_ptx_atomic(&path, &doc).map_err(|e| e.to_string())
    })();
    match result {
        Ok(()) => FileEvent::Saved { path },
        Err(message) if message == "cancelled" || cancel.is_cancelled() => {
            FileEvent::Cancelled { operation: "Save" }
        }
        Err(message) => FileEvent::Failed {
            operation: "Save",
            message,
        },
    }
}

fn autosave_document(
    graph: DocumentGraph,
    original: Option<PathBuf>,
    cancel: &CancelToken,
) -> FileEvent {
    if cancel.is_cancelled() {
        return FileEvent::Cancelled {
            operation: "Autosave",
        };
    }
    let result = (|| {
        let layers = phototux_canvas::read_all_layer_rgba().map_err(|e| e.to_string())?;
        let mut rasters = HashMap::new();
        for (id, width, height, pixels) in layers {
            let raster =
                Raster::new(width, height, pixels.into_boxed_slice()).map_err(|e| e.to_string())?;
            rasters.insert(id.0, raster);
        }
        let (mw, mh) = (graph.size.width, graph.size.height);
        let mut doc = PtxDocument::from_graph(graph, rasters);
        doc.masks = collect_mask_rasters(mw, mh)?;
        write_autosave(&doc, original.as_deref()).map_err(|e| e.to_string())
    })();
    match result {
        Ok(_) => FileEvent::Autosaved,
        Err(message) => FileEvent::Failed {
            operation: "Autosave",
            message,
        },
    }
}

fn placeholder_raster() -> Result<Raster, String> {
    Raster::new(1, 1, vec![0, 0, 0, 255].into_boxed_slice()).map_err(|error| error.to_string())
}

/// Convert GPU R8 masks into grayscale RGBA rasters for `.ptx` persistence.
fn collect_mask_rasters(width: u32, height: u32) -> Result<HashMap<u64, Raster>, String> {
    let masks = phototux_canvas::read_all_mask_r8().map_err(|e| e.to_string())?;
    let expected = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| "mask size overflow".to_owned())?;
    let mut out = HashMap::with_capacity(masks.len());
    for (id, r8) in masks {
        if r8.len() != expected {
            return Err(format!(
                "mask for layer {} has {} bytes; expected {expected}",
                id.0,
                r8.len()
            ));
        }
        let mut rgba = Vec::with_capacity(expected * 4);
        for &v in &r8 {
            rgba.extend_from_slice(&[v, v, v, 255]);
        }
        let raster =
            Raster::new(width, height, rgba.into_boxed_slice()).map_err(|e| e.to_string())?;
        out.insert(id.0, raster);
    }
    Ok(out)
}

fn export_document(path: PathBuf, format: RasterFormat, cancel: &CancelToken) -> FileEvent {
    if cancel.is_cancelled() {
        return FileEvent::Cancelled {
            operation: "Export",
        };
    }
    let result = (|| {
        let (width, height, pixels) =
            phototux_canvas::read_composite_rgba().map_err(|error| error.to_string())?;
        let raster = Raster::new(width, height, pixels.into_boxed_slice())
            .map_err(|error| error.to_string())?;
        phototux_io::encode_path_atomic(&path, &raster, format).map_err(|error| error.to_string())
    })();

    match result {
        Ok(()) => FileEvent::Exported { path },
        Err(message) => FileEvent::Failed {
            operation: "Export",
            message,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_reports_no_error_on_host() {
        let worker = FileWorker::start();
        assert!(worker.start_error().is_none());
        worker.send(FileCommand::Shutdown).expect("shutdown");
    }
}
