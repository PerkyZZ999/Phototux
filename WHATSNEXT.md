# What's Next

Session record and recommended next steps.

**Date:** 2026-08-12
**Previous session:** `perf/inspector-disclosure-and-cold-boot` (merged, `bdf3a4b`)
**Reference hardware:** Intel Arc B580 / Mesa 26.1.6 / CachyOS

Authoritative contracts remain in [`internal_docs/`](internal_docs/README.md). This file is a working note, not a spec: when it disagrees with the handbook, the handbook wins.

---

## 1. What was done

This session closed §3.1 of the previous note — the loose ends of the disclosure work — and gave the density change its first look on a real display.

### 1.1 Header badges derived from host state

Only Diagnostics had a badge, so a collapsed group could hide an invalid value in silence — which handbook 01/28 forbid.

`inspector_badges()` in [`shell.rs`](crates/phototux-engine/src/shell.rs) now maps group id → `{text, severity}` from a plain `InspectorState` snapshot, published as `inspectorBadgesJson`; [`DisclosureGroup.qml`](qml/DisclosureGroup.qml) resolves its own badge by id, so no call site has to remember to wire one. The rules are a pure function over plain data rather than a method on the session, which is what makes each one testable without a running shell — and the badge is computed identically whether or not the body exists, which the lazy-body contract requires.

Four rules ship: an adjustment parameter outside the editor's range, an active selection whose outline misses the canvas, a text layer whose font family is not installed, and GPU loss.

**Editor ranges became a registered contract.** `adjustment_editor_ranges()` is read by both the sliders and the out-of-range rule, so the two cannot drift apart about what is showable. Editor bounds are deliberately narrower than what `AdjustmentParams::clamped` accepts: a document may legally carry a gamma of 6.0 that the slider cannot reach, and that now raises a badge instead of pinning the control at 3.0 and misreporting the value.

Writing the test for that surfaced a coupling worth knowing about: Levels black at 1.0 makes the engine push white to 1.0001, just outside white's own editor range. The badge tolerance is a thousandth of each parameter's span for exactly that reason, and `slider_extremes_never_raise_a_badge` pins the property — *the editor's own output must never flag itself*.

### 1.2 Collapsed summaries, expand/collapse all

All ten groups now carry a summary, up from three. Where a badge and a summary compete for header width the summary elides first, and the badge elides against a bounded share rather than widening the row — verified with a deliberately overlong badge.

`expandAllDisclosureGroups` / `collapseAllDisclosureGroups` had no caller. They are now registered actions (`action.view.expand-all-groups`, `action.view.collapse-all-groups`) so they keep menu and action-search discovery, *and* a state-reflecting control on the Properties header: collapse while anything is expanded, expand otherwise.

### 1.3 Bugs the visual pass found

- **Panel-header drag areas swallowed button clicks at `comfortable` density.** All five headers reserved a literal 110 px for chrome; four buttons already span 112 px at that density. Each header now measures its own `PanelHeaderControls`. The Properties reserve for the panels below it moved to `Theme.dockStackReserve` for the same reason. Handbook 25 already required chrome extents to read density tokens — this was geometry *derived* from chrome, which the rule now covers explicitly.
- **Collapsed groups pointed the caret up**, contradicting the Right-expands / Left-collapses grammar on the same header. Collapsed points right now.
- **`inspector.brush` was laid out ninth though the registry declares it second.** Handbook 28 forbids reordering registered groups. The block moved, and `inspector_lays_groups_out_in_registry_order` reads `Main.qml` and asserts the two orders match — a declarative layout offers nothing else to assert against.

---

## 2. Verification status

Green: `./scripts/check-rust.sh`, 233 workspace tests, release build, offscreen launch with zero QML errors.

**The visual gap from last session is closed.** This session had a Wayland display, so the shell was reviewed at dense, `comfortable`, and `QT_SCALE_FACTOR=2`, plus a forced-visibility QML override (via `PHOTOTUX_QML`) that puts all ten group headers on screen at once. Density confirmed to drive layout, not only type.

Two things remain unverified, both narrow:

- **The header toggle was never actually pressed.** KWin ignores `ydotool`'s uinput events on this session, so no synthetic pointer click reached the app. Its two states were verified by seeding `disclosure_open` both ways and screenshotting; the click path itself is a four-line handler onto slots that were already exercised. If a later session has working input injection, press it once.
- **DR-030's one-frame staleness** is still argued from Vulkan submission-order semantics, not observed. Nothing in this session's captures showed a stale frame, but nothing was looking for one at frame granularity either.

`PHOTOTUX_QML=/path/to/Main.qml` loads QML from disk with no rebuild. That made the ten-group review cheap and is worth reaching for again — copy `qml/`, patch `visible:` to `true`, run.

---

## 3. Recommended next steps

Ordered by value per unit of risk. §3.1–3.4 carry over from the previous note unchanged in priority; §3.5–3.6 are new findings from this session.

### 3.1 Find the remaining ~260 ms of QML startup

A trivial root window in the same process costs ~283 ms, of which ~190 ms is Qt/QML engine plus Controls module load. `Main.qml` adds ~260 ms on top. Dialogs are ruled out — lazy-loading all eight produced zero measurable improvement — so the cost is in always-visible chrome.

Method: wrap a large region in `Loader { active: false }`, rebuild, measure `QML root loaded` over five runs. Candidates in rough order of size: the right dock's non-Properties panels, the canvas overlays (five `Canvas` items, thirteen `Shape`/`ShapePath`), the menu tree (28 `Menu`/`MenuItem`), the tool strip `Repeater`.

Bisect before optimizing. Both of the previous session's confident startup hypotheses were wrong, and each cost a build-and-measure cycle to disprove.

### 3.2 Replace the string-joined FFI projections

Nineteen `*_joined` projections cross the FFI as `|`-delimited strings — layer names, visibility, kinds, mask flags, clips, selection, history labels/kinds/ids, brush presets, recent colors, effects. Every layer change re-serializes all of them, and QML re-splits them into JS arrays on every dependent binding evaluation.

This is the largest remaining structural inefficiency in the UI path and it scales with layer count, so it gets worse exactly when documents get interesting. A `QAbstractListModel` per collection would remove both the serialize and the parse, and would let `ListView` reuse delegates properly.

### 3.3 Split `Main.qml`

Now ~6,400 lines. Beyond maintainability, extraction is what makes further lazy loading possible: a component in its own file can be `Loader`-gated, an inline block cannot without restructuring. The AOT module globs `qml/*.qml`, so adding files costs nothing in build configuration. Natural seams: the canvas and its overlays, the right dock's panels, the menu bar, the status bar.

### 3.4 Present-side latency instrumentation

The ADR-008 tablet input→render < 8 ms gate is **Provisional** and currently unmeasurable: stroke instrumentation reports input→submit, because measuring GPU execution inline would reintroduce the wait DR-030 removed. Closing it needs a timestamp at present time in the canvas item, correlated back to the input event that produced the frame. Until then, do not claim the gate.

### 3.5 Right-dock height distribution — new

With five panels stacked in a ~900 px window, **Layers and History get header-only height**, and at 200 % scale the Properties body clips mid-control. The model is a fixed fraction (42 %) plus a fixed minimum reserve, neither of which adapts to how many panels are actually stacked. Making the reserve a density token removed the density blindness but not this.

Distributing by content demand — each panel declaring a minimum and a preferred height, the dock allocating from those — would fix both symptoms. Logged as T-024.

### 3.6 Move font discovery off the UI thread — new

`fc-list` costs ~80 ms and is deferred to the first time the Character body builds. That keeps cold boot fast, but it means the missing-font badge cannot fire until then: a document whose text layer names an uninstalled family shows no warning while the group is collapsed. The rule is deliberately silent rather than guessing from the fallback list.

Discovering on a background thread after the first frame would close the hole without putting the subprocess back on any interactive path.

### 3.7 Decide `panic = "abort"`

Deliberately not set in `[profile.release]`. It would shrink the binary and is arguably safer given the C++ interop (unwinding across FFI is UB), but it changes crash-recovery behavior — a panic would abort before autosave or recovery could run. That is a product decision for the Decision Register, not a profile tweak.

### 3.8 Split `sync_from_engine()` by dirty domain

Runs once per command rather than twice, but is still monolithic: every document edit rebuilds the accessibility tree JSON, the effective-preferences JSON, and ~10 joined strings whether or not the edit touched them. Splitting by domain would let a command rebuild only what it invalidated. Lower priority than §3.2, and largely obsoleted by it for the string projections specifically.

---

## 4. Watch items

- **DR-030 depends on Qt and wgpu sharing one device *and* one queue.** If either stops holding, the barrier argument collapses and the present path needs an exported timeline semaphore before the host wait can stay removed. The invariant is in the Decision Register; it is not self-enforcing.
- **Timestamp queries are an optional device capability.** Where absent, the composite gate falls back to host timing that overstates GPU cost, and the readout reports unavailable. Check before trusting `compositeMs` on new hardware.
- **The five legacy `panel_*` bools in `Preferences` carry a field-level `#[serde(default)]`**, which overrides the container default and resolves missing keys to `false` rather than to the product default. A preferences file carrying neither those keys nor `panel_visibility` migrates to *every panel hidden*. Not reachable from any file the app itself writes — found by hand-authoring one during this session's capture harness — but the field-level attributes are redundant and the failure mode is severe.
- **`.cursor/` is untracked** and was left alone. Decide whether to track it or add it to `.gitignore` alongside `.idea/` and `.vscode/`.
