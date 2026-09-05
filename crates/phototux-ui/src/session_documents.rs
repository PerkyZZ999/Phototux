//! Opening, parking and switching documents.
//!
//! A tab is a parked `SessionState` plus the host-side state that does not live
//! inside one. Parking has to take both: the engine's history travels with the
//! document, and the selection and transform undo stacks are `Vec`s on
//! `AppSession` that do not — leave them and undoing in the next tab pops this
//! tab's entry, restoring a mask that need not even be the same size.
//!
//! The dirty flag lives here for the same reason. It is written from one place
//! on purpose: the title bar and the tab strip both read it, and they disagreed
//! for a while because two callers set it independently.

use std::collections::HashMap;
use std::path::PathBuf;

use phototux_engine::{DocumentGraph, LayerId, OpenDocumentId, SessionState};
use phototux_io::{PtxDocument, Raster};

use crate::AppSession;

impl AppSession {
    // `pub(crate)`, as in the sibling `session_*` modules: the same internal
    // helpers, in a file of their own.

    pub(crate) fn apply_opened_ptx(&mut self, path: PathBuf, document: PtxDocument) {
        let title = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Document.ptx".into());
        if let Err(error) = self.prepare_new_document_tab(&title) {
            self.fail_io("Open", &error);
            return;
        }
        let parts = document.into_parts();
        let graph = parts.graph;
        self.clear_selection_stacks();
        self.clear_transform_stacks();
        // Smart-object sources belong to the document being opened, so the
        // ones from whatever was open before must not survive into it.
        self.smart_sources.clear();
        for (id, source) in parts.sources {
            self.smart_sources.insert(
                LayerId(id),
                phototux_engine::SmartSource {
                    width: source.width(),
                    height: source.height(),
                    pixels: source.pixels().to_vec(),
                },
            );
        }
        if let Err(error) = phototux_engine::DocumentError::check_size(graph.size) {
            self.fail_io("Open", &error.to_string());
            return;
        }
        match phototux_canvas::open_document(graph.size, &graph.layers_resolved()) {
            Ok(ms) => {
                self.finish_opened_ptx(path, title, graph, parts.rasters, parts.masks, ms);
            }
            Err(error) => self.fail_io("Open", &error),
        }
    }

    pub(crate) fn finish_opened_ptx(
        &mut self,
        path: PathBuf,
        title: String,
        graph: DocumentGraph,
        rasters: HashMap<u64, Raster>,
        masks: HashMap<u64, Raster>,
        ms: f32,
    ) {
        for (id, raster) in rasters {
            if let Err(error) = phototux_canvas::write_layer_rgba(LayerId(id), raster.pixels()) {
                self.fail_io("Open", &error);
                return;
            }
        }
        for (id, mask) in masks {
            let r8: Vec<u8> = mask.pixels().chunks_exact(4).map(|rgba| rgba[0]).collect();
            if let Err(error) = phototux_canvas::write_mask_r8(LayerId(id), &r8) {
                self.fail_io("Open", &error);
                return;
            }
        }
        self.engine.replace_graph(graph);
        self.engine.document_path = Some(path.display().to_string());
        self.engine.source_path = self.engine.document_path.clone();
        self.record_composite(ms);
        self.document_name = title;
        self.io_busy = false;
        self.compatibility_report.clear();
        self.publish_pixel_snapshot_from_gpu();
        self.sync_from_engine();
        self.emit_doc_fields();
        self.compatibility_report_changed();
        self.refresh_document_tabs_json();
        if let Ok(export_path) = std::env::var("PHOTOTUX_DESKTOP_EXPORT") {
            let url = if export_path.starts_with("file:") {
                export_path
            } else {
                format!("file://{export_path}")
            };
            self.export_raster_file(url);
        }
    }

    /// Write the modified flag and publish it to both views.
    ///
    /// `dirty` is published twice: as the `dirty` property the window title
    /// and the close prompt bind to, and inside `documentTabsJson`, which the
    /// tab strip is *handed* rather than binding. Three callers set the field
    /// and emitted only the property, so the strip kept whatever it had last
    /// been published with — a freshly opened `.ptx` wore an unsaved marker on
    /// a document `Ctrl+W` would then close without asking, because the flag
    /// the prompt reads was correctly `false`. Every write goes through here;
    /// `the_modified_flag_is_written_in_one_place` holds that.
    pub(crate) fn set_dirty(&mut self, dirty: bool) {
        if self.dirty == dirty {
            return;
        }
        self.dirty = dirty;
        self.dirty_changed();
        self.refresh_document_tabs_json();
    }

    pub(crate) fn mark_dirty(&mut self) {
        if !self.dirty {
            self.set_dirty(true);
        }
    }

    pub(crate) fn fail_io(&mut self, operation: &str, message: &str) {
        self.io_busy = false;
        // Multi-line integrity diagnostics (e.g. .ptx CRC) go straight into the dialog.
        self.io_error = if message.contains('\n') {
            format!("{operation} failed:\n{message}")
        } else {
            format!("{operation} failed: {message}")
        };
        self.io_busy_changed();
        self.io_error_changed();
        if message.contains(".ptx integrity") || message.contains("CRC32") {
            self.compatibility_report = message.to_owned();
            self.compatibility_report_changed();
        }
    }

    pub(crate) fn refresh_document_tabs_json(&mut self) {
        self.document_tabs_json = self.doc_registry.tabs_json(&self.document_name, self.dirty);
        self.document_tabs_json_changed();
    }

    /// Park the active document so another tab can become active.
    pub(crate) fn park_current_document(&mut self) -> Result<(), String> {
        let Some(id) = self.active_doc_id else {
            return Ok(());
        };
        if !self.engine.has_document {
            self.active_doc_id = None;
            self.doc_registry.set_active_id(None);
            return Ok(());
        }
        self.publish_pixel_snapshot_from_gpu();
        let layer_pixels: Vec<_> = phototux_canvas::read_all_layer_rgba()
            .unwrap_or_default()
            .into_iter()
            .map(|(layer_id, _w, _h, pixels)| (layer_id, pixels))
            .collect();
        let viewport_width = self.engine.viewport_w;
        let viewport_height = self.engine.viewport_h;
        let title = self.document_name.clone();
        let dirty = self.dirty;
        let session = std::mem::take(&mut self.engine);
        // Layer ids restart at 1 in every graph, so leaving these behind would
        // hand the next document this one's sources under the same ids.
        let smart_sources: Vec<_> = std::mem::take(&mut self.smart_sources)
            .into_iter()
            .collect();
        phototux_canvas::close_document();
        // Selection and transform undo live on the host, not in the parked
        // `SessionState`, so they do not travel with the document the way
        // everything else here does. The engine's history *is* per document —
        // leave these behind and undoing in the next tab pops that tab's
        // Selection entry while the host restores this tab's mask over it, a
        // mask that is not even the same size when the documents differ.
        //
        // Cleared rather than parked, on memory: a `SelectionSnapshot` is a
        // document-sized mask with a limit of 64 and a `TransformSnapshot` is
        // full RGBA for every layer. A Selection entry with no snapshot left
        // deselects, which is defined and safe.
        self.clear_selection_stacks();
        self.clear_transform_stacks();
        self.doc_registry
            .park_active(id, title, session, layer_pixels, smart_sources, dirty);
        self.active_doc_id = None;
        self.engine = SessionState::default();
        self.engine.set_viewport(viewport_width, viewport_height);
        self.set_dirty(false);
        Ok(())
    }

    pub(crate) fn prepare_new_document_tab(&mut self, title: &str) -> Result<(), String> {
        // Refuse before parking so a full registry cannot leave the session with
        // no active document after a failed begin_active.
        if !self.doc_registry.can_open_another() {
            let limit = phototux_engine::max_open_documents();
            return Err(format!(
                "document limit reached ({limit}); close a tab first"
            ));
        }
        if self.engine.has_document {
            self.park_current_document()?;
        }
        let id = self.doc_registry.begin_active(title)?;
        self.active_doc_id = Some(id);
        Ok(())
    }

    pub(crate) fn activate_document_id(&mut self, id: OpenDocumentId) -> Result<(), String> {
        if self.active_doc_id == Some(id) {
            return Ok(());
        }
        if self.engine.has_document {
            self.park_current_document()?;
        }
        let parked = self
            .doc_registry
            .take_parked(id)
            .ok_or_else(|| format!("unknown document tab {}", id.0))?;
        let viewport_width = self.engine.viewport_w;
        let viewport_height = self.engine.viewport_h;
        self.engine = parked.session;
        self.engine.set_viewport(viewport_width, viewport_height);
        self.smart_sources = parked.smart_sources.into_iter().collect();
        self.document_name = parked.title;
        self.set_dirty(parked.dirty);
        self.active_doc_id = Some(id);
        self.doc_registry.set_active_id(Some(id));
        if let Some(graph) = self.engine.graph.as_ref() {
            let size = graph.size;
            let layers = graph.layers().to_vec();
            match phototux_canvas::recover_gpu_document(size, &layers, &parked.layer_pixels) {
                Ok(ms) => {
                    self.record_composite(ms);
                    self.publish_pixel_snapshot_from_gpu();
                }
                Err(error) => self.report_gpu("activate document GPU", &error),
            }
        }
        self.sync_from_engine();
        self.emit_doc_fields();
        self.refresh_document_tabs_json();
        Ok(())
    }
}
