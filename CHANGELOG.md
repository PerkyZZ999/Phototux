# Changelog

All notable decision milestones and project state changes.

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
