//! Crash-recovery journal for native documents (ADR-016).

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::ptx::{self, PtxDocument, PtxError};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryEntry {
    pub document_id: String,
    pub original_path: Option<String>,
    pub snapshot_path: String,
    pub saved_unix_ms: u128,
    pub dirty: bool,
}

#[derive(Debug, Error)]
pub enum RecoveryError {
    #[error(transparent)]
    Ptx(#[from] PtxError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Directory: `$XDG_STATE_HOME/phototux/recovery` or fallback under temp.
pub fn recovery_dir() -> PathBuf {
    if let Ok(state) = std::env::var("XDG_STATE_HOME") {
        return PathBuf::from(state).join("phototux").join("recovery");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".local")
            .join("state")
            .join("phototux")
            .join("recovery");
    }
    std::env::temp_dir().join("phototux-recovery")
}

/// Write an autosave snapshot and index entry.
///
/// # Errors
/// Returns [`RecoveryError`] on filesystem or encode failure.
pub fn write_autosave(
    doc: &PtxDocument,
    original_path: Option<&Path>,
) -> Result<RecoveryEntry, RecoveryError> {
    let dir = recovery_dir();
    fs::create_dir_all(&dir)?;
    let id = format!("{:032x}", doc.graph().document_id);
    let snapshot_path = dir.join(format!("{id}.ptx"));
    ptx::save_ptx_atomic(&snapshot_path, doc)?;
    let entry = RecoveryEntry {
        document_id: id.clone(),
        original_path: original_path.map(|p| p.display().to_string()),
        snapshot_path: snapshot_path.display().to_string(),
        saved_unix_ms: unix_ms(),
        dirty: true,
    };
    let index_path = dir.join(format!("{id}.json"));
    fs::write(index_path, serde_json::to_vec_pretty(&entry)?)?;
    Ok(entry)
}

/// List recoverable journal entries.
///
/// # Errors
/// Returns [`RecoveryError`] when the recovery directory cannot be read.
pub fn list_recoverable() -> Result<Vec<RecoveryEntry>, RecoveryError> {
    let dir = recovery_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        if let Ok(item) = serde_json::from_str::<RecoveryEntry>(&text) {
            out.push(item);
        }
    }
    out.sort_by_key(|b| std::cmp::Reverse(b.saved_unix_ms));
    Ok(out)
}

/// Load a recovery snapshot.
///
/// # Errors
/// Returns [`RecoveryError`] when the snapshot cannot be decoded.
pub fn load_recovery(entry: &RecoveryEntry) -> Result<PtxDocument, RecoveryError> {
    Ok(ptx::load_ptx(Path::new(&entry.snapshot_path))?)
}

/// Discard a recovery entry and its snapshot.
///
/// # Errors
/// Returns [`RecoveryError`] on filesystem failure.
pub fn discard_recovery(entry: &RecoveryEntry) -> Result<(), RecoveryError> {
    let _ = fs::remove_file(&entry.snapshot_path);
    let dir = recovery_dir();
    let index = dir.join(format!("{}.json", entry.document_id));
    let _ = fs::remove_file(index);
    Ok(())
}

fn unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ptx::PtxDocument;
    use phototux_engine::{DocumentGraph, DocumentSize};
    use std::collections::HashMap;
    use std::sync::Mutex;

    static LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn autosave_roundtrip() {
        let _guard = LOCK.lock().expect("lock");
        let dir =
            std::env::temp_dir().join(format!("phototux-recovery-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        // SAFETY: test-only env override scoped by mutex.
        unsafe {
            std::env::set_var("XDG_STATE_HOME", &dir);
        }
        let graph = DocumentGraph::new(DocumentSize::new(4, 4));
        let doc = PtxDocument::from_graph(graph, HashMap::new());
        let entry = write_autosave(&doc, None).expect("autosave");
        let loaded = load_recovery(&entry).expect("load");
        assert_eq!(loaded.graph().size.width, 4);
        discard_recovery(&entry).expect("discard");
        let _ = fs::remove_dir_all(&dir);
        unsafe {
            std::env::remove_var("XDG_STATE_HOME");
        }
    }
}
