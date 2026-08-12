# What's Next

Session record and recommended next steps.

**Date:** 2026-08-12
**Branch merged:** `perf/inspector-disclosure-and-cold-boot` (7 commits)
**Reference hardware:** Intel Arc B580 / Mesa 26.1.6 / CachyOS

Authoritative contracts remain in [`internal_docs/`](internal_docs/README.md). This file is a working note, not a spec: when it disagrees with the handbook, the handbook wins.

---

## 1. What was done

### 1.1 Progressive disclosure (handbook 01/28)

The Properties panel was a flat stack of ~25 sections in `Main.qml`, gated only by `visible:`. The handbook specifies a four-level disclosure model; the shell had exactly one disclosure toggle (`propertiesAdvancedOpen`, for advanced color).

Shipped:

- **[`qml/DisclosureGroup.qml`](qml/DisclosureGroup.qml)** — collapsible section with collapsed summary, header badge for hidden warnings, arrow-key grammar, and non-color accessible state.
- **Group registry in the engine** (`default_disclosure_groups()` in [`shell.rs`](crates/phototux-engine/src/shell.rs)), alongside the existing panel and tool descriptors. Ten groups with stable ids, concept titles, levels, and defaults. Levels 3–4 start collapsed, enforced by test.
- **Presence and disclosure as separate axes.** Context decides whether a group exists; the user decides how much shows. A group hidden because the eraser is active is not a collapsed group, and an absent group does not build its body.
- **Expansion persisted as presentation state** — preferences schema 7, sparse override map so untouched groups keep following their descriptor default. Cleared by safe start.
- **Diagnostics group** (level 4) added so the tenth registered id is not dangling.

### 1.2 Cold boot

Startup phases are self-reported on stderr, so these are reproducible with `QT_QPA_PLATFORM=offscreen ./target/release/phototux`.

| Phase | Before | After |
| --- | --- | --- |
| Host construction (`AppSession`) | ~91 ms | **~3 ms** |
| QML root object graph | ~541 ms | ~450 ms |
| First interactive frame | ~643 ms | **~558 ms** |

The win came from `AppSession::new()` spawning `fc-list` synchronously — ~82 ms for a font list only the Character panel reads. Now deferred behind `ensureFontsDiscovered()` with usable fallbacks available immediately.

**A hypothesis that failed, recorded so it is not retried:** lazy-loading all eight dialogs (including the ~450-line preferences dialog) produced **zero** measurable improvement, as did eliminating the `ColorOverlay` shader in `ThemedIcon`. The object-graph cost is concentrated in always-visible chrome, not in dialogs. Lazy dialogs were kept anyway — they are correct on their own terms and measurably help when all content is forced active (563 ms vs 450 ms) — but they are not where startup time goes.

### 1.3 Command path

- `document_edit` set both `sync_layers` and `sync_doc`, so **all 47 document-mutating commands** ran the ~130-line projection rebuild twice. Now once.
- Steady-state `recomposite()` borrows the graph's layer slice instead of cloning every layer; only the filter-gallery preview needs a patched copy.
- `[profile.release]`: thin LTO + `codegen-units = 1`. Dev builds optimize dependencies.

### 1.4 GPU present synchronization (DR-030 / DR-031)

`composite()` blocked on `device.poll(wait_indefinitely())` after submit, and `recomposite()` runs on the UI thread — where handbook 28 forbids waiting for GPU completion.

The wait turned out to be unnecessary. Qt Quick adopts wgpu's `VkInstance`, `VkPhysicalDevice`, `VkDevice` **and** the same queue family/index, so `vkGetDeviceQueue` returns the identical `VkQueue`. A pipeline barrier recorded before submit therefore applies to every later command on that queue, Qt's frame included. The shader-read transition already at the end of the pass is that barrier.

| | Before | After |
| --- | --- | --- |
| Host time in `composite()` | ≥ GPU time (stalled) | **0.05 ms** |
| GPU time, same pass | — | 0.20–0.30 ms |

Kept deliberately: `SharedQueueGuard` (`vkQueueSubmit` needs external synchronization regardless of who waits) and blocking polls on readback/sampling/export (they map GPU memory to host memory).

Measurement had to change with it — `compositeMs` was host wall time around the removed stall, so leaving it would have reported near zero and passed the ADR-008 gate vacuously. Composite is now timed with `TIMESTAMP_QUERY`, collected asynchronously one composite late. **10×4K measures 1.79 ms of GPU time** against the 2 ms gate.

### 1.5 Bugs fixed in passing

- **Brush sliders desynced.** Dragging a `Slider` breaks its `value` binding, so applying a brush preset did not move the size/hardness/texture sliders. A `typeof` guard was papering over one of them.
- **Editing `DisclosureGroup.qml` did not rebuild the app.** `build.rs` listed QML files by name; the AOT `CMakeLists.txt` had the same hardcoded-list pattern that caused commit `4d282d4`. Both now glob/watch the directory.
- **`preset_json_roundtrip` was failing on `main`** — asserted 3 default brush presets against a library shipping 4. Pre-existing.
- **`densityScale` only scaled type**, so "comfortable" gave larger text in identically tight chrome. It now drives spacing, control heights, hit targets, and chrome extents; tool strip and dock read tokens instead of literals.

---

## 2. Verification status

Green: `./scripts/check-rust.sh`, 223 workspace tests, release build, offscreen launch with zero QML errors.

Two verification gaps, both real:

- **No visual confirmation of anything in this session.** The session ran on a tty (`XDG_SESSION_TYPE=tty`), so the regrouped inspector, the density change, and the present path were verified structurally and numerically, not by looking at the canvas. **Do a manual pass before relying on this work.**
- **The one-frame staleness introduced by DR-030** is argued from Vulkan submission-order semantics and the continuous `FrameAnimation` repaint, not observed. Worth watching for during that manual pass, particularly on the first frame after an edit.

Lazily-loaded content hides binding errors, so it was smoke-tested by temporarily forcing every group body and dialog active — clean. That harness is worth rebuilding if the disclosure structure changes significantly.

---

## 3. Recommended next steps

Ordered by value per unit of risk.

### 3.1 Close the loose ends from this session — small, do first

1. **Wire expand/collapse all.** `expandAllDisclosureGroups` and `collapseAllDisclosureGroups` exist in `AppSession` but no QML calls them. Either surface them in the Properties panel header / View menu, or remove them. Dead exported API is worse than neither.
2. **Wire disclosure badges.** `DisclosureGroup` supports `badgeText`/`badgeSeverity`, but only Diagnostics uses it (GPU lost). Handbook 01 requires hidden invalid values to surface at the collapsed header — today a group can hide an invalid value silently. Candidates: out-of-range adjustment parameters, a text layer with a missing font family, a selection that resolves to zero coverage.
3. **Add collapsed summaries to the remaining groups.** Three of ten have one. The summary is what makes a collapsed group worth leaving collapsed.
4. **Manually verify at `comfortable` density and 200% scale.** The density tokens now drive layout; nothing has confirmed the result is not cramped or clipped.

### 3.2 Find the remaining ~260 ms of QML startup

A trivial root window in the same process costs ~283 ms, of which ~190 ms is Qt/QML engine plus Controls module load. `Main.qml` adds ~260 ms on top. Dialogs are ruled out (§1.2), so the cost is in always-visible chrome.

Method that worked for the dialogs and will work here: temporarily wrap a large region in `Loader { active: false }`, rebuild, and measure `QML root loaded` over five runs. Candidates in rough order of size: the right dock's non-Properties panels (Navigator, Swatches, Layers, History), the canvas overlays (five `Canvas` items, thirteen `Shape`/`ShapePath`), the menu tree (28 `Menu`/`MenuItem`), and the tool strip `Repeater`.

Bisect before optimizing. Both of this session's confident startup hypotheses were wrong, and each cost a build-and-measure cycle to disprove.

### 3.3 Replace the string-joined FFI projections

Nineteen `*_joined` projections cross the FFI as `|`-delimited strings — layer names, visibility, kinds, mask flags, clips, selection, history labels/kinds/ids, brush presets, recent colors, effects. Every layer change re-serializes all of them, and QML re-splits them into JS arrays on every dependent binding evaluation.

This is the largest remaining structural inefficiency in the UI path and it scales with layer count, so it gets worse exactly when documents get interesting. A `QAbstractListModel` per collection would remove both the serialize and the parse, and would let `ListView` reuse delegates properly. It is a contained change with a clear boundary — worth doing before the layer panel grows more features.

### 3.4 Split `Main.qml`

Still 6,326 lines, 84% of all QML. Beyond maintainability, extraction is what makes further lazy loading possible: a component in its own file can be `Loader`-gated, an inline block cannot without restructuring. The AOT module now globs `qml/*.qml`, so adding files costs nothing in build configuration.

Natural seams: the canvas and its overlays, the right dock's panels, the menu bar, the status bar.

### 3.5 Present-side latency instrumentation

The ADR-008 tablet input→render < 8 ms gate is **Provisional** and currently unmeasurable: stroke instrumentation reports input→submit, because measuring GPU execution inline would reintroduce the wait DR-030 removed. Closing it needs a timestamp at present time in the canvas item, correlated back to the input event that produced the frame. Until then, do not claim the gate.

### 3.6 Decide `panic = "abort"`

Deliberately not set in `[profile.release]`. It would shrink the binary and is arguably safer given the C++ interop (unwinding across FFI is UB), but it changes crash-recovery behavior — a panic would abort before autosave or recovery could run. That is a product decision for the Decision Register, not a profile tweak.

### 3.7 Split `sync_from_engine()` by dirty domain

Now runs once per command instead of twice, but is still monolithic: every document edit rebuilds the accessibility tree JSON, the effective-preferences JSON, and ~10 joined strings, whether or not the edit touched them. Splitting by domain — layers, selection, color, text, preferences — would let a command rebuild only what it invalidated. Lower priority than §3.3, and largely obsoleted by it for the string projections specifically.

---

## 4. Watch items

- **DR-030 depends on Qt and wgpu sharing one device *and* one queue.** If either stops holding — a Qt version that creates its own queue, a wgpu change in queue selection — the barrier argument collapses and the present path needs an exported timeline semaphore before the host wait can stay removed. The invariant is recorded in the Decision Register; it is not self-enforcing.
- **Timestamp queries are an optional device capability.** Where absent, the composite gate falls back to host timing that overstates GPU cost, and the interactive readout reports unavailable. Check this before trusting `compositeMs` on new hardware.
- **`.cursor/` is untracked** and was left alone. Decide whether to track it or add it to `.gitignore` alongside `.idea/` and `.vscode/`.
