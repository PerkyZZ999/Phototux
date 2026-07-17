//! Multi-document session registry (DR-024 v2 — tabs).
//!
//! Active document lives in the host `SessionState` slot; inactive docs are parked
//! here with optional CPU layer pixels for GPU rehydrate on activate.

use serde::Serialize;

use crate::SessionState;
use crate::layer::LayerId;

/// Stable id for an open document tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct OpenDocumentId(pub u64);

/// Maximum simultaneous open documents (memory bound for v1 tabs).
pub const MAX_OPEN_DOCUMENTS: usize = 8;

/// Parked inactive document (pixels optional until first park-from-GPU).
#[derive(Debug)]
pub struct ParkedDocument {
    pub id: OpenDocumentId,
    pub title: String,
    pub session: SessionState,
    /// Layer RGBA8 buffers for GPU rehydrate (`recover_gpu_document`).
    pub layer_pixels: Vec<(LayerId, Vec<u8>)>,
    pub dirty: bool,
}

/// Registry of open documents; at most one is active in the host engine slot.
#[derive(Debug)]
pub struct DocumentRegistry {
    parked: Vec<ParkedDocument>,
    active_id: Option<OpenDocumentId>,
    next_id: u64,
}

impl Default for DocumentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentRegistry {
    pub fn new() -> Self {
        Self {
            parked: Vec::new(),
            active_id: None,
            next_id: 1,
        }
    }

    pub fn active_id(&self) -> Option<OpenDocumentId> {
        self.active_id
    }

    pub fn open_count(&self) -> usize {
        self.parked.len() + usize::from(self.active_id.is_some())
    }

    pub fn can_open_another(&self) -> bool {
        self.open_count() < MAX_OPEN_DOCUMENTS
    }

    /// Allocate a new id and mark it active (caller places session in host slot).
    pub fn begin_active(&mut self, title: impl Into<String>) -> Result<OpenDocumentId, String> {
        if !self.can_open_another() {
            return Err(format!(
                "document limit reached ({MAX_OPEN_DOCUMENTS}); close a tab first"
            ));
        }
        let id = OpenDocumentId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.active_id = Some(id);
        let _ = title;
        Ok(id)
    }

    /// Set / clear the active id without parking (close last doc).
    pub fn set_active_id(&mut self, id: Option<OpenDocumentId>) {
        self.active_id = id;
    }

    /// Park the current active session and return its id.
    pub fn park_active(
        &mut self,
        id: OpenDocumentId,
        title: String,
        session: SessionState,
        layer_pixels: Vec<(LayerId, Vec<u8>)>,
        dirty: bool,
    ) {
        self.parked.push(ParkedDocument {
            id,
            title,
            session,
            layer_pixels,
            dirty,
        });
        if self.active_id == Some(id) {
            self.active_id = None;
        }
    }

    /// Remove a parked document by id.
    pub fn take_parked(&mut self, id: OpenDocumentId) -> Option<ParkedDocument> {
        let idx = self.parked.iter().position(|d| d.id == id)?;
        Some(self.parked.remove(idx))
    }

    pub fn parked_ids(&self) -> impl Iterator<Item = OpenDocumentId> + '_ {
        self.parked.iter().map(|d| d.id)
    }

    pub fn rename(&mut self, id: OpenDocumentId, title: String) {
        if let Some(doc) = self.parked.iter_mut().find(|d| d.id == id) {
            doc.title = title;
        }
    }

    /// Tab strip model for QML (active first, then parked order).
    pub fn tabs_json(&self, active_title: &str, active_dirty: bool) -> String {
        #[derive(Serialize)]
        struct Tab {
            id: u64,
            title: String,
            dirty: bool,
            active: bool,
        }
        let mut tabs = Vec::new();
        if let Some(aid) = self.active_id {
            tabs.push(Tab {
                id: aid.0,
                title: active_title.to_owned(),
                dirty: active_dirty,
                active: true,
            });
        }
        for d in &self.parked {
            tabs.push(Tab {
                id: d.id.0,
                title: d.title.clone(),
                dirty: d.dirty,
                active: false,
            });
        }
        serde_json::to_string(&tabs).unwrap_or_else(|_| "[]".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn park_and_take_roundtrip() {
        let mut reg = DocumentRegistry::new();
        let id = reg.begin_active("A").expect("open");
        assert_eq!(reg.open_count(), 1);
        reg.park_active(id, "A".into(), SessionState::default(), Vec::new(), true);
        assert_eq!(reg.open_count(), 1);
        assert!(reg.active_id().is_none());
        let taken = reg.take_parked(id).expect("parked");
        assert!(taken.dirty);
        assert_eq!(reg.open_count(), 0);
    }

    #[test]
    fn enforces_max_open() {
        let mut reg = DocumentRegistry::new();
        for i in 0..MAX_OPEN_DOCUMENTS {
            let id = reg.begin_active(format!("D{i}")).expect("open");
            reg.park_active(
                id,
                format!("D{i}"),
                SessionState::default(),
                Vec::new(),
                false,
            );
        }
        assert!(reg.begin_active("overflow").is_err());
    }
}
