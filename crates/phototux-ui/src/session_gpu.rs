//! The host's GPU- and canvas-facing half.
//!
//! Everything that reaches `phototux_canvas` lives here: opening and
//! recompositing the document, committing an edit that writes pixels, the
//! navigator thumbnail, device-loss recovery, and the two host-side undo
//! stacks whose contents are GPU snapshots rather than graph commands.
//!
//! Those stacks are the reason this is one module and not two. A
//! `TransformSnapshot` carries the document's size, every layer's pixels *and*
//! the graph, which is what lets one history entry take back an edit whose two
//! halves live in different places — a profile conversion, a bake, a
//! rasterize. Reading and restoring them is GPU work; deciding when belongs to
//! the command spine, which stays in `lib.rs`.

use std::time::Instant;

use phototux_engine::{EngineCommand, Layer, NoticeLevel};

use crate::publish;
use crate::{AppSession, SelectionSnapshot, TransformSnapshot};

impl AppSession {
    // `pub(crate)`, like the projection module: same helpers, different file,
    // and privacy in Rust flows to a module's descendants rather than its
    // ancestors.

    pub(crate) fn send_paint(&mut self, command: EngineCommand) {
        if let Err(error) = self.worker.send(command) {
            self.fail_io("Paint", &error);
        }
    }

    /// Surface a device or I/O failure. Takes a `String` because that is what
    /// the canvas returns; a *command* failure has a typed error and belongs in
    /// [`Self::report_action_error`], which does not run the device-lost check
    /// on text that can never name a device.
    pub(crate) fn report_gpu(&mut self, operation: &str, error: &str) {
        // Logged before the branch, not after it. Every failure that arrives
        // while the device is already lost was routed to `enter_gpu_lost` and
        // returned, dropping the operation and the reason — so a failed
        // *recovery* was indistinguishable from the loss it was trying to fix,
        // and clicking Recover again produced the same toast and no new
        // information anywhere, log included.
        tracing::warn!(operation, %error, "GPU operation failed");
        let lower = error.to_ascii_lowercase();
        if lower.contains("device lost")
            || lower.contains("surface lost")
            || phototux_canvas::gpu_is_lost()
        {
            self.enter_gpu_lost();
            return;
        }
        self.notify(NoticeLevel::Error, format!("{operation} failed: {error}"));
    }

    pub(crate) fn enter_gpu_lost(&mut self) {
        if !self.gpu_lost {
            self.gpu_lost = true;
            self.gpu_lost_changed();
            self.refresh_inspector_badges();
        }
        self.engine
            .announce("Graphics device lost — document preserved");
        self.notify(
            NoticeLevel::Error,
            "Graphics device lost — the document is preserved. Use Recover to restore the canvas.",
        );
        self.publish_announcement();
        tracing::error!("GPU lost — document authority preserved");
    }

    pub(crate) fn clear_selection_stacks(&mut self) {
        self.selection_history.clear();
    }

    pub(crate) fn clear_transform_stacks(&mut self) {
        self.transform_history.clear();
    }

    pub(crate) fn push_transform_snapshot(&mut self) {
        if let Some(snapshot) = self.transform_snapshot_now() {
            self.transform_history.record(snapshot);
        }
    }

    pub(crate) fn restore_transform_snapshot(&mut self, snap: TransformSnapshot) {
        let layers = snap.graph.layers().to_vec();
        match phototux_canvas::restore_document_layers(snap.size, &snap.layers, &layers) {
            Ok(ms) => {
                self.engine.size = snap.size;
                self.engine.graph = Some(snap.graph);
                self.record_composite(ms);
                self.engine.transform_session = None;
                self.crop_preview_active = false;
            }
            Err(error) => self.report_gpu("transform restore", &error),
        }
    }

    /// The selection as it is right now, ready to be stepped away from.
    ///
    /// The GPU mask and `engine.selection` are two halves of one state, and
    /// reading them apart at each of the four call sites is how they came to
    /// be written out four times.
    pub(crate) fn selection_snapshot_now(&self) -> SelectionSnapshot {
        SelectionSnapshot {
            state: self.engine.selection.clone(),
            mask: phototux_canvas::selection_snapshot().unwrap_or_default(),
        }
    }

    /// The document's pixels and graph right now, or `None` when either is
    /// unavailable — an unopened document, or a GPU that cannot be read.
    pub(crate) fn transform_snapshot_now(&self) -> Option<TransformSnapshot> {
        let (size, layers) = phototux_canvas::snapshot_document_layers().ok()?;
        Some(TransformSnapshot {
            size,
            layers,
            graph: self.engine.graph.clone()?,
        })
    }

    pub(crate) fn push_selection_snapshot(&mut self) {
        let snapshot = self.selection_snapshot_now();
        self.selection_history.record(snapshot);
    }

    /// Run a GPU selection edit with its undo snapshot taken first.
    ///
    /// The snapshot reads the GPU mask and `engine.selection` as they are
    /// *now*, so it has to be taken before the edit overwrites either. Every
    /// call site used to restate that ordering, and one of them —
    /// `apply_mask_to_selection_host` — had it inverted, so its snapshot
    /// captured the post-edit state and Ctrl+Z was a no-op. Taking it here
    /// makes the order a property of the operation instead.
    ///
    /// Returns whether the edit succeeded, so the caller can decide whether to
    /// record a command.
    pub(crate) fn commit_selection_edit<F>(&mut self, label: &str, run: F) -> bool
    where
        F: FnOnce() -> Result<(), String>,
    {
        self.push_selection_snapshot();
        self.run_selection_edit(label, run)
    }

    /// Run a GPU selection edit whose snapshot the caller has already taken.
    ///
    /// One caller needs that: the marquee asks the engine *first*, because the
    /// engine refuses a selection covering no document pixel and writing the
    /// mask before asking would leave the two disagreeing (QA-005). But the
    /// engine command also moves `engine.selection`, so a snapshot taken after
    /// it pairs the new state with the old mask — and undo then restores a
    /// selection identical to the one it is undoing while reverting the
    /// coverage underneath it. The ants stay on screen over pixels that no
    /// longer match them.
    ///
    /// So the marquee snapshots above its `invoke_command` and comes here.
    pub(crate) fn run_selection_edit<F>(&mut self, label: &str, run: F) -> bool
    where
        F: FnOnce() -> Result<(), String>,
    {
        match run() {
            Ok(()) => true,
            Err(error) => {
                self.report_gpu(label, &error);
                false
            }
        }
    }

    /// Run a GPU layer edit: snapshot, hand the op current layer metadata, then
    /// publish the composite time and the repaint together.
    ///
    /// The tail is the part that kept getting dropped — T-025 was a composite
    /// that published its time without asking the canvas to repaint. `run`
    /// receives the layer metadata the GPU needs and returns its own result
    /// plus the measured composite milliseconds.
    pub(crate) fn commit_layer_edit<T, F>(&mut self, label: &str, run: F) -> Option<T>
    where
        F: FnOnce(&[Layer]) -> Result<(T, f32), String>,
    {
        self.push_transform_snapshot();
        let layers = self
            .engine
            .graph
            .as_ref()
            .map(|g| g.layers().to_vec())
            .unwrap_or_default();
        match run(&layers) {
            Ok((value, ms)) => {
                self.record_composite(ms);
                Some(value)
            }
            Err(error) => {
                self.report_gpu(label, &error);
                None
            }
        }
    }

    pub(crate) fn restore_selection_snapshot(&mut self, snap: SelectionSnapshot) {
        self.engine.selection = snap.state;
        if let Err(error) = phototux_canvas::selection_restore(&snap.mask) {
            self.report_gpu("selection restore", &error);
        }
    }

    /// Record a completed composite: publish its time and repaint the canvas.
    ///
    /// Both halves belong together. Repaint is driven by the generation
    /// counter, so a path that composites and reports the time without bumping
    /// it produces work the user never sees — which is exactly what happened to
    /// stroke undo once the canvas stopped repainting every frame.
    pub(crate) fn record_composite(&mut self, ms: f32) {
        self.engine.set_composite_ms(ms);
        self.bump_composite_generation();
    }

    /// A new composite has been published; ask the canvas to repaint.
    pub(crate) fn bump_composite_generation(&mut self) {
        self.composite_generation = self.composite_generation.wrapping_add(1);
        self.composite_generation_changed();
    }

    /// Rebuild the Navigator's picture if enough has changed and enough time
    /// has passed.
    ///
    /// Both conditions, not either: the generation alone would rebuild on every
    /// dab of a stroke, and the clock alone would rebuild a document nobody is
    /// editing.
    pub(crate) fn refresh_navigator_thumbnail(&mut self) {
        // The canvas is asked, not `self.has_document`: that field is synced
        // *after* the composite that a new document triggers, so on the first
        // pass it still says there is nothing to draw.
        if !phototux_canvas::has_document() {
            if !self.navigator_thumbnail.is_empty() {
                self.navigator_thumbnail.clear();
                self.navigator_thumbnail_generation = u64::MAX;
                self.navigator_thumbnail_changed();
            }
            return;
        }
        let generation = u64::from(self.composite_generation.max(0).unsigned_abs());
        if generation == self.navigator_thumbnail_generation {
            return;
        }
        if let Some(last) = self.navigator_thumbnail_at
            && last.elapsed().as_millis() < Self::NAVIGATOR_THUMBNAIL_INTERVAL_MS
        {
            return;
        }
        self.navigator_thumbnail_at = Some(Instant::now());
        self.navigator_thumbnail_generation = generation;
        let Some(url) = Self::render_navigator_thumbnail() else {
            return;
        };
        publish!(self, navigator_thumbnail, url, navigator_thumbnail_changed);
    }

    /// Read the composite back, shrink it, and encode it as a `data:` URL.
    ///
    /// `None` on any failure: a Navigator that keeps its previous picture for a
    /// moment is better than one that blanks because a readback lost a race.
    pub(crate) fn render_navigator_thumbnail() -> Option<String> {
        let (width, height, pixels) = phototux_canvas::read_composite_rgba().ok()?;
        let thumb = phototux_engine::downsample_rgba8(
            &pixels,
            width,
            height,
            Self::NAVIGATOR_THUMBNAIL_EDGE,
        )?;
        let raster =
            phototux_io::Raster::new(thumb.width, thumb.height, thumb.pixels.into_boxed_slice())
                .ok()?;
        let mut png = Vec::new();
        phototux_io::encode(&mut png, &raster, phototux_io::RasterFormat::Png).ok()?;
        Some(crate::data_url::png_data_url(&png))
    }

    pub(crate) fn apply_convert_pixels(&mut self, from: &str, to: &str) {
        match phototux_canvas::read_all_layer_rgba() {
            Ok(layers) => {
                for (id, _w, _h, mut pixels) in layers {
                    phototux_engine::convert_rgba8_profile(&mut pixels, from, to);
                    if let Err(error) = phototux_canvas::write_layer_rgba(id, &pixels) {
                        self.report_gpu("convert profile upload", &error);
                        return;
                    }
                }
            }
            Err(error) => {
                self.report_gpu("convert profile read", &error);
                return;
            }
        }
        if let Some(graph) = self.engine.graph.as_mut() {
            graph.color.mark_converted();
            graph.bump_generation();
        }
        // A message, not the summary: the status bar carries state that stays
        // true, and the next refresh would erase this. It is also the one
        // sentence that says the conversion was destructive, so losing it is
        // worse than losing most.
        self.notify(
            NoticeLevel::Warning,
            format!("Converted pixels to {to} (from {from}) — this rewrote layer data"),
        );
        self.status_text = self.engine.status_summary();
        self.status_text_changed();
    }

    pub(crate) fn recomposite(&mut self) {
        let Some(graph) = self.engine.graph.as_ref() else {
            return;
        };
        // Only the live filter-gallery preview needs a patched layer list; the
        // steady-state path composites the graph's own slice without cloning
        // every layer (names, effect vectors, masks) on each edit.
        let preview = self
            .engine
            .filter_preview
            .as_ref()
            .and_then(|preview| Some((preview.layer_id, preview.to_effect()?)));
        let result = match preview {
            Some((layer_id, effect)) => {
                let mut layers = graph.layers_resolved().into_owned();
                if let Some(layer) = layers.iter_mut().find(|l| l.id == layer_id) {
                    layer.effects.push(effect);
                }
                phototux_canvas::sync_and_composite(&layers)
            }
            None => phototux_canvas::sync_and_composite(&graph.layers_resolved()),
        };
        match result {
            Ok(ms) => {
                self.engine.set_composite_ms(ms);
                self.engine.clear_dirty_rect();
                self.bump_composite_generation();
            }
            Err(e) => {
                // The composite runs from a timer as well as from edits, so it
                // can fire in the window before the GPU side of the document
                // exists. That is a transient state and the next pass will
                // succeed — reporting it would put a failure in front of the
                // user for something that is not wrong. It was always being
                // posted; the status bar simply overwrote it before anyone saw.
                if phototux_canvas::has_document() {
                    self.report_gpu("composite", &e);
                }
            }
        }
    }

    pub(crate) fn open_gpu_document(&mut self) {
        let Some((size, layers)) = self
            .engine
            .graph
            .as_ref()
            .map(|graph| (graph.size, graph.layers_resolved().into_owned()))
        else {
            return;
        };
        self.clear_selection_stacks();
        self.clear_transform_stacks();
        self.engine.selection.clear();
        self.engine.transform_session = None;
        self.selection_preview_active = false;
        self.crop_preview_active = false;
        match phototux_canvas::open_document(size, &layers) {
            Ok(ms) => {
                self.record_composite(ms);
                self.publish_pixel_snapshot_from_gpu();
            }
            Err(e) => self.report_gpu("open_document GPU", &e),
        }
        self.refresh_document_tabs_json();
    }

    /// Open GPU resources for a *freshly created* document and paint its
    /// background white.
    ///
    /// Photoshop's File > New starts on an opaque white Background, and so
    /// does this — a stack that is transparent all the way down reads as
    /// "nothing here" against the checkerboard, and every stroke, fill and
    /// flatten would then composite against nothing. The white lives in the
    /// pixels rather than in the graph because `DocumentGraph` owns no
    /// buffers; it is written once, before the document is handed over, and
    /// so is not an undoable step.
    pub(crate) fn open_new_gpu_document(&mut self) {
        self.open_gpu_document();
        let Some((background, layers)) = self.engine.graph.as_ref().and_then(|graph| {
            graph
                .layers()
                .first()
                .map(|bottom| (bottom.id, graph.layers().to_vec()))
        }) else {
            return;
        };
        match phototux_canvas::fill_layer(background, [1.0, 1.0, 1.0, 1.0], &layers, false) {
            Ok(ms) => {
                self.record_composite(ms);
                self.publish_pixel_snapshot_from_gpu();
            }
            Err(error) => self.report_gpu("new document background", &error),
        }
    }

    pub(crate) fn publish_pixel_snapshot_from_gpu(&mut self) {
        if let Ok((_w, _h, rgba)) = phototux_canvas::read_composite_rgba() {
            let _ = self.engine.publish_pixel_snapshot_rgba(rgba);
        }
    }
}
