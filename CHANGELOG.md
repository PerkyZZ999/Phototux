# Changelog

All notable decision milestones and project state changes.

## [architecture-slices] — 2026-08-13

### Internal

- **Five panel-visibility properties were shadowing the registry that already answered the same question.** `panelNavigatorVisible` and its four siblings each occupied ten sites — field, initializer, sync line, two emit lines, a host-op branch, a `qproperty!`, a `#[qsignal]`, a dedicated slot, and its body — fifty sites for five booleans, all subsumed by `panelVisibilityJson` and `setPanelVisible(id, value)`, which are driven off `default_panels()`. Deleting them concentrates the logic where it already lived rather than moving it. Adding a sixth panel is now a registry entry instead of ten Rust edits.
- The Window menu's checked state and its toggles now derive the panel id from the action id, so Paths and Character get real checked state — they previously read `false` unconditionally because no case existed for them.
- **The mask Feather slider no longer pretends to work.** The value is stored, clamped, recorded in undo and bound to QML, and no renderer consumes it — dragging the slider changed nothing. Unlike density, contrast and shift it is a neighbourhood operation, and implementing it properly needs an R8 blur pipeline, because the composite samples masks from an R8 array while the separable blur is RGBA-only, and the CPU route would mean a GPU readback per mask repack. The control is disabled and labelled "not yet applied" until that lands: the stored value still round-trips in existing documents, and a slider that moves without changing a pixel is worse than one that says it is unavailable.
- **A layer with both a drop shadow and an outer glow now renders both.** They shared one `drop_shadow` slot — a glow being a shadow at zero offset — so the shadow won and the glow was silently dropped. The glow has its own slot, and the effect chain applies the shadow then the glow, which is the back-to-front order raster editors use. The test that pinned the old behaviour now asserts the new one; it was written to fail loudly at exactly this change rather than let rendering shift unnoticed.
- **The shader's mask block is now held to `LayerMask::coverage`.** Making that function the single definition of mask semantics fixed the bake path, but nothing proved the WGSL copy still agreed — which is precisely how the bake had drifted in the first place. A device-backed parity case composites a masked layer over a full coverage ramp across six control combinations and compares the resulting alpha against `coverage`. Removing the shader's contrast/shift refine fails it.
- **Two invisible-row bugs turned out to have different causes.** Command-palette rows showed their menu and shortcut but no label, and combo drop-downs dropped the row at `currentIndex` on first open. The palette was the Basic-style contrast fault once more — the delegate had no background of its own, so the style's light plate showed through beneath near-white text, while the grey menu and blue chord survived by being darker. The combo was contention over the `ComboBox`'s shared `delegateModel`: neither reading the delegate's own `model`, nor indexing the source array, nor re-evaluating on `visible` recovered that row, but giving the popup its own `ListView` over the plain array did. Combo lists are now dark, complete, and selectable (T-035, T-031).
- **`Main.qml` gave up its three largest dialogs.** At 6,594 lines it held 78% of all QML, with every dialog inline — the Preferences body alone was 463 lines. `FilterGalleryDialog`, `CommandPaletteDialog` and `PreferencesDialog` are now their own files, following the pattern `NewDocumentDialog` and `WelcomeDialog` already proved: everything a component needs from the shell is a declared property rather than a reach into `root`, which is what made the inline ones unmovable. `Main.qml` is down to 5,791 lines.
- **The Properties panel body followed the dialogs out.** At about 1,600 lines it was the largest single thing left in `Main.qml` — the whole per-layer editor sitting between the right dock's `Flickable` and the Navigator header, so that reading either neighbour meant scrolling past the adjustment stack. What kept it inline was not its size but that every id in `Main.qml` was in its scope: measuring the seam first turned up thirteen crossings, including two the obvious greps missed. `PropertiesPanel.qml` declares eleven inbound properties, one `embedIccRequested` signal, and two functions the shell calls to push host state into controls that hold their own selection. Naming those properties exactly as they are on the shell root means the body's `root.adjRange(…)` and friends resolve against the component instead — the extraction is a move, not a rewrite. The `Flickable` stays behind with the dock's `Layout.*` attachments, which are the dock's business. `Main.qml` is now 4,194 lines, under half what it was at the start of these slices.
- **`UndoPolicy::Mergeable` was declared on four commands and read by nothing.** The taxonomy in `command_meta` described how each command participates in undo, and no code anywhere consulted it — so dragging an opacity or adjustment slider wrote one history entry per step, and undo walked back through every one instead of returning to where the gesture began. `HistoryService::push_graph_mergeable` folds a run of the same edit on the same target, keeping the oldest `prev` and newest `next`. An unrelated command between two mergeable ones ends the run, so an interrupted gesture stays two entries. Three conformance tests now assert the declaration is true rather than merely present.
- **Computing a projected value and announcing it are now one step.** They were two hand-maintained lists, so a guard could be added to one half and not the other — and was: `sync_from_engine` rebuilt the accessibility tree only when it changed, then `emit_layer_fields` fired its notify on every layer edit regardless, which is the shape of T-009 flooding AT-SPI until the session died. A `publish!` macro assigns and notifies together and only when the value moved, so the guard cannot be half-applied. Applied to the two JSON projections, the six index-aligned layer strings and the history triple — the expensive ones, rebuilt and re-announced on every edit. Eleven of the fifty-three unconditional emissions in `emit_layer_fields` are now conditional.
- **Clipboard policy was six numbers that happened to agree.** The 64 MiB refusal was written as a function-local `const` in five `#[qslot]` bodies, beside a sixth copy of the same value in the engine's snapshot ceiling, and the check that a coverage buffer matches its document was open-coded next to each. `clipboard.rs` holds the decidable part — may we carry this, and is it the right shape — with the cap now *derived from* `phototux_engine::MAX_SNAPSHOT_BYTES` rather than restated, since a payload the clipboard accepts but the publisher refuses is the failure that drift produces. Seven headless tests, including that an absurd document size cannot wrap into acceptance.
- **The other half of that pairing is now done too.** `sync_from_engine` assigned about fifty fields and `emit_layer_fields` announced forty-two of them — two hand-maintained lists, hundreds of lines apart, with nothing tying an entry in one to an entry in the other. Thirty-three fields now publish where they are computed, leaving thirteen blind announcements for the properties that are computed getters rather than stored state and so have nothing to compare against. The effect is on a hot path: slider drags reach the host through the command spine, so dragging opacity fired all forty-two notifies per pointer sample, waking every binding on every one of them. The nine fields that were assigned in both arms of an `if let … else` now read their value as one tuple, which also puts the no-mask defaults — what the panel shows for an unmasked layer — in one place beside the real ones. A source-level conformance test fails if a field ever publishes conditionally *and* announces unconditionally, since that combination silently buys nothing while looking like a guard.
- Twelve slots each stored the engine's latest announcement and emitted its notify as a hand-written pair. `publish_announcement` pairs them, so an announcement cannot be stored without being published — the failure that leaves a screen reader describing the previous action.
- **Durable file replacement has one implementation.** `phototux_io` wrote the same sequence twice — unique sibling, write, `sync_all`, rename, remove the temporary on failure — character-for-character identical between raster export and `.ptx` apart from the error type and one message, backed by two independent sequence counters that only avoided colliding because the file names differed. `atomic::write_atomic` owns it, generic over the caller's error type.
- **Autosave's index file was not written atomically.** The `.ptx` snapshot went through the atomic path; the JSON index beside it, which the restore chooser actually reads, used a plain `fs::write`. A crash midway through left a truncated index that made a perfectly good snapshot unlistable. It now uses the same path as everything else.
- **Which document features become which render passes moved into the engine.** `LayerPackPlan::from_layer` was 90 lines of pure Rust in `phototux_gpu` deciding that a blur effect or a shadow style earns a pass, which wins when two overlap, and what is too small to bother with — document policy in the crate that knows how to *run* a pass, not which passes a layer deserves. `phototux_engine::LayerRenderPlan` owns it now, with eight headless tests; the GPU crate keeps the descriptor-to-pipeline half and re-exports the type so call sites are unchanged. `effect_pass.rs` previously had no test coverage at all.
- That move surfaced a policy defect in daylight: a layer carrying both a drop shadow and an outer glow silently loses the glow, because both are expressed through one `drop_shadow` slot. The behaviour is unchanged for now, but it is documented at the function, pinned by a test so a fix fails loudly there first, and ranked in the gap analysis — fixing it needs a second shadow pass in the renderer, so it is not local to the plan.
- **The GPU crate stated its own conventions five times.** Every full-screen pass needs the same vertex stage, the same render-target usage flags and — for the stampers — the same scissor planning, and each pass wrote them out again: five byte-identical vertex shaders, three copies of the render-target constructor differing only in the order of the same four flags, and two copies of the dab batch planner. `pass.rs` holds one of each. A wgpu descriptor change, or adding a timestamp query to every pass, is now one edit rather than five, and the off-canvas dab rule — where an empty scissor is invalid, not merely wasteful — has one statement instead of one per stamper.
- **`LazyDialog` never loaded anything, and the command palette never opened.** The type wraps a `Loader`, whose default property is `data` rather than `sourceComponent`, so a dialog written inside `LazyDialog { … }` became an ordinary child object: nothing was ever loaded and `item` was null forever. Dialogs whose visibility is bound to host state still worked — they existed as eager children — which hid the fault and also meant the deferred construction this type exists for never actually happened. The palette, which reaches its API through `ensure()`, received null and failed silently. `LazyDialog` now declares `default property Component dialog` and binds `sourceComponent` to it; call sites are unchanged (T-034).
- **`statusText` stopped doubling as an RPC channel to QML.** Ten `"host:…"` magic strings were written into the status-bar property and prefix-matched in QML — a contract duplicated across two languages that nothing checked, where a typo was a silently dead menu item, and where `setStatus` let any QML caller forge one. `phototux_engine::HostRequest` names the vocabulary once with round-trip and distinctness tests, and a dedicated `pendingHostRequest` property carries it, so the status bar is only status again.
- **Shape rasterization moved into the engine.** `shape_pixels` sat on the qtbridge `AppSession`, calling only engine functions and touching neither Qt nor the GPU — engine work that happened to live in the toolkit crate, where DR-022 says core logic must not be, because a `#[qobject]` cannot be constructed in a unit test. `phototux_engine::rasterize_shape_content` replaces it, with `rgba_f32_to_u8` replacing six near-identical inlined clamps in the same function. Six headless tests cover fill, the empty case, the gradient path, boolean subtract, gamut clamping and dimension overflow.
- **PSD flattened export composited every layer as Normal.** `phototux_io` kept its own compositor that bound the blend mode as `_blend` and ignored it, so exporting a document with Multiply, Screen or Overlay silently lost them. It now delegates to `phototux_engine::composite_rgba8` — the crate already depended on the engine, so the third implementation was never needed.
- **Colour dodge and colour burn rendered wrong on the GPU.** WGSL's `select(f, t, cond)` returns `t` when the condition holds; the shader had both arms reversed, so dodge returned white for every source below 1.0 and burn returned black for every source above 0.0. The CPU reference did not catch it because it implemented neither mode — seven of seventeen fell through to a `_ => s` arm — and the parity fixture compared three modes.
- The CPU reference now covers every mode the shader genuinely computes, `PARITY_BLEND_MODES` lists all twelve rather than three, and hue/saturation/colour/luminosity are documented as a shared fallback rather than silently missing.
- **Layer styles were not undoable, and undoing a vector mask consumed a step without removing it.** Recording an edit in history was a convention each command restated rather than a step it could not skip, so five commands bumped the document generation and recorded nothing — while `command_meta` declared all five `UndoPolicy::Transaction`. Drop shadow, stroke, outer glow and colour overlay now share one `add_layer_style` and record a `SetStyles` edit; `mask.create-vector` records a real `SetVectorMask` instead of a `Transform` entry, which `undo_next` hands to the host as a no-op. `record_graph_edit` owns the tail — bump, record, produce effects — so the three cannot come apart again.
- The conformance suite now checks behaviour rather than table shape: each style command must round-trip through undo, creating a vector mask must round-trip, and — generically — a command that moves the document generation must leave an undo entry behind. All three fail against the previous code, verified by reverting.
- **Apply Layer Mask ignored the mask's contrast and shift.** Mask semantics had no owning definition: the composite shader applied invert → contrast/shift → density, while the bake path in the UI crate open-coded invert → density and dropped the middle step, so baking a mask with either refine set produced pixels that did not match the canvas. `LayerMask::coverage` is now the single definition, next to the type that declares the fields, with the ordering documented and six headless tests over it — including that refine pivots on 0.5 and that coverage stays in range across the extremes of all three controls.
- `LayerMask::feather` is recorded as a gap rather than quietly implemented or deleted: it is stored, clamped, undone and bound to QML while affecting no pixel anywhere, and unlike the other controls it is a neighbourhood operation that `coverage` cannot express per sample.
- **Undo after Mask → Selection was a no-op.** Thirteen slots hand-rolled the same GPU-edit protocol — snapshot the pre-state, copy layer metadata for the GPU, run the op, publish the composite time *and* the repaint, record the command, map the error — in an order that was load-bearing and written down nowhere. Twelve snapshotted before mutating; `apply_mask_to_selection_host` snapshotted after, so it stored the state the undo was supposed to reverse. `commit_selection_edit` and `commit_layer_edit` own the protocol, and `push_selection_snapshot` / `push_transform_snapshot` are now reachable only from inside them, so the ordering cannot be restated wrongly at a call site (T-033).
- **Hiding the raised tab of a dock group hid its siblings too.** The dock topology records which tab the user last raised, but visibility lives in `WorkspaceState`, so the stored selection was used even after it was hidden and no sibling qualified to draw. `effective_active_tab` falls through to the first visible tab, and returns `None` only when the group has genuinely nothing left to show (T-032).
- Seven workspace slots repeated the same failure epilogue verbatim (format a message, emit `statusText`, return early, else persist). One `commit_workspace_op` owns it, so "a rejected layout change must not be persisted" is a property of the operation rather than a convention each slot restates.

## [reentrancy-and-contrast] — 2026-08-13

### Fixes

- **Tearing a panel off aborted the process, and so did opening the Filter Gallery.** Both were the same bug reached two ways. A host slot holds the session mutably borrowed for its whole body, including the change notifications it emits, so QML bindings that react to those notifications run *inside* the slot — and any call back into the session from there fails the borrow check and aborts. Tear-off built the floating window synchronously and its geometry write-back re-entered immediately; the Filter Gallery's modal popup moved focus while `openFilterGallery` was still on the stack, and the focus handler pushed the shortcut-yield flag back to the host.
- Every reactive write-back now defers through `root.afterHostSlot`. `refreshShortcutYield` defers at the function rather than at each call site, which makes all six of its reactive callers safe at once. The same treatment covers the viewport size write-back, ending a stroke when the tool changes under it, the status-marker drain behind File ▸ New/Open/Save As, and the preferences and gallery close paths — each of which could abort on the right timing. Deferral also coalesces the write-back storms during a window drag or resize into one call per event-loop turn.
- The tear-off crash reproduced identically on the pre-tabbed-dock build, so it predates that work rather than being a regression from it. The rule is now a normative contract in handbook [32 — Host Slot Re-entrancy](internal_docs/32-Developer-Guide.md#host-slot-re-entrancy); recorded as T-027 and T-028.

### Shell

- **Preferences was unreadable.** No Qt Quick Controls style is configured, so the shell runs the Basic style, which hardcodes a light palette and — unlike Fusion — ignores `palette` overrides entirely. Controls that draw on their *own* background looked fine, but every `CheckBox` label and every inline dialog title landed as near-black text on dark chrome, around 1.3:1 against the AA floor handbook 28 requires. `ThemedCheckBox`, `ThemedComboBox`, `ThemedSpinBox` and `ThemedDialogHeader` draw from `Theme.qml` tokens; all seventeen checkboxes, five combo boxes, eight spin boxes and seven dialog titles use them (T-029).
- The Filter Gallery drew its content over its own title — the `contentItem` was anchored to `parent`, which spans the whole popup rather than the region a `Dialog` reserves between header and footer (T-030). Preferences grows into the window now that its header and footer occupy real height.
- Combo drop-down lists stay light-on-dark for now. Theming the popup left the row at `currentIndex` blank in every combo — the list reserved its slot but painted neither label nor highlight, through both delegate-model access and direct array indexing. A list that hides one of its options is worse than one that clashes, so the style's own popup was kept and the gap ranked (T-031).

## [familiar-shell-2] — 2026-08-12

### Shell

- **The right dock groups panels into tabbed sets.** Essentials is now Properties, then Navigator/Swatches, then Layers/History — three groups instead of five stacked panels. This is the layout raster-editor users expect, and it is also the fix for panels starving each other: Layers previously rendered as a bare header at 1440×900 and now shows its content.
- `DockTopology` version 2 derives groups from the existing ordered stack rather than storing nested lists, so ordering, move, tear-off and auto-hide keep operating on one flat structure. Every stack mutation normalizes the grouping, because tearing off the first tab of a group or reordering a grouped panel to the head are both reachable through ordinary use. A v1 topology migrates to the current grouping — it is presentation state, and preserving the old stack would leave existing users on the layout this replaces.
- A group of one presents exactly as an ungrouped panel did; tab chrome only appears where it carries information. Selection is shown by weight, colour **and** an accent underline, and reaches assistive technology as a tab role with selected state — handbook 28 forbids colour-only state. The header controls belong to the visible panel, not the group.

## [repaint-regression-fix] — 2026-08-12

### Fixes

- **Undo and redo of a brush stroke left the canvas unchanged.** Making repaint demand-driven tied it to a composite-generation counter, but only two of seventeen composite paths raised it — the rest, including stroke undo and redo, composited and published a time without ever asking the canvas to repaint. History moved, the document changed, and the screen did not. Every path now records through one call that does both, so the two halves cannot be separated again.

Found by driving the running app: the Edit menu showed Undo greyed and Redo enabled, proving the command had executed while the pixels were untouched.

## [familiar-shell-1] — 2026-08-12

Shell alignment with the conventions raster-editor users already have, so the
app stops asking them to relearn placement. Familiar *conventions* only —
handbook 28 rules out proprietary branding and menu naming, and the KDE-native
visual tokens stay.

### Shell

- **A tool options bar sits under the main toolbar** ([`ToolOptionsBar.qml`](qml/ToolOptionsBar.qml)), carrying the active tool's constantly-adjusted parameters: brush size/hardness/texture, selection combine mode, fill colour, font and size, crop extent, rotation and constrain, zoom with Fit and 100%. Handbook 06 has specified this as a SHOULD since it was written and the gap analysis tracked it as open; tool options previously lived only in the right dock, two surfaces away from the gesture that needs them.
- It is disclosure **level 1** and never collapses. Content is chosen by presence — an absent control means the parameter does not apply to the active tool. Options scroll rather than disappear on a narrow window, and Apply/Cancel for an uncommitted crop or transform sit outside the scrolling region so they cannot be scrolled out of reach.
- Both surfaces edit through the same session slots, so the options bar and the inspector cannot drift.

### Fixes

- **Three selection-mode icons were never embedded.** `selection-plus`, `minus-circle` and `intersect` are referenced by the Properties selection buttons but were absent from the AOT resource list, so those buttons have been rendering blank.
- **Shape and Path Edit were unreachable.** `set_active_tool` validates against a list of known ids that omitted both, so clicking either on the shelf silently activated the Brush — the shelf highlighted one tool while another was active. 2 of 17 tools.

### Input

- **Tools are registered actions with the conventional letter keys**: V move, M / Shift+M marquee, L / Shift+L lasso, C crop, I eyedropper, B brush, E eraser, G / Shift+G gradient and bucket, T type, P path, U shape, H hand, Z zoom, Ctrl+T free transform. Previously tools were shelf-only — no keys, no action search, nothing rebindable, which handbook 06 requires as equivalent routes.
- **The shelf follows the established order**: pointer, selection, crop and transform, sampling, painting, text, vector, navigation. It previously opened with Brush and Eraser and listed the `paint` family in two pieces with selection and transform between them — the shelf separates on group change, so that drew a divider through the middle of the painting family. Groups are now contiguous, and a test enforces it.
- `Fill` is titled Paint Bucket and the type tool is titled Text, the terms the wider field uses.
- Tools sit in a search-only `tools` group rather than a menu-bar entry, matching editors of this kind, and route through the same activation the shelf uses so an in-progress transform or crop cancels identically however the tool was switched.

## [paint-fluidity-3] — 2026-08-12

Canvas fluidity pass, part three: stop paying whole-canvas cost for small dabs.

### Rendering

- **A dab batch is one scissored render pass, not a pass per dab.** Each dab began its own `begin_render_pass` over the entire layer and drew a full-screen triangle whose off-dab fragments were discarded — on a 4K layer a 20 px dab rasterized 8.3 M fragments to keep about 1 250, and consecutive dabs paid a pipeline drain plus a full attachment load and store between them. The batch now records one pass and scissors each dab to the region it can touch. Draws within a pass blend in submission order, so the result is unchanged. Dabs entirely off-canvas are skipped, since an empty scissor rect is invalid.
- Pipeline switches are now only rebound when the paint/erase mode actually changes within a batch.
- Mask painting uses the same batched, scissored path, sharing the bounding helper rather than repeating it.
- **The composite re-blends only the region painted since the last one.** It was unconditionally whole-canvas, all layers, with `LoadOp::Clear` and no scissor, and the layer-array slice copy moved the entire layer — 33 MB at 4K — per composite. The engine now accumulates the painted region and bounds both the blend and the copy to it. Gated on a complete previous result, no structural repack, every invalidation carrying a region, and byte-identical layer parameters; failing any of those falls back to the full path, so the worst case is the cost it had before rather than a stale canvas. Effect layers still copy whole, since a blur changes output beyond the pixels that fed it.
- **The canvas repaints when content changes, not every vsync.** `FrameAnimation` wrote `phase` each frame and `setPhase` called `update()`, forcing a full RHI sync and render pass forever — but the shader reads `phase` only after an early-out on selection ants, which are almost always off. A new `contentTick` property, driven by a `compositeGeneration` counter the host bumps on every published composite, now schedules the repaint. `synchronize()` is also where the pending export is pulled from Rust, so the tick had to be the invalidation signal or the canvas would have frozen instead of idling.
- **Stroke undo is bounded by VRAM, not by entry count.** Each backup is a full copy of the painted surface, so the cap of 64 entries meant a few megabytes on a small document and over 2 GB at 4K — allocator pressure and eviction hitches long before the count was reached. A 512 MB budget now sets the depth, and the most recent stroke is always kept even if it alone exceeds it.
- **Mask strokes no longer end with a blocking full-mask readback.** `end_stroke` downloaded the whole mask — 8.3 MB at 4K — and blocked on GPU completion, holding both the shared queue guard and the document lock, at every mask pen-up. It was redundant: every consumer of the CPU mirror already refreshed it before reading. `read_mask_r8` now performs the sync itself, so the cost lands on the slow path that wants the bytes and a caller cannot forget it.
- Marching-ants `dashOffset` bindings no longer re-evaluate every frame when no selection exists; bindings on children of an invisible item still run, so the parent's visibility gate was not enough.
- `last_composite_bounded()` exposes which path ran, so a test comparing bounded against full cannot pass by never taking the bounded path — `write_layer_rgba` marks the array structurally dirty and had been doing exactly that.

### Tests

- Nothing in the suite read pixels back after stamping, so the previous 42 GPU tests could not have observed a scissor that clipped strokes. Four device-backed tests now cover full-radius coverage, every dab of a batch landing, edge clamping, and the off-canvas case; `a_stamped_dab_covers_its_whole_radius` is verified to fail against a deliberately tight bound.

## [paint-fluidity-2] — 2026-08-12

Canvas fluidity pass, part two: make the brush respond to pressure. This half is
the engine; it lands first so that wiring a stylus cannot regress anything.

### Brush engine

- **Spacing follows the diameter being stamped.** `spacing()` used the nominal brush size while the radius came from pressure, so with `size_pressure` on (the default) a light touch drew dabs a fraction of the nominal width and spaced them at the full width — at pressure 0.2 that is a 1.25-diameter gap between dabs, a dotted line rather than a thin one. Spacing is now `spacing_at(pressure)`, re-evaluated as the segment is walked, which is the handbook's integrated local spacing. A brush with `size_pressure` off is unaffected.
- **Pressure ramps across a segment.** `move_to` applied the arriving sample's pressure to every dab between it and the previous sample, so a smooth press produced one visible step per input event and the step count tracked the input rate. Pressure now interpolates between the two samples, and spacing tightens with it.
- Constant-pressure strokes — every mouse stroke until stylus input is wired — place dabs at exactly the same positions as before, pinned by `constant_pressure_spacing_is_unchanged`.

### Input

- **Stylus pressure reaches the brush.** Both stroke calls read `mouse.pressure` off `QQuickMouseEvent`, which has no such property — the check `typeof mouse.pressure === "number"` was always false, so every dab of every stroke was stamped at 1.0 and the engine's pressure dynamics never received a signal. Pressure now comes from a `PointHandler` tracking the same point, which exposes the device's real value. It takes only a passive grab, so the `MouseArea` keeps owning every gesture. Devices without pressure report 0 while held and fall back to full pressure, so a mouse behaves exactly as before.
- **Not verified on hardware.** No stylus is attached to the development machine and synthetic pointer input cannot be delivered on this session, so the handler is confirmed to instantiate without QML errors and to be a safe superset of the previous behaviour by construction — but nobody has yet drawn a pressure-varying stroke. Worth ten seconds with a tablet before relying on it.

## [paint-fluidity-1] — 2026-08-12

Canvas fluidity pass, part one: remove work from the stroke path that the display
never shows and the user always waits for.

### Rendering

- **The interactive composite no longer blocks on GPU completion.** `repack_array_if_needed` ended its submission with `PollType::wait_indefinitely`, and every mid-stroke composite reaches it: `stamp_dabs` marks the painted layer, which puts a dirty slice in the repack, and the early-out requires that list to be empty. Measured on Arc B580 / Mesa 26.1 at 10 layers × 4K: **1.94 ms → 0.57 ms** of host time per composite, held under the shared queue lock the Qt render thread needs.
- DR-030 already forbade this; the wait survived because the guard test could not see it. `interactive_composite_does_not_wait_for_the_gpu` never marked a layer painted, so it only ever measured the clean path, which skips repacking altogether. The test now paints, runs at the DR-017 gate size, and compares the fastest call rather than the mean — sustained submission hits backpressure, where wall time converges to GPU throughput whether or not the host waits, and the mean stops discriminating. Verified to fail at 1.94 ms against the old behaviour and pass at 0.57 ms with the fix.
- **Mid-stroke composites are paced against time, not dab count.** The trigger was `dabs_since_composite >= 4`; dab rate is pointer speed over spacing, so it tracked how fast you moved rather than the refresh rate. One constant failed in both directions — a size-4 brush at 1000 doc px/s asked for ~250 composites/s against 60 displayed frames, while a size-200 brush at 100 px/s went 2 s between composites with the stroke stamped but invisible. Now paced at 8 ms, which admits 120 Hz without firing twice per frame at 60 Hz. The worker waits on that gap when dabs are pending, so a stroke that pauses mid-air still lands its tail.
- **Stroke end composited twice.** The tail flush asked for a composite and `end_stroke` composited again immediately after; the flush no longer does.

### Shell

- **The action table is built once instead of per lookup.** `action_by_id` called `default_actions()`, which allocates ~800 strings across 101 descriptors, then linearly scanned the result — so a single "can I undo?" question constructed the whole table. Around 100 menu items bind to enablement and all of them re-evaluate together whenever an enablement input changes, which includes every pen-up. The table is now a `LazyLock` with an id index, and `action_by_id` borrows from it. `default_shortcut_map` / `default_action_shortcuts`, on the shortcut path, borrow it too.
- **Per-frame and per-composite property signals are guarded.** `fps` signalled on every frame from a smoothed float whose rounded readout rarely moved; `compositeMs` signalled on every paced composite throughout a stroke at a precision finer than its two-decimal readout. Both relaid out the status bar — and `compositeMs` the collapsed Diagnostics summary — for values that had not visibly changed. Both now emit only when the displayed value would differ. `can_undo` / `can_redo` / `dirty` at pen-up are guarded the same way, since ~100 enablement bindings re-evaluate together on them.

### Fixes

- **The stroke journal no longer writes to disk on the UI thread.** `StrokeJournaled` arrives in the frame tick at pen-up, and its handler did `create_dir_all` plus `fs::write` of a pretty-printed record of every sample and dab — a few hundred KB for a long stroke — synchronously, in the frame the user is watching. It now goes to the existing `FileWorker`, and the record serializes compactly since only recovery reads it.
- **The History panel omitted brush strokes.** A stroke pushes a history entry, but `raster.paint-stroke` reports no layer sync, so `emit_layer_fields` — the only emitter of `historyLabels` / `historyEntryIds` / `historyKinds` — never ran for it, and `emit_poll_dirty_changes` did not emit them either. The panel silently stayed stale until some unrelated edit refreshed it. Now emitted whenever the entry list changes.

---

## [prefs-missing-key-defaults] — 2026-08-12

### Fixes

- **A preferences file missing the legacy panel keys started with every right-dock panel hidden.** `Preferences` carries a container-level `#[serde(default)]`, which resolves missing keys against the product default — but the five legacy `panel_*` bools each also carried a *field-level* `#[serde(default)]`, and the field-level attribute wins. Those resolved to `false`, `migrate_panel_visibility()` then filled `panel_visibility` from them, and the shell came up with a blank dock. The redundant attributes are removed, so an absent key now means "not recorded" rather than "the user hid it".
- Same removal on `high_contrast`, `reduced_motion`, and `safe_start_next`, where the two defaults already agreed. This is a no-op by itself, and it is what makes the remaining attributes legible: every field-level `#[serde(default)]` left on the struct now marks a field whose emptiness is a **load-bearing sentinel** — `panel_visibility` (triggers migration), `dock_topology_json`, `brush_presets_json`, `user_workspace_presets_json`, `last_saved_workspace_json` (trigger backfill or normalization), and `disclosure_open` (sparse by contract). Each says so in its doc comment.

Not reachable from any file the app itself writes, since those always serialize `panel_visibility`. Found by hand-authoring a minimal file for a screenshot harness.

### Tests

- `sparse_file_keeps_product_panel_defaults` deserializes `{"schema_version": 7}` and asserts the essentials panels survive migration; verified to fail against the old attributes.
- `legacy_file_still_migrates_its_own_choices` pins the other half — an absent `panel_visibility` must keep resolving to empty, or a genuine schema-2 file would skip migration and lose the user's recorded choices.
- `sparse_file_preserves_backfill_sentinels` covers the remaining sentinel fields.

---

## [inspector-badges-and-visual-pass] — 2026-08-12

Closes the loose ends left by `[inspector-disclosure]`, and gives the density work its first
verification on a real display.

### UX

- **Header badges are derived from host state.** `inspector_badges()` computes a group id → `{text, severity}` map from an inspector state snapshot, published as `inspectorBadgesJson`; `DisclosureGroup` resolves its own badge by id. Handbook 28 requires an invalid value to reach a collapsed header without the body existing, so the rule cannot live in the widgets that own the value. Shipped rules: an adjustment parameter outside the editor's range, an active selection whose outline misses the canvas, a text layer whose font family is not installed, and GPU loss.
- **Adjustment editor ranges are a registered contract.** `adjustment_editor_ranges()` is read by both the sliders and the out-of-range rule, so the two cannot disagree about what is showable. Editor bounds stay narrower than the engine's accepted bounds: a document may legally carry a gamma the slider cannot reach, and that now raises a badge instead of silently pinning the control.
- **All ten groups carry a collapsed summary**, up from three — the parameter worth confirming before expanding. Where a badge and a summary compete for header width the summary elides first, and the badge elides against a bounded share rather than widening the row.
- **Expand-all / collapse-all are wired.** Registered as `action.view.expand-all-groups` / `action.view.collapse-all-groups` for menu and action-search discovery, and surfaced as a state-reflecting control on the Properties header — offering collapse while any group is expanded, expand otherwise. The `AppSession` slots that had no caller are now the header's handler.

### Fixes

- **Panel-header drag areas swallowed button clicks at `comfortable` density.** All five headers reserved a literal 110 px for chrome, but four buttons already span 112 px at that density. Each header now measures its own `PanelHeaderControls`. The Properties reserve for the panels stacked below it moves from a literal to `Theme.dockStackReserve` for the same reason.
- **Collapsed groups pointed the caret up**, contradicting the Right-expands / Left-collapses grammar on the same header. Collapsed now points right.
- **`inspector.brush` was laid out ninth though the registry declares it second.** Handbook 28 forbids reordering registered groups; `inspector_lays_groups_out_in_registry_order` now asserts the layout against the registry, since a declarative layout offers nothing else to assert against.

### Verification

- First visual pass on a real Wayland session: dense, `comfortable`, and `QT_SCALE_FACTOR=2`, plus a forced-visibility QML override that puts all ten group headers on screen at once. Density confirmed to drive layout, not only type.
- Known limits recorded rather than papered over: the right dock still gives Layers and History header-only height in a ~900 px window and at 200 % scale, and the missing-font badge stays silent until fontconfig discovery has run. Both are ranked in the gap-analysis backlog.
- Pointer clicks could not be injected on this session (KWin ignores `ydotool` uinput events), so the header control's two states were verified by seeding `disclosure_open` both ways rather than by pressing the button.

---

## [composite-no-host-wait] — 2026-08-12

### Decisions

- **DR-030** — zero-copy present is ordered by a shared-queue image memory barrier, not a host wait. Qt adopts wgpu's device *and* queue, so a barrier before submit applies to Qt's later frame in submission order.
- **DR-031** — GPU budgets are measured with timestamp queries, collected asynchronously; benchmarks use a separate entry point that waits.

### Rendering

- `composite()` no longer blocks on `device.poll(wait_indefinitely())`. Measured: host 0.05 ms vs 0.20–0.30 ms GPU for the same pass; `recomposite()` runs on the UI thread, where handbook 28 forbids waiting for GPU completion.
- Stroke path no longer stalls on every fourth dab; wgpu's own tracking supplies the stamp→composite barrier.
- Readback, sampling, and export paths still wait, as they must — they map GPU memory to host memory.
- `SharedQueueGuard` retained: `vkQueueSubmit` needs external synchronization regardless of GPU ordering.

### Measurement

- `compositeMs` is now GPU time from `TIMESTAMP_QUERY` (one composite behind), replacing host wall time around the removed stall. 10×4K gate: **1.79 ms GPU**.
- Devices without timestamp queries report "no GPU timing" rather than 0 ms.
- Stroke latency relabeled input→submit; end-to-end input→present stays Provisional pending present-side instrumentation.
- New `interactive_composite_does_not_wait_for_the_gpu` fails if a blocking wait returns to the interactive path.

---

## [inspector-disclosure] — 2026-08-12

### UX

- Properties panel regrouped from a flat stack of conditionally visible sections into registered disclosure groups (`DisclosureGroup.qml`), implementing the four-level model in handbook [01](internal_docs/01-Information-Architecture.md#disclosure-group-registry) / [28](internal_docs/28-UX-Guidelines.md).
- Group registry (`default_disclosure_groups()`) owns stable ids, concept titles, levels, and defaults; presence (context) and disclosure (user) are separate axes.
- Expansion persists as presentation state — preferences schema **7**, sparse `disclosure_open` map; cleared by safe start. The one-off `propertiesAdvancedOpen` toggle it replaces is removed.
- `Theme.densityScale` now drives spacing, control heights, hit targets, and chrome extents; previously "comfortable" scaled type only. Tool strip and dock widths read tokens instead of literals.

### Performance

- Fontconfig enumeration deferred out of host construction: `AppSession` build ~91 ms → ~3 ms.
- Dialogs, command palette, and collapsed inspector groups build on first use (`LazyDialog.qml`, lazy group bodies): first interactive frame ~643 ms → ~558 ms.
- `[profile.release]`: thin LTO + `codegen-units = 1`; dev builds optimize dependencies.
- One projection rebuild per command instead of up to three (`document_edit` set both `sync_layers` and `sync_doc`); steady-state recomposite no longer clones the layer vector.

### Fixes

- Brush size/hardness/texture sliders resync after host-side changes; dragging previously broke the value binding so brush presets did not move the slider.
- QML AOT module globs `qml/*.qml` and the build script watches `qml/`, so new components are embedded and rebuilt without hand-registration.
- `preset_json_roundtrip` asserted 3 default brush presets against a library shipping 4.

---

## [docs-archive-removed] — 2026-07-18

### Docs

- Removed `archive/` (former `/docs/` ADRs, journals, checklists, design mockups).
- Handbook (`internal_docs/`) is the sole normative documentation tree.
- Kept [Archived-ADR-to-DR-Map.md](internal_docs/Appendix/Archived-ADR-to-DR-Map.md) as an index of former ADR ids → live Decision Register entries.

---

## [alignment-roadmap] — 2026-07-16

### Docs

- [Alignment Roadmap](internal_docs/Appendix/Alignment-Roadmap.md): tech stack frozen to codebase; agent decisions for all other gaps; Phases 0–5.
- Decision Register: **DR-023** tech stack, **DR-024** single-doc v1, **DR-025** coarse crates, **DR-026** `.ptx` v1; **DR-008** superseded.
- [Implementation Checklist](internal_docs/Appendix/Implementation-Checklist.md) seeded; handbook charter/lifecycle/dev-guide updated for Qt/wgpu stack.

### Alignment stance

Stack = codebase. Contracts (commands, snapshots, workspace models, engine depth) = handbook, incremental.

---

## [engineering-handbook] — 2026-07-16

### Docs

- Adopted `internal_docs/` as the authoritative Engineering Handbook.
- Former `/docs/` was temporarily archived under `archive/docs/` (historical ADRs, journals, checklists); later removed — see `[docs-archive-removed]`.
- Added `internal_docs/Appendix/Codebase-Handbook-Gap-Analysis.md` (code vs handbook diffs + hybrid alignment plan).
- Pointed `README.md` / `AGENTS.md` at handbook; root `SPEC.md` / `CONSTRAINTS.md` remain bridge docs.

### Alignment stance

Keep shipping Qt + wgpu zero-copy + `.ptx` spine; evolve toward handbook command/snapshot/workspace contracts. Do not big-bang rewrite to the proposed fine-grained crate layout.

---

## [ia-parity-roadmap] — 2026-07-16

### Docs

- Merged owner `PREFERED_IA.md` into normative `INFORMATION_ARCHITECTURE.md` with Current / Planned / Blocked / Deferred tags (codebase = shipped truth).
- Retargeted `docs/03-checklists/development.md` production slices for full IA parity.
- Synced `DESIGN_BRIEF.md`, `FEATURES_TODO.md`, `AGENTS.md`, `README.md`; logged ADR tensions (multi-doc, Shape kind, plugins) in `conflicts.md`.

### Still gated

| Item | Gate |
|------|------|
| Document tabs / multi-doc | ADR-013 amendment |
| Shape layers | ADR-017 kind amendment |
| Plugin / script product surface | New ADR |

---

## [decisions-locked-v1] — 2026-07-15

### Decisions Locked

| ADR | Decision | Reversibility | Revisit Date |
|-----|----------|---------------|--------------|
| ADR-001 | Linux / Wayland v1 only | Hard | After Phase 5 |
| ADR-002 | Qt 6 QML shell | Hard | End Phase 1 / Qt 7 |
| ADR-003 | qtbridge primary + hybrid canvas | Medium | End Phase 1–2 |
| ADR-004 | wgpu 30 Vulkan-first | Medium–Hard | Phase 2 interop |
| ADR-005 | Zero-copy compositing only | Hard | Phase 2 present path |
| ADR-006 | Multi-crate Cargo workspace | Medium | End Phase 2 |
| ADR-007 | Command queue threading model | Medium | Phase 4 |
| ADR-008 | SLOs as acceptance gates | Easy | Each phase exit |
| ADR-009 | Layered testing + profiling | Easy | End Phase 3 |

### Constraints at Lock

| Constraint | Status | Notes |
|------------|--------|-------|
| Linux / Wayland | Satisfied | ADR-001 |
| Rust + Qt 6 QML | Satisfied | ADR-002, ADR-003 |
| Zero-copy GPU canvas | Satisfied (design) | ADR-005; runtime unvalidated (spike skipped) |
| Performance SLOs | Satisfied (gates) | ADR-008 |
| qtbridge preferred | Satisfied | ADR-003 with hybrid escape |

### Validated Assumptions

| Assumption | Spike Branch | Result |
|------------|--------------|--------|
| qtbridge builds on host Qt 6.11 / Rust 1.95 | *(spike skipped)* | **Unvalidated in-repo** — host packages present |
| Zero-copy wgpu ↔ Qt RHI | *(spike skipped)* | **Unvalidated** — Phase 2 owns risk |

### Success Criteria Baseline

| Criterion | Target | Current Status |
|-----------|--------|----------------|
| Steady-state FPS | ≥ 60 | Not started |
| Input latency | < 8 ms | Not started |
| Cold boot | < 250 ms | Not started |
| 10×4K composite | < 2 ms GPU | Not started |
| Zero-copy hot path | No full-frame CPU upload | Design locked |

### Known Risks at Lock

1. **qtbridge 0.2 beta API churn**
2. **Custom QQuickItem / RHI import may require C++** (hybrid ADR-003)
3. **Spike skipped** — interop is highest technical risk
4. **RefCell re-entrancy panics** if command boundaries sloppy

### Next Milestone

`agent-bootstrap` → `AGENTS.md` + development checklists → **Phase 1** Cargo/qtbridge bootstrap + QML skeleton.

---

## [grill-round-1-owner-lock] — 2026-07-15

### Owner-confirmed (interactive grill)

| ID | Lock |
|----|------|
| G1 | Linux/Wayland v1 only (ADR-001) |
| G2 | Qt 6 QML (ADR-002) |
| G3 | Hybrid FFI: qtbridge + canvas C++ allowed (ADR-003) |
| G4 | wgpu 30 Vulkan-first (ADR-004) |
| G5 | Zero-copy only + **mandatory interop spike before Phase 2** (ADR-005, **ADR-010**) |

### Process change

Earlier “spike skipped” is **partially reversed**: interop spike is required before Phase 2 production canvas code.

---

## [grill-round-2-owner-lock] — 2026-07-15

| ID | Lock |
|----|------|
| G6 | Multi-crate workspace, strict `phototux_*` naming (ADR-006) |
| G7 | Command queue / phased worker (ADR-007) |
| G8 | Controls 2 first; Kirigami deferred (ADR-002) |
| G9 | SLOs hard gates; ≥60 FPS fluid UX (ADR-008) |
| G10 | Layered testing + HUD/tracing (ADR-009) |
| G11 | Full document graph in Phase 3 only (ADR-011) |
| G12 | GPL-3.0-or-later; public OSS late (ADR-012) |

**Grill status:** Rounds 1–2 complete. Core stack + process locked. Optional Round 3 only for secondary product prefs.

---

## [grill-round-3-owner-lock] — 2026-07-15

| ID | Lock |
|----|------|
| G13 | New doc: ask every time + presets 720p / 1080p / 2K / 4K |
| G14 | Single document only (v1) |
| G15 | Bundled FOSS icon pack under `assets/` (owner supplies pack) |
| G16 | Undo = one committed action/gesture per step |
| G17 | CI: local Arch/CachyOS only for now |
| G18 | Zoom-to-fit on open/new |

**Grill status:** Rounds 1–3 complete. Architecture + product prefs locked (ADR-001…013).

---

## [doc-review-and-desktop-surface] — 2026-07-15

- Doc alignment review: `docs/04-journal/2026-07-15-doc-review.md`
- **ADR-014:** MVP/v1 = **desktop GUI only** (no CLI/TUI product)
- Fixed IA F1 vs New Document presets; SPEC verify path; checklist phase-level rewrite
- CONSTRAINTS hard list updated

---
