# Development Checklist

Living tracker. **Phases 0–12** = foundation (closed). **Production readiness** below maps [`docs/FEATURES_TODO.md`](../FEATURES_TODO.md) — the feature bar for a production-ready desktop editor.

Legend: `[ ]` todo · `[~]` in progress / partial · `[x]` done · `[!]` blocked · `[P]` post-MVP (explicitly deferred)

**Product surface (ADR-014):** desktop GUI only — **no** CLI / TUI / web product. `cargo` = developer tooling.

**Governing docs:** ADRs `docs/01-decisions/`, `docs/DESIGN.md`, IA, `AGENTS.md`, `SPEC.md`, `docs/FEATURES_TODO.md`.

**Known ADR tension:** FEATURES_TODO lists multi-document / tabs; ADR-013 is **single document** until amended. Track multi-doc as `[!]` until ADR change.

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

## Production readiness (from FEATURES_TODO)

Work remaining to close the production feature bar. Prefer vertical slices; amend ADRs when scope conflicts.

### File Support

#### Native / open

- [x] PNG
- [x] JPG / JPEG
- [x] WebP
- [x] TIFF
- [x] BMP
- [x] GIF
- [x] Native `.ptx` (ADR-016)
- [~] PSD (subset import + compatibility report; ADR-018 — expand channels/layers/export)
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
- [~] PSD export (subset TBD)
- [ ] SVG export
- [ ] PDF export
- [ ] ICO export
- [ ] Animated GIF export
- [~] PNG / JPEG / WebP quality controls (defaults exist; UI polish)

---

### Workspace & UI

#### Panels

- [x] Layers
- [x] History
- [~] Properties (tool/doc chrome; expand)
- [~] Color (FG/BG state; full panel TBD)
- [ ] Brushes
- [ ] Characters / Fonts
- [ ] Paragraph
- [ ] Swatches
- [ ] Navigator
- [ ] Info
- [ ] Channels
- [ ] Paths

#### Interface

- [x] Dark theme (DESIGN.md / Breeze-dark)
- [x] Zoom (+ zoom-to-fit)
- [~] Guides (toggle + engine types)
- [ ] Fullscreen
- [!] Multiple documents (needs ADR-013 amendment)
- [!] Tabs (depends on multi-doc)
- [ ] Rulers
- [ ] Grid
- [ ] Snap
- [ ] Smart Guides

---

### Layers

#### Types

- [x] Raster layer
- [x] Text layer (basic)
- [x] Adjustment layer (types + add UI; full GPU eval TBD)
- [x] Group
- [~] Layer mask (metadata + add; paint/refine TBD)
- [ ] Shape layer
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
- [ ] Clipping mask
- [ ] Convert to Smart Object
- [ ] Rasterize

---

### Selections

#### Tools

- [x] Move (viewport / layer interaction baseline)
- [~] Marquee — Rectangular
- [ ] Marquee — Elliptical / Single Row / Single Column
- [ ] Lasso — Freehand / Polygonal / Magnetic
- [ ] Magic Wand
- [ ] Quick Selection
- [ ] Object Selection

#### Operations

- [x] Select All / Deselect
- [~] Add / Subtract / Intersect (engine `SelectionCombine`; UI incomplete)
- [x] Clipboard copy / paste-as-layer
- [ ] Feather / Expand / Contract / Border / Smooth
- [ ] Inverse / Grow / Similar / Reselect
- [ ] Marching-ants / GPU selection overlay polish

---

### Masks

- [~] Layer mask (create + graph field)
- [ ] Vector mask
- [ ] Clipping mask
- [ ] Disable / Apply / Delete mask
- [ ] Refine Mask

---

### Transformations

- [~] Crop (commit command)
- [~] Free Transform (engine types / preview; handle chrome TBD)
- [ ] Scale / Rotate / Skew / Distort / Perspective / Warp
- [ ] Flip Horizontal / Vertical
- [ ] Rotate Canvas
- [ ] Perspective Crop

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

---

### Vector Tools

- [ ] Pen / Free Pen / Curvature Pen
- [ ] Add / Delete / Convert anchor
- [ ] Path Selection / Direct Selection

---

### Text Tools

- [~] Horizontal text layer (basic create)
- [ ] Vertical / Paragraph text
- [ ] Character / Paragraph formatting panels
- [ ] Font selection / OpenType
- [ ] Kerning / Tracking / Leading / Baseline Shift
- [ ] Warp Text

---

### Shape Tools

- [ ] Rectangle / Rounded Rectangle / Ellipse / Polygon / Line / Custom Shape
- [ ] Stroke / Fill / Corner Radius / Dashed Stroke
- [ ] Align / Distribute shapes

---

### Color & Fill

- [x] Foreground / Background color + swap
- [~] Color picker / eyedropper (partial)
- [ ] Gradient tool (Linear / Radial / Angle / Reflected / Diamond)
- [ ] Paint Bucket
- [ ] Swatches panel

---

### Adjustments

Adjustment **layer kinds + params** exist; full nondestructive GPU evaluation per kind is remaining work.

- [~] Brightness / Contrast
- [~] Invert
- [ ] Levels / Curves / Exposure / Vibrance
- [ ] Hue / Saturation / Color Balance / Black & White
- [ ] Photo Filter / Channel Mixer / Color Lookup
- [ ] Posterize / Threshold / Gradient Map / Selective Color

---

### Filters

Filter **param / pass descriptors** exist; product filter menu + GPU implementations TBD.

#### Blur

- [ ] Average / Blur / Blur More / Box Blur
- [ ] Gaussian Blur / Motion / Radial / Surface Blur

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

Post-MVP per FEATURES_TODO — do not block core production path.

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

---

### Automation & Scripts

- [~] Keyboard shortcuts (core menus; incomplete vs Photoshop parity)
- [ ] Custom shortcuts
- [ ] Batch processing

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

#### Measurement

- [ ] Ruler Tool / Notes / Count Tool

#### Navigation

- [x] Hand (pan) / Zoom
- [ ] Rotate View Tool

#### Misc

- [x] Clipboard copy / paste (selection → layer)
- [~] Drag & drop images
- [!] Multiple image editing (ADR-013)
- [~] PSD compatibility (subset + report)
- [ ] Unlimited canvas (memory-dependent tiling)
- [~] Non-destructive workflow (graph v2 foundation; styles/SO/filters incomplete)
- [~] Photoshop-like shortcuts (partial)

---

## Suggested next slices (priority)

Order is guidance, not locked ADR:

1. **Selection polish** — overlay/marching ants, elliptical/lasso, combine modes in UI
2. **Transform chrome** — free-transform handles + quality resample; flip/rotate canvas
3. **Mask paint + clipping** — usable layer masks end-to-end
4. **PSD depth** — channel decompress, layered import fidelity, subset export
5. **Adjustment/filter GPU** — ship Brightness/Levels/Gaussian first
6. **Panels** — Swatches, Navigator, Properties completeness
7. **Vector / shapes / rich text** — after raster core feels solid
8. **Multi-doc** — only after ADR-013 amendment

---

## Standing rules (all work)

- [ ] Update this file when starting/finishing a production slice
- [ ] Keep `docs/FEATURES_TODO.md` and this checklist aligned (status lives here; wish-list source there)
- [ ] Log blockers in `blockers.md` immediately
- [ ] No major deps / surface changes without ADR
- [ ] UI changes respect `DESIGN.md` (extend tokens if needed)
- [ ] `./scripts/check-rust.sh` green when Rust workspace exists
- [ ] Fix code rather than paragraph-long workaround comments (`AGENTS.md`)
