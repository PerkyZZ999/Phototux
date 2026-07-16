//! Background raster file operations for the desktop session.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use phototux_io::{Raster, RasterFormat};

pub(crate) enum FileCommand {
    Open(PathBuf),
    Export { path: PathBuf, format: RasterFormat },
    Shutdown,
}

pub(crate) enum FileEvent {
    Opened {
        path: PathBuf,
        raster: Raster,
    },
    Exported {
        path: PathBuf,
    },
    Failed {
        operation: &'static str,
        message: String,
    },
}

pub(crate) struct FileWorker {
    commands: Sender<FileCommand>,
    events: Receiver<FileEvent>,
    join: Option<JoinHandle<()>>,
}

impl FileWorker {
    pub(crate) fn start() -> Self {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let join = thread::Builder::new()
            .name("phototux-file-io".into())
            .spawn(move || worker_loop(command_rx, event_tx))
            .expect("spawn raster I/O worker");
        Self {
            commands: command_tx,
            events: event_rx,
            join: Some(join),
        }
    }

    pub(crate) fn send(&self, command: FileCommand) -> Result<(), String> {
        self.commands
            .send(command)
            .map_err(|_| "raster I/O worker stopped".to_owned())
    }

    pub(crate) fn poll_events(&self) -> Vec<FileEvent> {
        self.events.try_iter().collect()
    }
}

impl Drop for FileWorker {
    fn drop(&mut self) {
        let _ = self.commands.send(FileCommand::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn worker_loop(commands: Receiver<FileCommand>, events: Sender<FileEvent>) {
    while let Ok(command) = commands.recv() {
        let event = match command {
            FileCommand::Open(path) => match phototux_io::decode_path(&path) {
                Ok(raster) => FileEvent::Opened { path, raster },
                Err(error) => FileEvent::Failed {
                    operation: "Open",
                    message: error.to_string(),
                },
            },
            FileCommand::Export { path, format } => export_document(path, format),
            FileCommand::Shutdown => break,
        };
        if events.send(event).is_err() {
            break;
        }
    }
}

fn export_document(path: PathBuf, format: RasterFormat) -> FileEvent {
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
