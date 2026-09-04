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

/// Effective open-document cap (override with `PHOTOTUX_MAX_OPEN_DOCUMENTS` for tests).
pub fn max_open_documents() -> usize {
    std::env::var("PHOTOTUX_MAX_OPEN_DOCUMENTS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|&n| (1..=MAX_OPEN_DOCUMENTS).contains(&n))
        .unwrap_or(MAX_OPEN_DOCUMENTS)
}

/// Parked inactive document (pixels optional until first park-from-GPU).
#[derive(Debug)]
pub struct ParkedDocument {
    pub id: OpenDocumentId,
    pub title: String,
    pub session: SessionState,
    /// Layer RGBA8 buffers for GPU rehydrate (`recover_gpu_document`).
    pub layer_pixels: Vec<(LayerId, Vec<u8>)>,
    /// Smart-object sources belonging to this document (DR-032).
    ///
    /// They park here for the same reason `layer_pixels` does — the host holds
    /// exactly one document's worth of them, and layer ids restart at 1 in
    /// every graph. Left in the host, a second document's smart object at
    /// layer 3 would read, show and save the *first* document's pixels.
    pub smart_sources: Vec<(LayerId, SmartSource)>,
    pub dirty: bool,
}

/// A smart object's pristine source pixels.
///
/// A named triple rather than a bare one: a buffer that does not match its own
/// dimensions is the kind of thing that reaches the rasterizer and produces a
/// shape with no pixels and no explanation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartSource {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes.
    pub pixels: Vec<u8>,
}

/// Registry of open documents; at most one is active in the host engine slot.
#[derive(Debug)]
pub struct DocumentRegistry {
    parked: Vec<ParkedDocument>,
    /// Strip order, oldest tab first, independent of which one is active.
    ///
    /// The strip used to be built as "active first, then whatever order the
    /// parked vector happened to be in" — and parking pushes to the end of
    /// that vector, so switching tabs shuffled twice: the tab you clicked
    /// jumped to position 0 and the one you left went to the back. Nothing
    /// else in the shell moves under the pointer like that, and a strip of
    /// three or more became unreadable.
    order: Vec<OpenDocumentId>,
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
            order: Vec::new(),
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
        self.open_count() < max_open_documents()
    }

    /// Allocate a new id and mark it active (caller places session in host slot).
    pub fn begin_active(&mut self, title: impl Into<String>) -> Result<OpenDocumentId, String> {
        if !self.can_open_another() {
            let limit = max_open_documents();
            return Err(format!(
                "document limit reached ({limit}); close a tab first"
            ));
        }
        let id = OpenDocumentId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.active_id = Some(id);
        self.order.push(id);
        let _ = title;
        Ok(id)
    }

    /// Drop a document from the session entirely.
    ///
    /// Closing is the one path that removes a tab: activating only moves a
    /// document between the parked vector and the host's active slot, and the
    /// strip order must survive that. A closed *active* document is never
    /// parked, so nothing else would take it out of the order.
    pub fn forget(&mut self, id: OpenDocumentId) {
        self.order.retain(|open| *open != id);
        self.parked.retain(|doc| doc.id != id);
        if self.active_id == Some(id) {
            self.active_id = None;
        }
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
        smart_sources: Vec<(LayerId, SmartSource)>,
        dirty: bool,
    ) {
        self.parked.push(ParkedDocument {
            id,
            title,
            session,
            layer_pixels,
            smart_sources,
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

    /// Tab strip model for QML, in the order the tabs were opened.
    ///
    /// The active document's title and dirty flag come from the caller because
    /// it is the host's live session, not a parked record. Its *position*
    /// comes from the same order as everyone else's.
    pub fn tabs_json(&self, active_title: &str, active_dirty: bool) -> String {
        #[derive(Serialize)]
        struct Tab {
            id: u64,
            title: String,
            dirty: bool,
            active: bool,
        }
        let tabs: Vec<Tab> = self
            .order
            .iter()
            .filter_map(|id| {
                if self.active_id == Some(*id) {
                    return Some(Tab {
                        id: id.0,
                        title: active_title.to_owned(),
                        dirty: active_dirty,
                        active: true,
                    });
                }
                self.parked.iter().find(|doc| doc.id == *id).map(|doc| Tab {
                    id: doc.id.0,
                    title: doc.title.clone(),
                    dirty: doc.dirty,
                    active: false,
                })
            })
            .collect();
        serde_json::to_string(&tabs).unwrap_or_else(|_| "[]".into())
    }

    /// The tab already holding `path`, if the session has one.
    ///
    /// Opening a file that is already open used to make a second tab with its
    /// own history, so a save from one silently lost the other's edits.
    /// Photoshop raises the existing tab, and so does this.
    ///
    /// The active document is not in `parked`, so the caller passes its path
    /// separately — the host owns that session, and the registry has no view
    /// of it.
    pub fn tab_for_path(&self, path: &str, active_path: Option<&str>) -> Option<OpenDocumentId> {
        if path.is_empty() {
            return None;
        }
        if let (Some(active), Some(id)) = (active_path, self.active_id)
            && active == path
        {
            return Some(id);
        }
        self.parked
            .iter()
            .find(|doc| doc.session.source_path.as_deref() == Some(path))
            .map(|doc| doc.id)
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
        reg.park_active(
            id,
            "A".into(),
            SessionState::default(),
            Vec::new(),
            Vec::new(),
            true,
        );
        assert_eq!(reg.open_count(), 1);
        assert!(reg.active_id().is_none());
        let taken = reg.take_parked(id).expect("parked");
        assert!(taken.dirty);
        assert_eq!(reg.open_count(), 0);
    }

    /// Layer ids restart at 1 in every graph, so a source left behind by one
    /// document is a source the next one will read under the same id.
    #[test]
    fn a_documents_smart_sources_park_and_come_back_with_it() {
        let mut reg = DocumentRegistry::new();
        let a = reg.begin_active("A").expect("open");
        let source = SmartSource {
            width: 1,
            height: 1,
            pixels: vec![7, 7, 7, 255],
        };
        reg.park_active(
            a,
            "A".into(),
            SessionState::default(),
            Vec::new(),
            vec![(LayerId(3), source.clone())],
            false,
        );
        let b = reg.begin_active("B").expect("open");
        reg.park_active(
            b,
            "B".into(),
            SessionState::default(),
            Vec::new(),
            Vec::new(),
            false,
        );
        assert!(
            reg.take_parked(b).expect("B").smart_sources.is_empty(),
            "B was handed A's sources"
        );
        assert_eq!(
            reg.take_parked(a).expect("A").smart_sources,
            vec![(LayerId(3), source)]
        );
    }

    #[test]
    fn enforces_max_open() {
        let mut reg = DocumentRegistry::new();
        let limit = max_open_documents();
        for i in 0..limit {
            let id = reg.begin_active(format!("D{i}")).expect("open");
            reg.park_active(
                id,
                format!("D{i}"),
                SessionState::default(),
                Vec::new(),
                Vec::new(),
                false,
            );
        }
        assert!(reg.begin_active("overflow").is_err());
    }
    /// The strip keeps the order tabs were opened in.
    ///
    /// It used to be "active first, then whatever order the parked vector
    /// happened to be in", and parking pushes to the end of that vector — so
    /// switching tabs shuffled twice: the one clicked jumped to position 0 and
    /// the one left behind went to the back.
    #[test]
    fn the_strip_keeps_the_order_tabs_were_opened_in() {
        let titles = |json: &str| {
            serde_json::from_str::<serde_json::Value>(json)
                .expect("tabs json")
                .as_array()
                .expect("array")
                .iter()
                .map(|tab| tab["title"].as_str().unwrap_or_default().to_owned())
                .collect::<Vec<_>>()
        };
        let park = |reg: &mut DocumentRegistry, id, title: &str| {
            reg.park_active(
                id,
                title.to_owned(),
                SessionState::default(),
                Vec::new(),
                Vec::new(),
                false,
            );
        };

        let mut reg = DocumentRegistry::new();
        let a = reg.begin_active("A").expect("open A");
        park(&mut reg, a, "A");
        let b = reg.begin_active("B").expect("open B");
        park(&mut reg, b, "B");
        let c = reg.begin_active("C").expect("open C");

        assert_eq!(titles(&reg.tabs_json("C", false)), ["A", "B", "C"]);

        // Switch to A: park C, take A. The strip must not move.
        park(&mut reg, c, "C");
        let taken = reg.take_parked(a).expect("A is parked");
        reg.set_active_id(Some(taken.id));
        assert_eq!(
            titles(&reg.tabs_json("A", false)),
            ["A", "B", "C"],
            "the strip reordered itself when the active tab changed"
        );

        // Closing takes a tab out of the order for good.
        reg.forget(a);
        park(&mut reg, b, "B");
        assert_eq!(titles(&reg.tabs_json("", false)), ["B", "C"]);
    }

    /// A file that is already open is found by path, not opened twice.
    #[test]
    fn a_file_already_open_is_found_by_path() {
        let mut reg = DocumentRegistry::new();
        let a = reg.begin_active("A").expect("open A");
        let session = SessionState {
            source_path: Some("/tmp/a.ptx".to_owned()),
            ..SessionState::default()
        };
        reg.park_active(a, "A".into(), session, Vec::new(), Vec::new(), false);

        let b = reg.begin_active("B").expect("open B");
        assert_eq!(reg.tab_for_path("/tmp/a.ptx", Some("/tmp/b.ptx")), Some(a));
        assert_eq!(reg.tab_for_path("/tmp/b.ptx", Some("/tmp/b.ptx")), Some(b));
        assert_eq!(reg.tab_for_path("/tmp/c.ptx", Some("/tmp/b.ptx")), None);
        assert_eq!(
            reg.tab_for_path("", Some("")),
            None,
            "an untitled document has no path to match"
        );
    }
}
