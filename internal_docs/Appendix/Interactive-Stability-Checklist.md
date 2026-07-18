# Interactive Stability Checklist

Living **interactive / GUI** verification checklist for PhotoTux. Use this to hunt bugs, edge cases, conflicts, and regressions until the desktop shell is stable.

**Not** a replacement for:

- Headless core / property / fuzz suites — [31 — Testing](../31-Testing.md)
- Implementation slice tracker — [Handbook-Parity-Checklist.md](Handbook-Parity-Checklist.md)
- Accessibility conformance matrix — [Accessibility-Checklist.md](Accessibility-Checklist.md)

**Related history:** pass journals under `archive/docs/04-journal/*interactive-testing*`.

**Stack:** Linux / Wayland, Qt 6 QML + `qtbridge`, wgpu Vulkan ([DR-023](Decision-Register.md)).  
**Gates:** P11 tiling/spill/sparse and P12 plugin ABI remain gated ([DR-029](Decision-Register.md)) — mark those rows `[N]` or `[!]` until Decision Register amends.

---

## How to run

1. Build: `PATH=/usr/lib/qt6/bin:$PATH cargo build -p phototux`
2. Launch: `PATH=/usr/lib/qt6/bin:$PATH cargo run -p phototux` (or `./target/debug/phototux`)
3. Prefer an isolated Wayland session (kwinmcp / clean Plasma) so host chrome does not steal focus.
4. Mark each item: `[ ]` todo · `[~]` flaky / partial · `[x]` pass · `[!]` fail (log below) · `[N]` skipped / gated
5. On `[!]`: reproduce once, file a row in **Issues log**, fix, retest, then mark `[x]`.
6. After a pass: update **Pass log**, commit fixes + checklist deltas, keep `./scripts/check-rust.sh` green.

**Evidence tips:** status bar text, window title dirty `*`, AT-SPI names (`find_ui_elements` / accessibility tree), app stderr (`[phototux] …`), short screenshot of failure.

---

## 0. Environment & cold boot

Handbook: [02](../02-Application-Lifecycle.md), [30](../30-Performance.md), [DR-017](Decision-Register.md)

- [ ] Binary starts with Qt 6 on `PATH` / `QMAKE`
- [ ] Window maps and focuses under Wayland (listed by compositor / AT)
- [ ] Log reaches `AppSession ready` and `first interactive frame` without panic
- [ ] No silent QML root failure (window visible; welcome or canvas chrome present)
- [ ] Status / FPS / GPU path labels update (no frozen “Working…” forever)
- [ ] Cold interactive frame is plausible on reference hardware (stretch &lt; 250 ms; gate &lt; 1 s when measuring)
- [ ] Safe-start / recovery dialog path does not block forever when entries exist
- [ ] Display ICC discovery completes without hang (colord / env / sRGB fallback)

---

## 1. Lifecycle & documents

Handbook: [02](../02-Application-Lifecycle.md), [10](../10-Document-Model.md), [26](../26-Dialogs.md), [01](../01-Information-Architecture.md)

### 1.1 Welcome & new document

- [ ] Welcome appears when no document and no recovery
- [ ] **New File** opens New Document dialog; Welcome closes
- [ ] **Open File** opens file chooser; Welcome closes
- [ ] Ctrl+N / File → New closes Welcome and opens New Document
- [ ] Presets 720p / 1080p / 2K / 4K create correct size (status / title)
- [ ] Custom width/height create path works
- [ ] Cancel leaves no half-open document; Welcome returns if still empty
- [ ] Enter / Create confirms; Escape cancels
- [ ] Zoom-to-fit on open/new

### 1.2 Open / save / close / dirty

- [ ] Open raster (png/jpeg/…) loads without crash
- [ ] Dirty `*` in title and Unsaved affordance after first edit
- [ ] Save / Save As / Export complete or show actionable error
- [ ] Close last document returns to empty/welcome state without ghost canvas
- [ ] Quit with dirty prompts (unsaved dialog); discard / cancel / save paths
- [ ] Quit clean with no document
- [ ] Recovery list: restore and discard entries

### 1.3 Multi-document tabs (shipped spine)

Handbook: [DR-024](Decision-Register.md) (single-doc v1 amended for tabs)

- [ ] New while another doc open parks prior tab
- [ ] Switch tabs without crash; active tool/layer context follows
- [ ] Dirty flag per tab
- [ ] Close one tab leaves others intact
- [ ] Document limit rejection is user-visible (no silent fail)

---

## 2. Action chrome, menus, shortcuts, palette

Handbook: [01](../01-Information-Architecture.md), [06](../06-Toolbar-System.md), [07](../07-Context-Menus.md), [08](../08-Command-System.md), [09](../09-Shortcut-System.md), [26](../26-Dialogs.md), [Command-Taxonomy](Command-Taxonomy.md)

### 2.1 Menus

- [ ] File / Edit / Select / Image / Layer / Filter / View / Window / Help open
- [ ] Menu items invoke via action IDs (no bypass of `invoke` for document mutations)
- [ ] Disabled items match enablement (no document → save/export disabled)
- [ ] Help → About (or equivalent) opens and closes cleanly

### 2.2 Tool strip & overflow

- [ ] Essentials tools visible or reachable (Brush … Zoom)
- [ ] Active tool highlight matches status `tool.*`
- [ ] Tool switch cancels in-progress transform/crop when required
- [ ] Overflow “More tools” lists remaining tools when strip is short
- [ ] Narrow window: tools remain reachable via overflow / palette
- [ ] Each tool button has Accessible name (AT tree)

### 2.3 Shortcuts

- [ ] Ctrl+N / O / S / Z / Shift+Z / W / Q behave as mapped
- [ ] Ctrl+Shift+P opens command palette
- [ ] Shortcuts yield while text field / on-canvas TextEdit focused
- [ ] Custom keymap in Preferences persists across restart
- [ ] Conflict detection UI surfaces duplicate chords

### 2.4 Command palette

- [ ] Fuzzy filter finds actions by label
- [ ] Enter invokes selected action
- [ ] Escape closes without mutation
- [ ] Rejected commands show status / error (not silent)

### 2.5 Context menus

- [ ] Canvas / layer / selection / mask context menus open
- [ ] Actions match selection / edit target
- [ ] Closing menu restores usable focus

---

## 3. Workspace, docking, panels

Handbook: [03](../03-Workspace-System.md), [04](../04-Docking-System.md), [05](../05-Panel-System.md)

- [ ] Essentials panels present: Properties, Navigator, Swatches, Layers, History
- [ ] Toggle panel visibility (Window menu / actions) without crash
- [ ] Move panel up/down in dock stack
- [ ] Auto-hide / tear-off (if shipped) do not orphan canvas
- [ ] Workspace reset to Essentials restores defaults
- [ ] Workspace preset switch (Essentials / Compact / Painting / Factory)
- [ ] Layout changes do **not** mark document dirty
- [ ] Floating panel clamp stays on-screen after resize

---

## 4. Tools & canvas interaction

Handbook: [06](../06-Toolbar-System.md), [14](../14-Brush-Engine.md), [12](../12-Selection-System.md), [28](../28-UX-Guidelines.md)

### 4.1 Navigation

- [ ] Pan tool / space-drag (if mapped) pans view
- [ ] Zoom tool / wheel zoom; Fit and 100% buttons
- [ ] Rapid zoom/pan does not hang or desync Navigator

### 4.2 Brush / eraser

- [ ] Stroke paints on raster layer; dirty + history entry
- [ ] Brush size / hardness / **texture** sliders affect stroke
- [ ] Eraser removes paint
- [ ] Mid-stroke tool switch ends stroke cleanly (no stuck painting)
- [ ] Brush presets apply (Default / Soft / Hard / Noise Tip)

### 4.3 Selection tools

- [ ] Rect / ellipse / lasso / polygon create selection (ants / status)
- [ ] Shift add / Alt subtract / Shift+Alt intersect (status hint)
- [ ] Deselect (Ctrl+D) clears
- [ ] Empty document: selection tools do not panic

### 4.4 Move / transform / crop

- [ ] Move tool repositions selection or layer per policy
- [ ] Transform: begin, drag handles, Enter apply, Esc cancel
- [ ] Constrain proportions toggle
- [ ] Crop: preview, apply, cancel
- [ ] Switching away from transform/crop cancels in-progress session

### 4.5 Fill / gradient / eyedropper

- [ ] Fill paints FG into layer / respects selection
- [ ] Gradient drag preview + commit
- [ ] Eyedropper samples to foreground (swatches / hex update)

### 4.6 Empty / edge

- [ ] Tools with no document: no panic; status or no-op
- [ ] Click letterbox (outside document) does not corrupt camera

---

## 5. Layers, masks, edit target

Handbook: [11](../11-Layer-System.md), [13](../13-Mask-System.md)

### 5.1 Layers

- [ ] Add raster layer; rename if UI allows
- [ ] Visibility toggle; undo restores
- [ ] Opacity / blend mode from Properties
- [ ] Active layer highlight matches edit target
- [ ] Delete layer; focus/selection fallback sane
- [ ] Lock px / lock pos / lock all block the right edits
- [ ] Clipping / group actions if shipped

### 5.2 Masks

- [ ] Add layer mask
- [ ] Edit target Layer pixels vs Layer mask
- [ ] Paint on mask; composite updates
- [ ] Density / feather / invert / link
- [ ] **Contrast / shift** refine sliders
- [ ] Apply mask / delete mask
- [ ] Mask ops with no mask: rejected or disabled, no crash

### 5.3 Conflicts

- [ ] Filter requiring raster on text/shape layer → clear rejection
- [ ] Mask edit while wrong target selected → no silent pixel write to wrong buffer

---

## 6. Selection vs object vs edit target

Handbook: [01](../01-Information-Architecture.md), [12](../12-Selection-System.md)

- [ ] Pixel selection active ≠ layer selection ≠ keyboard focus (independently observable)
- [ ] Object selection label updates
- [ ] Invert / feather / expand selection commands if shipped
- [ ] Copy/paste selection or layer per clipboard policy ([21](../21-Clipboard.md))

---

## 7. Creative engines (DR-028 depth)

Handbook: [14](../14-Brush-Engine.md), [15](../15-Filter-Engine.md), [18](../18-Text-Engine.md), [19](../19-Shape-Engine.md)

### 7.1 Filters & adjustments

- [ ] Filter Gallery opens (menu or palette)
- [ ] Gaussian / Motion / Emboss / Sharpen / **Noise** preview + apply
- [ ] Cancel preview restores prior pixels
- [ ] Exposure adjustment layer + Properties sliders
- [ ] Brightness/Contrast / levels-style adjustments if present
- [ ] Effect on wrong layer kind → actionable error

### 7.2 Text

- [ ] Text tool + canvas click creates text layer
- [ ] On-canvas editor visible; typing updates layer
- [ ] Character panel: font list, size, tracking, leading, align, color
- [ ] Frame W/H / wrap
- [ ] Bake Text → raster; editor dismisses
- [ ] Shortcut yield while editing text

### 7.3 Shapes & paths

- [ ] Shape tool creates rect (default)
- [ ] Polygon / gradient fill / live vector paths if UI exposes
- [ ] Path Edit: add / move / delete anchor; close toggle
- [ ] Shape boolean partner (two shapes) without crash
- [ ] Vector → raster bake boundary explicit

---

## 8. Color, soft-proof, ICC

Handbook: [16](../16-Color-Management.md)

- [ ] Foreground / background swatches and hex field
- [ ] Recent colors / palette clicks
- [ ] Soft-proof toggle; **Use display profile** path
- [ ] Embed ICC / Clear ICC
- [ ] Display profile name falls back to sRGB when colord absent
- [ ] Soft-proof does not freeze UI

---

## 9. History, undo, lifecycle jobs

Handbook: [20](../20-History-Undo.md), [02](../02-Application-Lifecycle.md)

- [ ] Undo / redo after paint, layer, filter, text
- [ ] History panel lists entries; jump if supported
- [ ] History retention preference applies
- [ ] Long filter / export: cancel if offered; busy state exposed
- [ ] Autosave / recovery after kill −9 mid-edit (spot check)

---

## 10. Import / export / formats

Handbook: [22](../22-Import-Export.md), [27](../27-File-Formats.md)

- [ ] Open `.ptx` round-trip
- [ ] Export PNG / JPEG
- [ ] Export PSD if shipped
- [ ] Corrupt / truncated file → error, no crash
- [ ] Huge dimension rejection or progress (no hang forever)
- [ ] Path with spaces / unicode in filename

---

## 11. Preferences, themes, density

Handbook: [24](../24-Preferences.md), [25](../25-Themes.md), [28](../28-UX-Guidelines.md)

- [ ] Preferences open / close; Esc
- [ ] UI density / high contrast / reduced motion toggles apply
- [ ] Show guides / grid / rulers / snap persist
- [ ] Restore last tool preference
- [ ] Prefs survive restart
- [ ] Theme tokens remain single source (no accidental light flash)

---

## 12. Guides, grid, overlays

Handbook: [10](../10-Document-Model.md), overlays in shell

- [ ] Show/hide grid; overlay redraws on pan/zoom
- [ ] Dirty-rect grid clip does not leave stale lines
- [ ] Guides add / clear; snap when enabled
- [ ] Overlay view generation bumps on camera change

---

## 13. Accessibility & keyboard-only

Handbook: [29](../29-Accessibility.md) · full matrix: [Accessibility-Checklist.md](Accessibility-Checklist.md)

**Smoke subset (always run with interactive pass):**

- [ ] Application / window / Tools toolbar / canvas named in AT-SPI
- [ ] Tool strip buttons named (not icon-only)
- [ ] Dialogs: New File, Create, Cancel named
- [ ] Tab order reaches primary chrome without trap
- [ ] Modal dialog focuses useful control; Esc closes
- [ ] Status region remains readable when panels crowded

*Defer detailed A–K rows to Accessibility-Checklist during a11y-focused passes.*

---

## 14. Rendering, GPU, performance smoke

Handbook: [17](../17-Rendering-Engine.md), [30](../30-Performance.md), [Performance-Budget-Ledger](Performance-Budget-Ledger.md)

- [ ] Steady-state present is GPU path (no full-frame CPU upload as default)
- [ ] Navigator tracks canvas
- [ ] Device-loss UI (if injectable) offers recover
- [ ] Brush stroke remains interactive (no multi-second UI freeze)
- [ ] Composite ms / FPS labels update under paint
- [ ] Zoom/pan ≥ 60 FPS target on reference hardware when measuring (ADR-008 / ledger)

---

## 15. Conflicts, races, hostile UX

Handbook: [08](../08-Command-System.md), [Error-Taxonomy](Error-Taxonomy.md), [31](../31-Testing.md)

- [ ] Rapid undo/redo spam during stroke end
- [ ] Open dialog while IO busy
- [ ] Switch tab mid-filter preview
- [ ] Close document while filter gallery open
- [ ] New document while save in progress
- [ ] Double-click Create / double palette Enter (idempotent or safe reject)
- [ ] Host status markers (`host:document.*`) clear after handling
- [ ] No stuck modal with no focus escape

---

## 16. Gated / out of scope (do not fail the pass)

Mark `[N]` unless Decision Register amends:

- [N] P11 tiling / VRAM spill / sparse buffers ([DR-029](Decision-Register.md))
- [N] P12 third-party plugin ABI / marketplace
- [N] Full lcms2 soft-proof pipeline (residual `[P]` on DR-028)
- [N] Custom AT-SPI D-Bus server beyond Qt Accessible projection
- [N] Cloud / accounts / AI features (product excluded)

---

## Issues log

| ID | Severity | Area (§) | Symptom | Repro | Status |
| --- | --- | --- | --- | --- | --- |
| | blocker / high / med / low / info | | | | open / fixed / deferred |

Severity guide: **blocker** = no window / data loss / crash on smoke; **high** = core workflow broken; **med** = feature wrong or a11y gap; **low** = polish; **info** = expected rejection.

---

## Pass log

| Date | Runner | Env | Smoke | Depth | Notes / commit |
| --- | --- | --- | --- | --- | --- |
| 2026-07-17 | agent (kwinmcp) | Wayland isolated | mostly green | partial DR-028 | `0c78559`, `59ce147` — see archive journal |
| | | | | | |

---

## Sign-off (per pass)

- [ ] §0–§2 green (boot + lifecycle + action chrome)
- [ ] §4–§5 green (tools + layers/masks) or `[!]` filed
- [ ] §7 creative engines exercised or explicitly `[N]` with reason
- [ ] §13 a11y smoke green
- [ ] All `[!]` fixed or deferred with Decision Register / gap note
- [ ] `./scripts/check-rust.sh` green for code fixes
- [ ] This checklist + journal updated; commits landed

---

## Cross references

- [31 — Testing](../31-Testing.md) — pyramid; UI tests wait on semantic barriers
- [Handbook-Parity-Checklist.md](Handbook-Parity-Checklist.md) — what is *supposed* to be implemented
- [Command-Taxonomy.md](Command-Taxonomy.md) — command IDs for palette/menu coverage
- [Accessibility-Checklist.md](Accessibility-Checklist.md) — full a11y matrix
- [Performance-Budget-Ledger.md](Performance-Budget-Ledger.md) — measured budgets
- [Decision-Register.md](Decision-Register.md) — DR-023 / 024 / 028 / 029
- [AGENTS.md](../../AGENTS.md) — agent setup and quality gate
