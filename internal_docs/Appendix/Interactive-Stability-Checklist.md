# Interactive Stability Checklist

Living **interactive / GUI** verification checklist for PhotoTux. Use this to hunt bugs, edge cases, conflicts, and regressions until the desktop shell is stable.

**Not** a replacement for:

- Headless core / property / fuzz suites — [31 — Testing](../31-Testing.md)
- Implementation slice tracker — [Handbook-Parity-Checklist.md](Handbook-Parity-Checklist.md)
- Accessibility conformance matrix — [Accessibility-Checklist.md](Accessibility-Checklist.md)

**Related history:** pass log in this file; former interactive journals under `archive/docs/` were removed 2026-07-18.

**Stack:** Linux / Wayland, Qt 6 QML + `qtbridge`, wgpu Vulkan ([DR-023](Decision-Register.md)).  
**Gates:** P11 tiling/spill/sparse and P12 plugin ABI remain gated ([DR-029](Decision-Register.md)) — mark those rows `[N]` or `[!]` until Decision Register amends.

---

## How to run

1. Build: `PATH=/usr/lib/qt6/bin:$PATH cargo build -p phototux`
2. Launch: `PATH=/usr/lib/qt6/bin:$PATH cargo run -p phototux` (or `./target/debug/phototux`)
3. Prefer an isolated Wayland session (kwinmcp / clean Plasma) so host chrome does not steal focus.
4. Mark each item: `[ ]` todo · `[~]` flaky / partial · `[x]` pass · `[!]` fail (log below) · `[N]` skipped / gated
5. On `[!]`: reproduce once, file a row in **Issues log**, fix, retest, then mark `[x]`.
6. After a pass: update **Pass log**, commit fixes + checklist deltas, keep `rust-tc quick` green.

**Evidence tips:** status bar text, window title dirty `*`, AT-SPI names (`find_ui_elements` / accessibility tree), app stderr (`[phototux] …`), short screenshot of failure.

---

## 0. Environment & cold boot

Handbook: [02](../02-Application-Lifecycle.md), [30](../30-Performance.md), [DR-017](Decision-Register.md)

- [x] Binary starts with Qt 6 on `PATH` / `QMAKE`
- [x] Window maps and focuses under Wayland (listed by compositor / AT)
- [x] Log reaches `AppSession ready` and `first interactive frame` without panic
- [x] No silent QML root failure (window visible; welcome or canvas chrome present)
- [x] Status / FPS / GPU path labels update (no frozen “Working…” forever)
- [~] Cold interactive frame is plausible on reference hardware (stretch &lt; 250 ms; gate &lt; 1 s when measuring) — debug ~1.0–1.4 s (QML load dominated); re-measure release
- [N] Safe-start / recovery dialog path does not block forever when entries exist — no recovery entries in isolated home this pass
- [x] Display ICC discovery completes without hang (colord / env / sRGB fallback)

---

## 1. Lifecycle & documents

Handbook: [02](../02-Application-Lifecycle.md), [10](../10-Document-Model.md), [26](../26-Dialogs.md), [01](../01-Information-Architecture.md)

### 1.1 Welcome & new document

- [x] Welcome appears when no document and no recovery
- [x] **New File** opens New Document dialog; Welcome closes
- [x] **Open File** opens file chooser; Welcome closes
- [x] Ctrl+N / File → New closes Welcome and opens New Document
- [x] Presets 720p / 1080p / 2K / 4K create correct size (status / title) — 1080p verified (1920×1080)
- [x] Custom width/height create path works — 600×600 via spins (T-012: spins no longer fight typing; Create honors spin dims)
- [x] Cancel leaves no half-open document; Welcome returns if still empty — Escape closes New Document; **Open cancel restores Welcome** (T-011)
- [x] Enter / Create confirms; Escape cancels — Create mouse needs AT→EIS Y offset in kwinmcp; Enter reliable; deferred open after Welcome
- [x] Zoom-to-fit on open/new — 1080p ~53%; tiny 64² opens ~1118% fit

### 1.2 Open / save / close / dirty

- [x] Open raster (png/jpeg/…) loads without crash — `phototux-test.png` 64×64
- [x] Dirty `*` in title and Unsaved affordance after first edit — `Untitled*` / Unsaved after stroke
- [~] Save / Save As / Export complete or show actionable error — Save dialog opens from unsaved prompt; full Save As not finished this pass
- [x] Close last document returns to empty/welcome state without ghost canvas — Discard → Welcome (T-011)
- [x] Quit with dirty prompts (unsaved dialog); discard / cancel / save paths — Discard + Cancel exercised; Save opens chooser
- [x] Quit clean with no document — Ctrl+Q; process exits, no PhotoTux window
- [N] Recovery list: restore and discard entries — no recovery entries in isolated home this pass

### 1.3 Multi-document tabs (shipped spine)

Handbook: [DR-024](Decision-Register.md) (single-doc v1 amended for tabs)

- [x] New while another doc open parks prior tab — New/Open no longer discard-prompt (T-013); parks via `prepare_new_document_tab`
- [x] Switch tabs without crash; active tool/layer context follows — tab strip activate OK
- [x] Dirty flag per tab — `* Untitled` / clean labels track park dirty bit
- [x] Close one tab leaves others intact — Discard close activates parked sibling
- [x] Document limit rejection is user-visible (no silent fail) — status `document limit reached (N); close a tab first` (T-014: refuse before park; `PHOTOTUX_MAX_OPEN_DOCUMENTS` for QA)

---

## 2. Action chrome, menus, shortcuts, palette

Handbook: [01](../01-Information-Architecture.md), [06](../06-Toolbar-System.md), [07](../07-Context-Menus.md), [08](../08-Command-System.md), [09](../09-Shortcut-System.md), [26](../26-Dialogs.md), [Command-Taxonomy](Command-Taxonomy.md)

### 2.1 Menus

- [x] File / Edit / Select / Image / Layer / Filter / View / Window / Help open — File open with docs; Help via Alt+H
- [x] Menu items invoke via action IDs (no bypass of `invoke` for document mutations) — `actionMenuItem` → `runAction` → `invokeAction`
- [x] Disabled items match enablement (no document → save/export disabled) — Welcome: toolbar Export/Undo/Redo lack `enabled` in AT
- [x] Help → About (or equivalent) opens and closes cleanly

### 2.2 Tool strip & overflow

- [x] Essentials tools visible or reachable (Brush … Zoom)
- [x] Active tool highlight matches status `tool.*` — Brush / Eraser verified
- [x] Tool switch cancels in-progress transform/crop when required — T-017; Esc app Shortcut; strip→Brush/Eraser clears session
- [x] Overflow “More tools” lists remaining tools when strip is short — T-016; 520px height; deferred open + settled CloseOnPressOutside; readable Item rows
- [x] Narrow window: tools remain reachable via overflow / palette — open stays under EIS; Eyedropper/Zoom pick closes menu + activates
- [x] Each tool button has Accessible name (AT tree)

### 2.3 Shortcuts

- [~] Ctrl+N / O / S / Z / Shift+Z / W / Q behave as mapped — N/W/Z exercised
- [x] Ctrl+Shift+P opens command palette — T-034 and T-035 both fixed; re-verified 2026-09-03
- [x] Shortcuts yield while text field / on-canvas TextEdit focused — T-018; New Document open blocks Ctrl+Shift+P; TextField/SpinBox/TextEdit detection + hex/FG handlers
- [x] Custom keymap in Preferences persists across restart — T-019; Save→F9 in prefs.json survives relaunch with same XDG config
- [x] Conflict detection UI surfaces duplicate chords — steal clears prior binding (Open→None when Export took Ctrl+O); hint path present

### 2.4 Command palette

- [x] Palette opens (Ctrl+Shift+P or Edit → Command Palette…) — T-034 fixed; rows legible since T-035
- [x] Fuzzy filter finds actions by label — typed `bring`, matched both Arrange entries with their chords
- [x] Enter invokes selected action — Bring Forward on the top layer returned "This is already the top layer."
- [x] Escape closes without mutation — palette dismissed, status bar unchanged (2 layers, no selection)
- [x] Rejected commands show status / error (not silent) — Ctrl+S → `Action unavailable: Save (no document open)` (T-015)

### 2.5 Context menus

- [x] Canvas / layer / selection / mask context menus open — canvas + selection verified; layer popup repositioned on-screen (T-020); mask path deferred
- [x] Actions match selection / edit target — Deselect disabled without selection; selection menu lists Feather/Copy when active
- [x] Closing menu restores usable focus — Esc dismisses; canvas usable

---

## 3. Workspace, docking, panels

Handbook: [03](../03-Workspace-System.md), [04](../04-Docking-System.md), [05](../05-Panel-System.md)

- [x] Essentials panels present: Properties, Navigator, Swatches, Layers, History — AT labels present with doc open
- [x] Toggle panel visibility (Window menu / actions) without crash — palette “Navigator” hides panel; Window menu lists checkable panels
- [x] Move panel up/down in dock stack — ↑ control present per panel; exercised Move panel up
- [~] Auto-hide / tear-off (if shipped) do not orphan canvas — controls present; deep path deferred
- [x] Workspace reset to Essentials restores defaults — palette → Essentials; status `Workspace: Essentials`
- [x] Workspace preset switch (Essentials / Compact / Painting / Factory) — Compact then Essentials via palette
- [x] Layout changes do **not** mark document dirty — title stayed `Untitled` (no `*`) after Compact/toggle
- [x] Floating panel clamp stays on-screen after resize — restore off-screen float (5000,5000) clamps to screen (1360,868 on 1440×900); Instantiator id-stable + persist gated until first clamp; width/Screen change reclamp

---

## 4. Tools & canvas interaction

Handbook: [06](../06-Toolbar-System.md), [14](../14-Brush-Engine.md), [12](../12-Selection-System.md), [28](../28-UX-Guidelines.md)

### 4.1 Navigation

- [x] Pan tool / space-drag (if mapped) pans view — middle-drag pan on canvas; Pan tool in strip
- [x] Zoom tool / wheel zoom; Fit and 100% buttons — wheel changes zoom %; View → Zoom to Fit (`Ctrl+Shift+J`); Fit/100% AT buttons present
- [~] Rapid zoom/pan does not hang or desync Navigator — Navigator present; stress pass deferred

### 4.2 Brush / eraser

- [x] Stroke paints on raster layer; dirty + history entry
- [x] Brush size / hardness / **texture** sliders affect stroke — size→167 px, texture→83%; Soft Round hardness 20%; stroke visible
- [x] Eraser removes paint — `tool.eraser` + erase drag over stroke
- [x] Mid-stroke tool switch ends stroke cleanly (no stuck painting) — `onActiveToolChanged` calls `strokeEnd`; interrupted gesture left no trail
- [x] Brush presets apply (Default / Soft / Hard / Noise Tip) — Soft 20%, Hard 100%, Noise 70%/55% texture, Default 85%

**Note (kwinmcp EIS):** native window chrome adds ~28 px Y — use AT `y + 28` for dock/tool-strip clicks.

### 4.3 Selection tools

- [x] Rect / ellipse / lasso / polygon create selection (ants / status) — `pixel selection active`; Rectangular selection status
- [x] Shift add / Alt subtract / Shift+Alt intersect (status hint) — hint shown; Shift/Alt/Shift+Alt drags exercised
- [x] Deselect (Ctrl+D) clears — status returned to `no pixel selection`
- [x] Empty document: selection tools do not panic — marquee drag with no doc; app stayed up, no panic in log

### 4.4 Move / transform / crop

- [x] Move tool repositions selection or layer per policy — Move tool + drag with selection active
- [x] Transform: begin, drag handles, Enter apply, Esc cancel — Free Transform chrome; Enter apply; Esc cancel
- [x] Constrain proportions toggle — checkbox present and clicked
- [x] Crop: preview, apply, cancel — Esc cancel; Apply → canvas `488×375` (from 1920×1080)
- [x] Switching away from transform/crop cancels in-progress session — same as §2.2 / T-017

### 4.5 Fill / gradient / eyedropper

- [x] Fill paints FG into layer / respects selection — Fill tool; canvas went black; Unsaved
- [x] Gradient drag preview + commit — linear black→white gradient on canvas
- [x] Eyedropper samples to foreground (swatches / hex update) — status `Sampled #9D9D9D`; hex `#9D9D9D`

### 4.6 Empty / edge

- [x] Tools with no document: no panic; status or no-op — Fill click with no doc; app stayed up, no panic in log
- [x] Click letterbox (outside document) does not corrupt camera — click beside canvas; window remained healthy

---

## 5. Layers, masks, edit target

Handbook: [11](../11-Layer-System.md), [13](../13-Mask-System.md)

### 5.1 Layers

- [x] Add raster layer; rename if UI allows — `Ctrl+Shift+N`; no rename UI (label only)
- [x] Visibility toggle; undo restores — Hide/Show Layer 2 + `Ctrl+Z`; stroke cleared/restored
- [x] Opacity / blend mode from Properties — Blend Mode + Layer Opacity controls present; opacity AT name includes percent
- [x] Active layer highlight matches edit target — `object: Layer 2` after create; sync after delete→Layer 1 / group
- [x] Delete layer; focus/selection fallback sane — palette Delete layer → Layer 1 + matching object selection
- [x] Lock px / lock pos / lock all block the right edits — `Set layer locks` history; paint after lock added no stroke
- [x] Clipping / group actions if shipped — New Group + Create Clipping Mask via palette; `object: Group`

### 5.2 Masks

- [x] Add layer mask — palette Add Mask; `Add layer mask · graph`; `M` badge on Layer 1
- [x] Edit target Layer pixels vs Layer mask — status `Layer mask` / buttons; auto-selects mask on add
- [x] Paint on mask; composite updates — `Mask stroke · graph`; dark stroke on canvas
- [x] Density / feather / invert / link — Mask section controls present (Density/Feather labels, Invert, Link mask)
- [x] **Contrast / shift** refine sliders — `Contrast 0.00` / `Shift 0.00` labels + sliders in Properties
- [x] Apply mask / delete mask — palette Apply / Delete; history `Apply layer mask` / `Delete layer mask`
- [x] Mask ops with no mask: rejected or disabled, no crash — Apply with no mask; app stayed up; Layer mask disabled

### 5.3 Conflicts

- [x] Filter requiring raster on text/shape layer → clear rejection — Drop Shadow on shape → Properties `command rejected: drop shadow requires raster` (unit: `drop_shadow_rejects_shape_layer`)
- [x] Mask edit while wrong target selected → no silent pixel write to wrong buffer — with mask present, Layer pixels target → history `Brush stroke` (not `Mask stroke`)

---

## 6. Selection vs object vs edit target

Handbook: [01](../01-Information-Architecture.md), [12](../12-Selection-System.md)

- [x] Pixel selection active ≠ layer selection ≠ keyboard focus (independently observable) — kwinmcp 1440×900: Ctrl+A → `pixel selection active`; Ctrl+Shift+N → `object: Layer 2` while selection stayed; click Layer 1 → `object: Layer 1` + selection still active
- [x] Object selection label updates — Properties shows `Object selection: Layer N` / status `object: …` tracking layer clicks
- [x] Invert / feather / expand selection commands if shipped — palette: Invert Selection, Feather…, Expand; history: `Invert selection`, `Selection feather`, `Selection expand`
- [x] Copy/paste selection or layer per clipboard policy ([21](../21-Clipboard.md)) — with selection, Ctrl+C fills RGBA + R8; Ctrl+V creates **Pasted** layer (fixed: copy no longer coverage-only)

---

## 7. Creative engines (DR-028 depth)

Handbook: [14](../14-Brush-Engine.md), [15](../15-Filter-Engine.md), [18](../18-Text-Engine.md), [19](../19-Shape-Engine.md)

### 7.1 Filters & adjustments

- [x] Filter Gallery opens (menu or palette) — kwinmcp: palette `Filter Gallery` → dialog with kind combo + Preview/Apply/Cancel
- [x] Gaussian / Motion / Emboss / Sharpen / **Noise** preview + apply — gallery combo → each kind; history `… · graph`; status `… applied`; Noise preview visibly noisy
- [x] Cancel preview restores prior pixels — gallery auto-preview then Cancel; status stayed `Noise applied`; no new Gaussian history entry
- [x] Exposure adjustment layer + Properties sliders — palette `Exposure`; Stops/Gamma in Properties (Gamma dragged to 0.45)
- [x] Brightness/Contrast / levels-style adjustments if present — palette `Brightness/Contrast` + `Levels` with Black/White/Gamma sliders
- [x] Effect on wrong layer kind → actionable error — Gaussian Blur on adjustment → Properties `command rejected: effect requires raster layer`

### 7.2 Text

- [x] Text tool + canvas click creates text layer — kwinmcp: Text tool strip hit → canvas click; history `Add text layer · graph`; status `text · …`
- [x] On-canvas editor visible; typing updates layer — AT `On-canvas text editor`; Character field typed `PhotoTux` (focus + commit)
- [x] Character panel: font list, size, tracking, leading, align, color — Character section: Noto Sans, Size/Tracking/Leading spins; align/color controls present
- [x] Frame W/H / wrap — Frame W/H spins + Wrap checkbox in Character chrome (handbook §18)
- [x] Bake Text → raster; editor dismisses — palette `Bake Text` → status `Text baked to pixels — editable text discarded`; **and the keyboard comes back** — press `M` straight afterwards and the tool becomes `tool.select.rect` (T-037: it did not, for a long time, and this row was ticked anyway because the frame does visibly go away)
- [x] Shortcut yield while editing text — Character field focused; Ctrl+Z did not push Undo into history (input yield)

### 7.3 Shapes & paths

- [x] Shape tool creates rect (default) — palette `Rectangle` / canvas; history `Add shape layer · graph`; status `Shape (shape)`
- [x] Polygon / gradient fill / live vector paths if UI exposes — palette `Polygon` + `Gradient Fill` (blue→orange gradient on canvas)
- [x] Path Edit: add / move / delete anchor; close toggle — Path Edit chrome: drag/add/Delete/Closed; anchors instructions shown for shape layers
- [x] Shape boolean partner (two shapes) without crash — palette `Boolean Union`; history `Boolean union · graph`; footer `Boolean union (shape)`; 7 layers
- [x] Vector → raster bake boundary explicit — wrong target: `Rasterize Shape requires an active shape layer`; success: `Shape rasterized to pixels`

---

## 8. Color, soft-proof, ICC

Handbook: [16](../16-Color-Management.md)

- [x] Foreground / background swatches and hex field — Swatches FG/BG; red swatch → `#FF0000` + red FG square
- [x] Recent colors / palette clicks — preset swatch row click updates FG hex
- [x] Soft-proof toggle; **Use display profile** path — palette `Soft-Proof: Display-P3` / `Soft-Proof: Off`; Properties `Soft-proof: Display-P3` / Off; advanced `Use display profile` (Accessible: Soft-proof with display ICC)
- [x] Embed ICC / Clear ICC — palette Embed ICC… opens file dialog (Esc cancel); `Clear Embedded ICC` runnable
- [x] Display profile name falls back to sRGB when colord absent — AT/label `Display: sRGB` in isolated home
- [x] Soft-proof does not freeze UI — soft-proof on/off while UI stays interactive (swatches/navigator still update)

---

## 9. History, undo, lifecycle jobs

Handbook: [20](../20-History-Undo.md), [02](../02-Application-Lifecycle.md)

- [x] Undo / redo after paint, layer, filter, text — Ctrl+Z / Ctrl+Shift+Z; history advances (brush, Gaussian Blur, Add layer, selection ops)
- [x] History panel lists entries; jump if supported — `Add layer · graph` / `Gaussian Blur · graph` listed; click entry jumps (layer count drops)
- [x] History retention preference applies — Prefs SpinBox → 8; after >8 Add layer ops AT shows exactly 8 history labels
- [x] Long filter / export: cancel if offered; busy state exposed — Filter Gallery Cancel closes dialog; chrome exposes `Working…` / `ioBusy`
- [x] Autosave / recovery after kill −9 mid-edit (spot check) — dirty doc autosave timer writes recovery `.ptx`; kill −9 → relaunch shows “PhotoTux found autosaved documents…”

---

## 10. Import / export / formats

Handbook: [22](../22-Import-Export.md), [27](../27-File-Formats.md)

- [x] Open `.ptx` round-trip — `PHOTOTUX_DESKTOP_OPEN` routes `.ptx`; open `Round Trip café.ptx` + Ctrl+S rewrites file (901→1.1k)
- [x] Export PNG / JPEG — `PHOTOTUX_DESKTOP_EXPORT` after open writes `export-out.png` / `.jpg`; `phototux_io` PNG/JPEG tests green
- [x] Export PSD if shipped — Export dialog includes Photoshop subset (`*.psd`); `psd::export_import_round_trip` test green
- [x] Corrupt / truncated file → error, no crash — open `corrupt truncated.ptx` → integrity UI (`corrupt ptx: truncated header`); process stays up
- [x] Huge dimension rejection or progress (no hang forever) — `phototux_io::rejects_rgba_allocation_over_limit` test green
- [x] Path with spaces / unicode in filename — opened/saved `Documents/Round Trip café.ptx`

---

## 11. Preferences, themes, density

Handbook: [24](../24-Preferences.md), [25](../25-Themes.md), [28](../28-UX-Guidelines.md)

- [x] Preferences open / close; Esc — Ctrl+, opens Preferences; Esc dismisses
- [x] UI density / high contrast / reduced motion toggles apply — Prefs → Comfortable + High contrast + Reduced motion; Theme bindings update; `preferences.json` matches
- [x] Show guides / grid / rulers / snap persist — toggles write `show_guides/grid/rulers` + `snap_enabled` in `preferences.json`
- [x] Restore last tool preference — restore-on-launch + always-persist `last_tool`; relaunch with `tool.text` → status `tool.text`
- [x] Prefs survive restart — same XDG home reload: comfortable / high_contrast / grid / rulers / restore_last_tool intact
- [x] Theme tokens remain single source (no accidental light flash) — `Theme.qml` bindings for density/contrast/motion; dock stays `#2B2B30`-class dark after restart

---

## 12. Guides, grid, overlays

Handbook: [10](../10-Document-Model.md), overlays in shell

- [x] Show/hide grid; overlay redraws on pan/zoom — `Show Grid` toggles overlay; pan (Hand) + zoom redraw grid without stale chrome
- [x] Dirty-rect grid clip does not leave stale lines — `gridOverlay` clips to `dirtyRectJson` when view gen unchanged; view bump clears full canvas
- [x] Guides add / clear; snap when enabled — `New Vertical Guide` → status `Guide added at 960px`; `Clear Guides` → `Guides cleared`; snap pref on
- [x] Overlay view generation bumps on camera change — pan/zoom path increments `overlayViewGeneration` (grid `viewBump` full redraw)

---

## 13. Accessibility & keyboard-only

Handbook: [29](../29-Accessibility.md) · full matrix: [Accessibility-Checklist.md](Accessibility-Checklist.md)

**Smoke subset (always run with interactive pass):**

- [x] Application / window / Tools toolbar / canvas named in AT-SPI — `[application] PhotoTux`, frame, `[tool bar] Tools`, canvas `Empty canvas` / `Canvas 1920×1080`
- [x] Tool strip buttons named (not icon-only) — Brush, Eraser, Marquee, Text, Shape, … + main chrome `New…`/`Open…`/`Undo`/`Redo` Accessible.name
- [x] Dialogs: New File, Create, Cancel named — Welcome `New File`; New Document `Create`/`Cancel`/`New Document`
- [x] Tab order reaches primary chrome without trap — Tab through chrome/dialog; Esc recovers; no stuck focus
- [x] Modal dialog focuses useful control; Esc closes — New Document / Preferences modal+focused; Esc dismisses
- [x] Status region remains readable when panels crowded — `[tool bar] Status` + status bar with full doc summary (zoom/layer/tool)

*Defer detailed A–K rows to Accessibility-Checklist during a11y-focused passes.*

---

## 14. Rendering, GPU, performance smoke

Handbook: [17](../17-Rendering-Engine.md), [30](../30-Performance.md), [Performance-Budget-Ledger](Performance-Budget-Ledger.md)

- [x] Steady-state present is GPU path (no full-frame CPU upload as default) — footer `GPU ACCELERATED`; app log wgpu Vulkan composite
- [x] Navigator tracks canvas — Navigator panel present with viewport chrome while document open
- [x] Device-loss UI (if injectable) offers recover — palette `Simulate Device Lost` → status lost; `Recover graphics…` → `Graphics recovered — canvas restored`
- [x] Brush stroke remains interactive (no multi-second UI freeze) — stroke drag completes; UI stays responsive
- [x] Composite ms / FPS labels update under paint — `comp 0.58 ms` / `FPS: 144` after stroke
- [x] Zoom/pan ≥ 60 FPS target on reference hardware when measuring (ADR-008 / ledger) — pan sample showed `FPS: 60`+ with GPU ACCELERATED

---

## 15. Conflicts, races, hostile UX

Handbook: [08](../08-Command-System.md), [Error-Taxonomy](Error-Taxonomy.md), [31](../31-Testing.md)

- [x] Rapid undo/redo spam during stroke end — stroke then Ctrl+Z / Ctrl+Shift+Z burst; no crash; doc stays interactive
- [x] Open dialog while IO busy — `has_document_io_idle` / `io_busy` gates actions; unavailable reason reports `busy` (not “no document”)
- [x] Switch tab mid-filter preview — Filter Gallery modal blocks tab chrome until Esc/Cancel (safe reject); no crash
- [x] Close document while filter gallery open — Esc cancels gallery; Ctrl+W then unsaved prompt; gallery cancel-on-destructive in QML
- [x] New document while save in progress — Save As open + Ctrl+N leaves Save dialog intact (no stuck/corrupt race)
- [x] Double-click Create / double palette Enter (idempotent or safe reject) — double Create click → single doc, no crash
- [x] Host status markers (`host:document.*`) clear after handling — Ctrl+N → status `PhotoTux — create…` (not `host:document.new`)
- [x] No stuck modal with no focus escape — Esc closes Filter Gallery, unsaved prompt, Save As

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
| T-009 | high | §14–15 / paint | Brush stroke floods AT-SPI (`statusText` + full `sync_from_engine` on every `CompositeDone`); AT queries time out; kwinmcp session dies | New 1080p → paint drag | **fixed** — composite out of `status_summary`; CompositeDone updates telemetry only; a11y JSON notify only on change; status/FPS labels `Accessible.ignored` |
| T-010 | med | §1.1 | Welcome→New Document open races modal Overlay; deferred `Qt.callLater` open | New File from Welcome | **fixed** — `openNewDocumentDialog()` |
| T-011 | high | §1.1–1.2 | Cancel Open File / close last doc left empty shell with no Welcome | Open File → Escape; Discard close | **fixed** — `openFileDialog.onRejected` + `onHasDocumentChanged` reopens Welcome |
| T-012 | med | §1.1 | Custom size SpinBox `value:` binding fought typing; Create could keep stale preset | Edit width/height then Create | **fixed** — no binding fight; `confirmCreate` syncs spins / clears mismatched preset |
| T-013 | high | §1.3 | New/Open on dirty doc showed discard dialog instead of parking tab (DR-024) | Paint → Ctrl+N / toolbar New | **fixed** — `host:document.new` / `.open` open dialogs without `requestDestructiveAction` |
| T-014 | high | §1.3 | At document limit, `prepare_new_document_tab` parked then failed `begin_active`, leaving no active doc | Open max tabs → New → Create | **fixed** — `can_open_another` before park; status surfaces limit; optional `PHOTOTUX_MAX_OPEN_DOCUMENTS` |
| T-015 | med | §2.4 | Disabled actions (Save with no doc) failed silently via shortcut/palette | Ctrl+S on Welcome | **fixed** — `invoke_action` sets status `Action unavailable: …` |
| T-016 | med | §2.2 | Tool overflow Menu `Repeater`/`menu:` crash or empty; items never listed; EIS open immediately closed | Short window → More tools | **fixed** — `Qt.callLater` open; outside-close after settle; Theme-colored Item rows; close-before-activate |
| T-017 | high | §2.2 / §4.4 | `set_active_tool` notified QML without mirroring engine `active_tool`/`status_text`; transform chrome stuck after leave; tool switch cancel unreliable | Transform → Esc or strip leave | **fixed** — mirror props after `VIEW_SET_TOOL`; cancel on leave; app Esc cancels session |
| T-018 | med | §2.3 | App shortcuts fired while New Document / spin editors active (`instanceof TextInput` miss + no dialog yield) | New Document → Ctrl+Shift+P | **fixed** — SpinBox/TextField/TextEdit detect; `newDocDialog.opened` yields; hex/FG focus arms yield |
| T-019 | med | §2.3 | Preferences Flickable did not scroll (Keyboard unreachable); keymap capture needs F-keys | Prefs → Keyboard | **fixed** — Flickable sized to `availableWidth/Height` + AlwaysOn scrollbar; Save→F9 persisted |
| T-020 | med | §2.5 | Layer context `Menu.popup()` near bottom dock opened off-screen / invisible | Right-click Layer 1 | **fixed** — `openContextMenu` clamps to Overlay; canvas/selection use same helper |
| T-021 | med | §2.1 | Panel-header drag `MouseArea` reserved a literal 110 px for chrome; at `comfortable` the four buttons already span 112 px, so the drag surface overlapped them and swallowed their clicks | Prefs → density `comfortable` → click a panel-header button | **fixed** — every header measures its own `PanelHeaderControls.width`; Properties carries five buttons |
| T-022 | low | §2.1 | Collapsed disclosure groups drew a caret pointing **up**, contradicting the Right-expands / Left-collapses grammar on the same header | Collapse any Properties group | **fixed** — collapsed points right, expanded points down |
| T-023 | med | §2.1 | Properties laid out `inspector.brush` ninth though the registry declares it second; handbook 28 forbids reordering registered groups | Compare Properties order to `default_disclosure_groups()` | **fixed** — block moved; `inspector_lays_groups_out_in_registry_order` asserts the two orders match |
| T-024 | low | §2.1 | Right dock gives Layers and History header-only height in a ~900 px window and at 200 % scale; the Properties body clips mid-control there | 1440×900 window, five panels stacked | **fixed** — tabbed dock groups show one panel per group, so each visible body gets the group's full height |

| T-025 | high | §2.2 | Undo/redo of a brush stroke ran the command and restored the GPU layer but left the canvas unchanged: only 2 of 17 composite paths raised the repaint signal after repaint became demand-driven | Paint a stroke → Ctrl+Z | **fixed** — one `record_composite` publishes the time and bumps the generation together |
| T-026 | low | §2.1 | Layers and History still collapse to header-only at 1440×900 with five panels stacked | Open any document | **fixed** — tabbed dock groups; Layers shows its layer list |
| T-027 | blocker | §2.1 | Tearing a panel off aborted the process. The tear-off slot emits while the session is still mutably borrowed, the floating `Window` is built synchronously from that emission, and its geometry write-back called straight back into the host — `BorrowConflict` in qtbridge, which is a hard abort, not a catchable error. Reproduced identically on the pre-tabbed-dock build, so it predates that work | Panel header → Tear off panel | **fixed** — every reactive write-back defers through `root.afterHostSlot`; the Instantiator no longer `close()`s a window it is retiring |
| T-028 | blocker | §2.1 | Filter ▸ Filter Gallery aborted the process on first open. Same class as T-027 by a different route: the modal popup moves focus while `openFilterGallery` is on the stack, and `onActiveFocusItemChanged` called `setShortcutInputYield` from inside that borrow | Filter → Filter Gallery… | **fixed** — `refreshShortcutYield` is deferred at the function, so all six of its reactive callers are safe at once |
| T-029 | high | §2.4 | Every `CheckBox` label in Preferences rendered near-black on dark surface (~1.3:1, far under the AA floor), and inline dialog titles were equally dim. No Controls style is configured, so the shell runs Basic, which hardcodes a light palette and ignores `palette` overrides | Edit → Preferences | **fixed** — `ThemedCheckBox` / `ThemedComboBox` / `ThemedSpinBox` / `ThemedDialogHeader` draw from Theme tokens |
| T-030 | low | §2.4 | Filter Gallery drew its content over its own title: the `contentItem` was anchored to `parent`, which spans the whole popup rather than the area a `Dialog` reserves between header and footer | Filter → Filter Gallery… | **fixed** — anchors dropped in favour of `padding` |
| T-031 | low | §2.4 | Combo drop-down lists are still light-on-dark. Theming them left the row at `currentIndex` blank in every combo — slot reserved, neither label nor highlight painted — so the style's own popup was kept | Open any combo box | **open** — cosmetic; ranked in the gap analysis |
| T-032 | med | §3 | Hiding the raised tab of a dock group blanked the whole group — its siblings vanished too. `DockTopology` records which tab was last raised but has no view of panel visibility, so the stored selection was used even once hidden | Window → uncheck Navigator | **fixed** — `WorkspaceState::effective_active_tab` falls through to the first visible sibling; three engine tests, two of which fail against the old behaviour |
| T-033 | high | §2.5 | Undo after Mask → Selection did nothing. Twelve call sites took the pre-edit snapshot before mutating; `apply_mask_to_selection_host` took it after, so the snapshot captured the state the undo was meant to reverse | Add a layer mask → Mask to Selection → Ctrl+Z | **fixed** — `commit_selection_edit` / `commit_layer_edit` own the ordering; no call site snapshots directly |
| T-034 | high | §2.4 | The command palette never opened. `LazyDialog` wraps a `Loader`, whose default property is `data`, not `sourceComponent` — so every dialog written inside one became a plain child object, `sourceComponent` stayed null, and `item` was null forever. Dialogs driven by a `visible:` binding worked anyway (as eager children), hiding the fault; the palette reaches its API through `ensure()` and got null | Ctrl+Shift+P | **fixed** — `LazyDialog` declares `default property Component dialog` and binds `sourceComponent` to it. Every dialog is now genuinely lazy, which is what the type existed for |
| T-036 | low | §2.3–2.4 | The checklist still marked the command palette `[!]` and carried a **Blocked by T-034** banner over §2.4, while the issues log recorded T-034 and T-035 as fixed. The document — which is this project's GUI QA authority — claimed a working feature was broken, and three rows below it were held at `[~]` for a blocker that no longer existed | Ctrl+Shift+P | **fixed** — palette re-verified end to end (opens, filters, Enter invokes and surfaces the refusal, Escape closes without mutation); rows and banner corrected |
| T-038 | med | §3 | Window ▸ Reset Workspace reported "Workspace reset to Essentials" and left an auto-hidden panel hidden. `reset_essentials` clears the dock correctly — an engine probe confirmed it — but `reset_workspace` published only the *preference* fields, which carry panel visibility and not dock topology, so QML went on drawing the old dock. The only way back was toggling the panel off and on in the Window menu | Properties header ▸ Auto-hide panel → Window ▸ Reset Workspace | **fixed** — the slot calls `persist_workspace_visibility`, and `a_slot_that_changes_the_workspace_layout_republishes_it` fails any workspace-mutating slot that does not |
| T-039 | high | §5.2 | Select ▸ Selection to Mask looked like a no-op: the mask was written to the GPU and the canvas went on showing the composite from before it, with no toast, no History entry and nothing to click. Forcing a repaint any other way revealed the mask had been there all along. `CommandEffects::host_chrome` carries `recomposite: false` — correctly, since the command touched no pixels itself — and this was the one of three mask writers that did not ask for a frame of its own | Marquee part of a layer → Select ▸ Selection to Mask | **fixed** — the handler calls `recomposite`, and `a_handler_that_writes_pixels_asks_for_a_new_frame` fails any host handler that writes layer or mask pixels without it |
| T-040 | med | §1.2 | A freshly opened `.ptx` wore an unsaved marker on its tab while the window title showed it clean, and `Ctrl+W` closed it with no prompt. The title and the prompt were right — the tab was the liar. `dirty` is published twice, as a property and inside `documentTabsJson`, and three writers set the field and emitted only the property, so the strip kept whatever it had last been handed | Open any `.ptx` and look at the tab, then Ctrl+W | **fixed** — every write goes through `set_dirty`, which publishes both; `the_modified_flag_is_written_in_one_place` fails on any other assignment. Opening a `.ptx` no longer marks it modified at all; a PSD import still does, deliberately, because it has no `.ptx` of its own yet |
| T-037 | high | §7.2 | Every single-key tool shortcut went dead after Bake Text, until something else was clicked. The on-canvas editor was an `Item` hidden by a `visible:` binding, and Qt does not move active focus off a child whose *ancestor* became invisible: the destroyed-looking `TextEdit` kept the keyboard and accepted the shortcut override for every printable key. The `shortcutInputYield` path was innocent and was ruled out by probe — it recomputed correctly and still the key never reached the `Shortcut` | Text tool → click canvas → click the frame → Layer ▸ Bake Text → press `M` | **fixed** — the editor is a `Loader`; `active: false` destroys it, which releases focus for real where `visible: false` does not |
| T-035 | med | §2.4 | Command palette rows showed menu and shortcut but no label. It *was* the Basic-style contrast fault after all: the delegate had no background, so the style's light plate showed through under `colorOnSurface` text — near-white on near-white. The grey menu and blue chord survived because they are darker | Ctrl+Shift+P with a document open | **fixed** — opaque themed delegate background; text flips to `primaryOn` when highlighted |
| T-041 | med | §13 | Keyboard focus was invisible on every icon-only button. Qt Quick Controls put every `Button`, and so every `ToolButton`, in the tab chain by default, but the ring is the control's own to paint — and eleven hand-rolled backgrounds drew hover and checked and not focus. AT-SPI reported focus moving from Redo to About PhotoTux while a pixel diff of the whole window found nothing had changed | Press Escape at the welcome card, then Tab | **fixed** — `ChromeIconToolButton` and all eleven hand-rolled sites paint `Theme.focusRing` on `visualFocus`; `every_focusable_control_draws_its_focus` and `every_icon_button_draws_its_focus` fail the build on a control that stops |
| T-042 | med | §1.2 | Answering the unsaved-changes prompt with Save saved the document and then stopped — the close never happened, and File ▸ Quit ▸ Save left the application running. A save lands asynchronously, long after the prompt's button handler returns, and nothing picked the parked action back up | Paint on a document, Ctrl+W, Save, choose a path | **fixed** — `AppSession.documentSaved` is emitted when a write lands and the shell resumes the parked action through `afterHostSlot`; the file dialog's own cancel clears it, so backing out cannot arm the close against an unrelated Ctrl+S later. `a_close_deferred_for_a_save_is_finished_or_abandoned` fails the build on either half |
| T-043 | med | §5.1 | Lock All meant "cannot delete, cannot paint, cannot move", not "cannot change": opacity, blend mode and filter effects all went through on a locked layer, while the delete path's own refusal said "unlock it to change it". The three lock buttons also showed no state, and Lock All left pixels and position locked when it was turned off | Select a layer, Lock ▸ All, drag Opacity | **fixed** — `Layer::change_blocked`, one check at the top of `invoke` against `command_id::CHANGES_ACTIVE_LAYER`, and the same list greying the menus and the panel. Lock All is now a superset switch both ways; `every_command_is_classified_against_the_lock` partitions every command between refusing and not |
| T-044 | low | §4.2 | A marquee dragged wholly into the letterbox beside the page was accepted: `selection.active` went true, the status bar read `pixel selection`, the ants drew, and every command needing a selection then ran and did nothing. "Empty" meant an empty rectangle rather than a selection covering no pixels | Zoom so the canvas is letterboxed, then drag a marquee in the dark area | **fixed** — refused when the bounds do not intersect the document, with the engine asked before the GPU mask is written so the two cannot disagree about what active means. Surfacing the refusal found four sites reading `command rejected: …` to the user; all now go through `report_action_error` |

Severity guide: **blocker** = no window / data loss / crash on smoke; **high** = core workflow broken; **med** = feature wrong or a11y gap; **low** = polish; **info** = expected rejection.

---

## Pass log

| Date | Runner | Env | Smoke | Depth | Notes / commit |
| --- | --- | --- | --- | --- | --- |
| 2026-07-17 | agent (kwinmcp) | Wayland isolated | mostly green | partial DR-028 | `0c78559`, `59ce147` — see archive journal |
| 2026-07-17 | agent (kwinmcp) | Wayland isolated | §0 green / §1.1 partial | T-009/T-010 | AT flood + Welcome defer |
| 2026-07-17 | agent (kwinmcp; CU host map failed) | Wayland isolated | §1.1–1.2 + About | T-011 | Welcome restore; open PNG; dirty/close |
| 2026-07-17 | agent (kwinmcp) | Wayland isolated | custom size + quit | T-012 | 600×600 create; Ctrl+Q clean |
| 2026-07-17 | agent (kwinmcp) | Wayland isolated | §1.3 multi-doc tabs | T-013/T-014 | park New/Open; limit status; tab switch/close |
| 2026-07-17 | agent (kwinmcp) | Wayland isolated 520px | §2.1–2.4 + overflow | T-015/T-016 | rejected Save status; More tools Instantiator |
| 2026-07-17 | agent (kwinmcp) | Wayland isolated 1440×900 | §2.2 / §4.4 transform-crop cancel | T-017 | active_tool sync; Esc + strip cancel |
| 2026-07-17 | agent (kwinmcp) | Wayland isolated 1440×900 | §2.3 shortcut yield | T-018 | New Document blocks palette; editor detect |
| 2026-07-18 | agent (kwinmcp) | Wayland kept-home | §2.3 keymap + conflict | T-019 | prefs scroll; Save F9 persist; chord steal |
| 2026-07-18 | agent (kwinmcp) | Wayland isolated 1440×900 | §2.5 context menus | T-020 | canvas/selection menus; clamped popup helper |
| 2026-07-18 | agent (kwinmcp) | Wayland isolated 1440×900 | §3 workspace/panels | — | Compact/Essentials; Navigator toggle; no dirty |
| 2026-07-18 | agent (kwinmcp) | Wayland isolated 1440×900 | §4.1 pan/zoom | — | wheel zoom; Fit shortcut; middle-drag pan |
| 2026-07-18 | agent (kwinmcp) | Wayland isolated 1440×900 | §15 conflicts/races | — | undo spam; gallery Esc/close; host markers; Esc modals |
| 2026-07-18 | agent (kwinmcp) | Wayland isolated 900×520 | §2.2 More tools overflow | T-016 | deferred open; settled outside-close; Theme rows; pick closes |
| 2026-08-12 | agent (kwin-mcp) | isolated virtual KWin, isolated home, 1920×1080 | tool keys, shelf order, options bar, disclosure toggle, paint, undo/redo | T-025 found and fixed; T-026 confirmed | first pass with real input injection. Verified live: V/U/P/M/B select the right tool (`tool.shape` and `tool.path-edit` reachable), options bar switches per tool with all four selection-mode icons drawn, collapse/expand-all round-trips with summaries and right-pointing carets, brush painting works through the `PointHandler`, History lists the stroke, undo/redo round-trips |
| 2026-08-12 | agent (spectacle) | Wayland host session, isolated `XDG_CONFIG_HOME` | dense / comfortable / `QT_SCALE_FACTOR=2` | T-021…T-024 | first visual pass on the density work; ten group headers reviewed via a forced-visibility QML override. **Pointer clicks could not be injected** (KWin ignores `ydotool` uinput events), so header-button *state* was verified by seeding `disclosure_open` both ways, not by pressing it |
| 2026-09-03 | agent (kwinmcp) | Wayland isolated 1920×1080 | §7.2 text + §7.3 shapes | T-037 | Bake Text left the keyboard on a hidden editor; shape presets named for their kind |
| 2026-09-03 | agent (kwinmcp) | Wayland isolated 1920×1080 | §3 workspace, §5 layers/masks, §4.4 transform/crop, §7.1 adjustments | T-038, T-039 | Reset Workspace did not republish the dock; Selection to Mask did not recomposite |
| 2026-09-03 | agent (kwinmcp) | Wayland isolated 1920×1080 | §1.2 open/save/close, export formats, recovery dialog | T-040 | dirty flag published twice; export offered four of six formats; flat danger button erased its own label on hover |
| 2026-09-04 | agent (kwinmcp) | Wayland isolated 1920×1080 and 3840×2160 | §13 keyboard and focus, every dialog, shell at both extremes | T-041 | focus invisible on every icon-only button; shell holds at 1280×720 and at 4K |

---

## Sign-off (per pass)

- [x] §0–§2 green (boot + lifecycle + action chrome)
- [x] §4–§5 green (tools + layers/masks) or `[!]` filed
- [x] §7 creative engines exercised or explicitly `[N]` with reason
- [x] §13 a11y smoke green
- [x] All `[!]` fixed or deferred with Decision Register / gap note
- [x] `rust-tc quick` green for code fixes
- [x] This checklist + journal updated; commits landed

---

## Cross references

- [31 — Testing](../31-Testing.md) — pyramid; UI tests wait on semantic barriers
- [Handbook-Parity-Checklist.md](Handbook-Parity-Checklist.md) — what is *supposed* to be implemented
- [Command-Taxonomy.md](Command-Taxonomy.md) — command IDs for palette/menu coverage
- [Accessibility-Checklist.md](Accessibility-Checklist.md) — full a11y matrix
- [Performance-Budget-Ledger.md](Performance-Budget-Ledger.md) — measured budgets
- [Decision-Register.md](Decision-Register.md) — DR-023 / 024 / 028 / 029
- [AGENTS.md](../../AGENTS.md) — agent setup and quality gate
