# Development Checklist

Living tracker. **Phases 0–12** = foundation (closed). **Production readiness** = IA parity with [`docs/INFORMATION_ARCHITECTURE.md`](../INFORMATION_ARCHITECTURE.md) (merged from [`PREFERED_IA.md`](../PREFERED_IA.md)). Wish-list inventory: [`docs/FEATURES_TODO.md`](../FEATURES_TODO.md).

Legend: `[ ]` todo · `[~]` in progress / partial · `[x]` done · `[!]` blocked · `[P]` post-MVP (explicitly deferred)

**Product surface (ADR-014):** desktop GUI only — **no** CLI / TUI / web product. `cargo` = developer tooling.

**Source of truth:** **codebase** = what is built · **IA + this checklist** = production-ready destination · **ADRs** = hard gates.

**Governing docs:** ADRs `docs/01-decisions/`, `docs/DESIGN.md`, IA, `AGENTS.md`, `SPEC.md`, `docs/FEATURES_TODO.md`.

**Known ADR tensions** (see [`docs/04-journal/conflicts.md`](../04-journal/conflicts.md)):

- Multi-document / tabs (IA) vs ADR-013 single document → `[!]` until amend.
- Shape layers (IA) vs ADR-017 kind set → `[!]` until amend.
- Plugins / script store (IA) → new ADR before product surface.

---

## Foundation — Phases 0–12 (closed)

| Phase | Goal | Status |
|-------|------|--------|
| 0 | Docs & decisions | `[x]` |
| 1 | Desktop shell bootstrap | `[x]` 2026-07-15 |
| 1.5 | Interop spike (ADR-010) | `[x]` 2026-07-15 |
| 2 | GPU viewport (zero-copy) | `[x]` 2026-07-15 |
| 3 | Layer / composite engine | `[x]` 2026-07-15 (10×4K &lt; 2 ms) |
| 4 | Tools & brush | `[x]` 2026-07-15 |
| 5 | Desktop integration | `[x]` 2026-07-15 (cold boot gate met) |
| 6 | Graph v2, `.ptx`, history, Save | `[x]` foundation 2026-07-16 |
| 7 | Selections, clipboard, transforms | `[x]` slice 2026-07-16 |
| 8 | Groups, masks, blends, Layers UI | `[x]` slice 2026-07-16 |
| 9 | Color / text / brush presets | `[x]` slice 2026-07-16 |
| 10 | Adjustments & filters contracts | `[x]` slice 2026-07-16 |
| 11 | History dock, guides, cancel, a11y | `[x]` slice 2026-07-16 |
| 12 | Rasters + PSD subset + report | `[x]` slice 2026-07-16 |

Pre-commit gate: rustfmt + clippy (`./scripts/check-rust.sh`). Full rust-doctor: `CHECK_RUST_FULL=1`.

---

## Production readiness (IA parity)

Work remaining to close the production feature bar. Prefer vertical slices; amend ADRs when scope conflicts. Status below reflects **codebase truth** (2026-07-16).

### Shell chrome (IA workspace)

- [x] Menu bar (core File/Edit/Image/Layer/Select/Filter/View/Help)
- [ ] Full menu IA parity (Open Recent, Print, Document Properties, Window, Preferences…)
- [x] Top toolbar (document actions)
- [~] Tool Options Bar (today: Properties dock; dedicated strip Planned)
- [x] Left tool strip (partial tool set)
- [x] Canvas (zero-copy GPU)
- [x] Status bar
- [ ] Workspace Manager (Essentials / Painting / Design / Minimal / Custom + Reset)
- [!] Document tabs (ADR-013)
- [ ] Welcome: Recent Files + Templates + Preferences entry

### File Support

#### Native / open

- [x] PNG
- [x] JPG / JPEG
- [x] WebP
- [x] TIFF
- [x] BMP
- [x] GIF
- [x] Native `.ptx` (ADR-016)
- [x] PSD (RGB8 Raw/RLE composite + layered import; subset export; ADR-018)
- [ ] PSB
- [ ] XD
- [ ] Sketch
- [ ] PDF
- [ ] AI
- [ ] SVG
- [ ] RAW
- [ ] ICO
- [ ] HEIC
- [ ] DDS
- [ ] AVIF
- [ ] OpenEXR
- [ ] TGA
- [ ] PPM
- [ ] HDR

#### Export / save

- [x] Save / Save As (`.ptx`)
- [x] Export PNG / JPEG
- [x] Export WebP / TIFF / BMP / GIF
- [x] PSD export (RGB8 Raw subset; File → Export)
- [ ] SVG export
- [ ] PDF export
- [ ] ICO export
- [ ] Animated GIF export
- [~] PNG / JPEG / WebP quality controls (defaults exist; UI polish)
- [ ] Print

---

### Workspace & UI

#### Panels

- [x] Layers
- [x] History
- [x] Properties (tool chrome, blend, FG RGB, Fit/100%)
- [x] Color (FG/BG + HEX + recent via Swatches)
- [ ] Brushes (dedicated panel; engine presets exist)
- [ ] Characters / Fonts
- [ ] Paragraph
- [x] Swatches
- [x] Navigator (geometric viewport; no GPU thumb)
- [ ] Info
- [ ] Channels
- [ ] Paths
- [ ] Layer Styles panel
- [ ] Adjustments panel (dedicated; menus + Properties today)
- [ ] Histogram
- [P] Actions / Tool Presets

#### Interface

- [x] Dark theme (DESIGN.md / Breeze-dark)
- [x] Zoom (+ zoom-to-fit)
- [~] Guides (toggle + engine types)
- [ ] Fullscreen / screen modes
- [!] Multiple documents (needs ADR-013 amendment)
- [!] Tabs (depends on multi-doc)
- [ ] Rulers
- [ ] Grid
- [ ] Snap
- [ ] Smart Guides
- [ ] Preferences dialog (General, Interface, Performance, Cursors, Tools, File Handling, Themes)

---

### Layers

#### Types

- [x] Raster layer
- [x] Text layer (metadata create; glyph bake TBD)
- [x] Adjustment layer (Brightness/Levels GPU; other kinds contracts)
- [x] Group
- [x] Layer mask (R8 paint + composite + `.ptx`)
- [!] Shape layer (needs ADR-017 kind amendment)
- [ ] Fill layer
- [ ] Smart Object
- [ ] Background layer (as distinct kind)
- [ ] Vector mask

#### Operations

- [x] Duplicate / delete / rename (core graph ops)
- [x] Hide / opacity / blend
- [x] Group / ungroup (group create + hierarchy)
- [~] Lock (lock flags present; UI completeness TBD)
- [ ] Fill (layer fill %)
- [ ] Merge / Merge Visible / Flatten
- [x] Clipping mask (`clips_to_below` in composite)
- [ ] Convert to Smart Object
- [ ] Rasterize (incl. text bake)

---

### Selections

#### Tools

- [x] Move (viewport / layer interaction baseline)
- [x] Marquee — Rectangular (GPU mask + ants + combine)
- [~] Marquee — Elliptical (ellipse marquee done; Single Row / Column TBD)
- [x] Lasso — Freehand / Polygonal (GPU edge ants); Magnetic TBD
- [ ] Magic Wand
- [ ] Quick Selection
- [ ] Object Selection

#### Operations

- [x] Select All / Deselect
- [x] Add / Subtract / Intersect (UI toggles + Shift/Alt modifiers; GPU mask)
- [x] Invert Selection
- [x] Clipboard copy / paste-as-layer
- [ ] Feather / Expand / Contract / Border / Smooth
- [ ] Grow / Similar / Reselect
- [x] Marching-ants overlay (GPU edge ants for freehand/polygonal; QML bounds for rect/ellipse)
- [x] Selection undo/redo restores mask + outline
- [ ] Color Range

---

### Masks

- [x] Layer mask (R8 GPU channel, paint target, composite multiply, `.ptx` round-trip)
- [ ] Vector mask
- [x] Clipping mask (`clips_to_below` in composite)
- [x] Disable / Delete mask (Apply / bake TBD)
- [ ] Refine Mask

---

### Transformations

- [x] Crop (pixel crop + Apply/Cancel overlay; undo restores size/pixels)
- [x] Free Transform (handles, live GPU affine preview, bake on commit)
- [~] Scale / Rotate (via free transform; Skew / Distort / Perspective / Warp TBD)
- [x] Flip Horizontal / Vertical (Image menu)
- [x] Rotate Canvas 90° CW (Image menu)
- [ ] Perspective Crop
- [ ] Image Size / Canvas Size dialogs

---

### Brushes & Painting

#### Tools

- [x] Brush
- [x] Eraser
- [ ] Pencil
- [ ] Color Replacement
- [ ] Mixer Brush
- [ ] Clone Stamp / Pattern Stamp
- [ ] History Brush
- [ ] Background Eraser / Magic Eraser

#### Features

- [x] Size / Hardness / Opacity / Color
- [x] Pressure support (tablet path)
- [x] Brush presets (JSON library)
- [ ] Flow / Smoothing / Spacing polish
- [ ] Dedicated Brushes panel UI

---

### Vector Tools / Paths

- [ ] Pen / Free Pen / Curvature Pen
- [ ] Add / Delete / Convert anchor
- [ ] Path Selection / Direct Selection
- [ ] Paths panel
- [ ] Stroke / Fill path

---

### Text Tools

- [~] Horizontal text layer (basic create; no glyph bake)
- [ ] Vertical / Paragraph text
- [ ] Character / Paragraph formatting panels
- [ ] Font selection / OpenType
- [ ] Kerning / Tracking / Leading / Baseline Shift
- [ ] Warp Text
- [ ] Rasterize / bake text to pixels

---

### Shape Tools

- [!] Rectangle / Rounded Rectangle / Ellipse / Polygon / Line / Custom Shape (ADR-017)
- [ ] Stroke / Fill / Corner Radius / Dashed Stroke
- [ ] Align / Distribute shapes

---

### Color & Fill

- [x] Foreground / Background color + swap
- [x] Color picker / eyedropper (click sample → FG)
- [~] Gradient tool (Linear shipped; Radial / Angle / Reflected / Diamond deferred)
- [x] Paint Bucket (layer/selection fill; no flood tolerance yet)
- [x] Swatches panel (defaults + recent; FG/BG swap)
- [ ] Full Color Picker dialog (HSV/RGB/HEX chrome)
- [ ] Flood fill tolerance / contiguous options

---

### Adjustments

Brightness/Contrast and Levels evaluate on GPU; other kinds remain contracts / deferred.

- [x] Brightness / Contrast
- [~] Invert
- [~] Levels / Curves / Exposure / Vibrance (Levels GPU shipped; Curves+ deferred)
- [ ] Hue / Saturation / Color Balance / Black & White
- [ ] Photo Filter / Channel Mixer / Color Lookup
- [ ] Posterize / Threshold / Gradient Map / Selective Color

---

### Filters

Gaussian Blur ships as nondestructive layer effect; other filters deferred.

#### Blur

- [ ] Average / Blur / Blur More / Box Blur
- [~] Gaussian Blur / Motion / Radial / Surface Blur (Gaussian shipped)

#### Sharpen

- [ ] Sharpen / Sharpen More / Smart Sharpen / Unsharp Mask

#### Noise / Pixelate / Distort / Stylize / Render / Other

- [ ] Noise suite (Add / Dust & Scratches / Median / Reduce)
- [ ] Pixelate suite (Mosaic / Crystallize / Facet / Fragment / Pointillize)
- [ ] Distort suite (Displace … ZigZag)
- [ ] Stylize suite (Emboss / Find Edges / Oil Paint / Solarize / Wind)
- [ ] Render suite (Clouds / Difference Clouds / Lens Flare)
- [ ] Other (High Pass / Maximum / Minimum / Offset)

---

### Layer Styles

- [ ] Drop Shadow / Inner Shadow
- [ ] Outer Glow / Inner Glow
- [ ] Bevel & Emboss / Satin
- [ ] Color / Gradient / Pattern Overlay
- [ ] Stroke (style)

---

### Blending Modes

Engine + WGSL cover a **core Photoshop-like set**; remaining modes listed as gaps.

- [x] Normal
- [x] Multiply / Screen / Overlay
- [x] Darken / Lighten
- [x] Color Dodge / Color Burn
- [x] Hard Light / Soft Light
- [x] Difference / Exclusion
- [x] Hue / Saturation / Color / Luminosity
- [x] Pass Through (groups)
- [ ] Dissolve
- [ ] Linear Burn / Darker Color
- [ ] Linear Dodge / Lighter Color
- [ ] Vivid Light / Linear Light / Pin Light / Hard Mix
- [ ] Subtract / Divide

---

### Smart Objects

- [ ] Embedded Smart Objects
- [ ] Linked Smart Objects
- [ ] Non-destructive editing via SO
- [ ] Smart Filters
- [ ] Replace Contents

---

### AI Features `[P]`

Post-MVP — do not block core production path.

- [P] Remove Background
- [P] Magic Replace / AI Fill / AI Expand
- [P] AI Image Generation
- [P] AI Selection Improvements

---

### History

- [x] Undo / Redo
- [x] History panel
- [x] Multiple history states (`HistoryService`)
- [ ] Tile / delta undo for large documents (perf)

---

### Guides & Layout

- [~] Guides (visibility + types)
- [ ] Smart Guides
- [ ] Grid
- [ ] Snap
- [ ] Align / Distribute
- [ ] New Guide Layout
- [ ] Artboards `[P]` / Deferred

---

### Automation & Scripts

- [~] Keyboard shortcuts (core menus; incomplete vs preferred IA)
- [ ] Custom shortcuts
- [ ] Batch processing
- [P] Actions panel
- [P] Scripts / Plugin Manager (needs ADR)

---

### Context menus

- [ ] Layer row context menu
- [ ] Canvas context menu
- [ ] Selection context menu
- [ ] History context menu
- [ ] Remaining IA targets (guide, path, text, swatch, tab…)

---

### Export (product UX)

- [x] Save / Save As
- [x] Export As (flattened rasters)
- [ ] Export As dialog with format-specific optimization
- [ ] SVG / PDF / animated GIF product paths

---

### Miscellaneous

#### Retouching

- [ ] Spot Healing / Healing Brush / Patch / Red Eye / Content-Aware Move
- [ ] Dodge / Burn / Sponge / Blur / Sharpen / Smudge tools

#### Measurement

- [ ] Ruler Tool / Notes / Count Tool

#### Navigation

- [x] Hand (pan) / Zoom
- [ ] Rotate View Tool

#### Misc

- [x] Clipboard copy / paste (selection → layer)
- [~] Drag & drop images
- [!] Multiple image editing (ADR-013)
- [x] PSD compatibility (subset import/export + report)
- [ ] Unlimited canvas (memory-dependent tiling)
- [~] Non-destructive workflow (graph v2 + Brightness/Levels/Gaussian; styles/SO incomplete)
- [~] Photoshop-like shortcuts (partial)
- [~] Recovery / autosave UX polish

---

## Suggested next slices (IA parity roadmap)

Order is guidance, not locked ADR. Prefer vertical slices that land chrome + engine + checklist + journal.

### A — Shell & document UX (no ADR amend)

1. **Preferences dialog** — General / Interface / Performance / Cursors / Tools / File Handling / Themes; wire from File/Edit + Welcome.
2. **Menu IA parity** — Open Recent, Document Properties, View fullscreen modes, Window → Panels / Workspaces / Reset; Help shortcuts polish.
3. **Tool Options Bar** — dedicated contextual strip above canvas (Properties remains dock fallback).
4. **Workspace Manager** — presets (Essentials, Painting, Design, Minimal) + Custom + Reset Workspace.
5. **Welcome polish** — Recent Files, Templates entry, Preferences.
6. **Context menus v1** — Layer row + Canvas + Selection.
7. **Export / Print polish** — format quality UI; Print dialog stub → real print path.
8. **Recovery UX** — surface autosave recover on launch / File menu.

### B — Editing depth (engine + UI)

9. **Text bake + Character/Paragraph** — rasterize TextLayer; font/size/color; Character + Paragraph panels.
10. **Selection modify** — Feather / Expand / Contract / Smooth / Reselect; Color Range.
11. **Magic Wand + Quick Selection** — contiguous / tolerance; refine path.
12. **Image / Canvas Size** — dialogs + Image menu; Trim.
13. **Layer ops** — Merge / Flatten / Rasterize; Apply mask; lock UI completeness.
14. **Fill layer + flood polish** — fill %; Paint Bucket tolerance / contiguous.
15. **Gradient suite** — Radial / Angle / Reflected / Diamond.
16. **Brush panel + Pencil** — Brushes dock; Pencil tool; flow/smoothing polish.
17. **Adjustment GPU wave 2** — Hue/Sat, Curves, Invert full eval; Adjustments panel.
18. **Filter suites (priority)** — Unsharp Mask / High Pass / Box Blur / Noise Reduce (one suite per slice).
19. **Layer Styles v1** — Drop Shadow + Stroke (+ overlay); Styles panel.
20. **Guides / Grid / Rulers / Snap** — View menu + canvas overlays; Smart Guides later.
21. **Channels panel** — RGB + alpha visibility (edit later if needed).
22. **Retouch v1** — Clone Stamp + Healing or Dodge/Burn (pick one vertical).
23. **Magnetic Lasso** — edge-aware; reuse GPU ants.

### C — Vectors (ADR gate)

24. **Paths engine + Paths panel** — Pen / Direct Selection; stroke/fill path onto raster (no Shape kind yet).
25. **ADR-017 Shape kind amendment** — then Shape tools + Shape layers.
26. **Shape tool chrome** — Rect/Ellipse/Polygon/Line; Align/Distribute.

### D — Interchange & color

27. **SVG / PDF open or export** (pick one direction per slice; honest subset + report).
28. **Color management foundation** — document profile metadata; soft-proof later (ADR if ICC becomes major).
29. **PSB / deeper PSD** — only if ADR-018 amended for scope.

### E — Blocked / deferred (do not start without gate)

30. **[!]** Multi-doc tabs — after ADR-013 amendment.
31. **[P]** Actions / Batch / Scripts / Plugin Manager — after new ADR.
32. **[P]** AI features — post-MVP.
33. **[P]** Artboards / unlimited canvas tiling — after core IA chrome.

### Already shipped (reference)

- Selection polish, Transform chrome, Mask paint + clipping, Lasso + GPU ants
- PSD depth, Adjustment/filter GPU (Brightness/Levels + Gaussian)
- Panels (Swatches, Navigator, blend/FG Properties)
- Color & Fill (Paint Bucket, Linear Gradient, Eyedropper)

---

## Standing rules (all work)

- [ ] Update this file when starting/finishing a production slice
- [ ] Keep IA status tags honest vs codebase; wish-list detail stays in `FEATURES_TODO.md`
- [ ] Log blockers in `blockers.md` immediately
- [ ] No major deps / surface changes without ADR
- [ ] UI changes respect `DESIGN.md` + IA (extend tokens if needed)
- [ ] `./scripts/check-rust.sh` green when Rust workspace exists
- [ ] Fix code rather than paragraph-long workaround comments (`AGENTS.md`)
- [ ] Commit locally on slice complete; do not push unless asked
