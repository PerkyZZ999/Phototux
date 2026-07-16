# Information Architecture: PhotoTux Workspace

> Structural map of the PhotoTux desktop editor: navigation, content hierarchy, user flows, naming, and reuse.  
> Complements [DESIGN_BRIEF.md](./DESIGN_BRIEF.md), [DESIGN.md](./DESIGN.md), [SPEC.md](../SPEC.md), and ADRs.  
> Capability status: **current / planned / deferred**. Single-document and desktop-only navigation remain until their ADRs are amended.

## Application Map

```
PhotoTux (desktop window)
├── Menu bar
│   ├── File — New, Open, Save, Save As, Open Recent, Export, Recover…, Quit
│   ├── Edit — Undo, Redo, Cut, Copy, Paste, Paste as New Layer, Clear, Preferences…
│   ├── Image — Canvas Size, Image Size, Crop, Rotate Canvas…
│   ├── Layer — New, Duplicate, Delete, Group, Mask ops, Rasterize…
│   ├── Select — All, None, Invert, Modify (feather/grow/shrink), Load/Save Selection…
│   ├── Filter — Adjustments & filters (nondestructive when applicable)
│   ├── View — Zoom, Fit, Actual, Rulers, Guides, Grid, Snap
│   ├── Window — Show/hide docks, Reset Layout
│   └── Help — Shortcuts, About
├── Top tool bar — New/Open/Save, Undo/Redo, mode actions, optional search/command palette
├── Left tool strip — tools (paint, select, transform, fill, text, …)
├── Center — Canvas viewport (+ selection/transform overlays, optional rulers)
├── Right docks
│   ├── Properties / Tool Options (contextual)
│   ├── Layers (hierarchy, masks, effects)
│   └── Optional: History, Color/Swatches, Brush Presets
├── Status / HUD — zoom, tool, FPS, dirty, GPU, progress
└── Dialogs — Welcome/New, Export, Adjust/Filter, Preferences, Compatibility Report, Recovery, Alerts
```

| Entry | Destination | Notes |
|-------|-------------|-------|
| Desktop launcher / `cargo run -p phototux` | Main window | Single process; developer run is not a product CLI surface |
| File → New / Welcome | New Document dialog | Presets 720p / 1080p / 2K / 4K + custom; zoom-to-fit on create |
| File → Open | Native file dialog | Raster import + later `.ptx` / PSD subset |
| File → Save / Save As | Native `.ptx` | Atomic write; planned |
| File → Export | Export dialog | Flattened PNG/JPEG (current); WebP/TIFF planned |
| File → Recover… | Recovery UI | Crash journal / autosave; planned |
| Tool strip | Active tool + Tool Options schema | Exclusive tool mode |
| Layers dock | Active layer / mask / effect target | Hierarchy planned |
| History dock | Transaction list | Planned |
| Close / Quit | Unsaved changes dialog | Dirty check |

**Capability notes**

- **Current**: shell, GPU canvas, brush/eraser/pan/zoom, flat layers, stroke undo, PNG/JPEG I/O, QML AOT startup
- **Planned**: `.ptx`, selections, transforms, masks/groups, text, adjustments/filters, history, clipboard, PSD subset
- **Deferred**: multi-document tabs (ADR-013), KDE global menu, floating multi-monitor docks, photography/RAW/DAM

## Content Hierarchy

```
Document session (single document v1)
├── Document metadata — size, name, path, dirty, color/depth policy
├── History — transactional undo/redo (+ planned named snapshots)
├── Selection — GPU selection channel + boolean mode + feather metadata
├── Canvas viewport state — zoom, pan, fit mode, overlays (ants, transform, guides)
└── Layer graph (typed, hierarchical — planned graph v2)
    ├── RasterLayer — pixels, opacity, blend, transform, locks, label color
    ├── Group — children, pass-through / blend, visibility
    ├── TextLayer — content + typography + transform (rasterized/cached for composite)
    ├── AdjustmentLayer — typed params + optional mask
    ├── Layer mask (optional) — density, feather, linked, enabled
    └── Filter / effect stack (optional) — reorderable nondestructive nodes
```

**Presentation priority (on-screen)**

1. Canvas composite (always primary)
2. Active tool affordance (cursor, overlays, Tool Options)
3. Active layer / mask / effect target (Layers + Properties)
4. Selection / transform state
5. Document dirty / save / recovery status
6. Secondary docks (History, Color, Presets)
7. HUD metrics (FPS, composite time) — diagnostic, not content

## User Flows

### Flow: First run → paint (current)

1. Launch → main window (empty or last session policy TBD)
2. Welcome / File → New → choose preset → OK
3. Document created; zoom-to-fit; default RasterLayer active; Brush selected
4. Adjust size/opacity/hardness in Properties
5. Paint on canvas; stroke completes → one undo step
6. Continue editing or File → Export / Save (when `.ptx` available)

### Flow: Native Save / Recovery (planned)

1. Edit until dirty indicator shows
2. File → Save (or Save As if untitled)
3. Atomic `.ptx` write with progress for large docs
4. On crash: next launch or File → Recover offers journaled snapshot
5. Failed save never overwrites the previous good file

### Flow: Selection → transform (planned)

1. Choose Marquee / Ellipse / Lasso; set replace/add/subtract/intersect
2. Drag on canvas → selection ants appear
3. Choose Move / Free Transform
4. Manipulate handles / numeric fields; live preview
5. Enter/Apply → one undo transaction; Escape → cancel without mutation

### Flow: Selection → mask (planned)

1. Create selection
2. Layer → Add Mask from Selection (or Layers context menu)
3. Mask thumbnail appears; optional enter mask-edit mode
4. Paint on mask with black/white/gray; toggle enable/link/invert
5. Undo restores mask + selection policy per transaction design

### Flow: Nondestructive adjustment / filter (planned)

1. Select target layer or group
2. Layer → New Adjustment / Filter → choose type
3. Dialog or Properties show params with live preview
4. OK adds editable stack entry (hideable/reorderable)
5. Explicit “Apply destructively” flattens when user requests

### Flow: Text creation (planned)

1. Select Text tool; click or drag on canvas
2. Type content; Properties show font/size/color/alignment
3. Commit creates/updates TextLayer; remains editable until rasterize
4. Missing fonts show fallback + warning in Properties / status

### Flow: Clipboard transfer (planned)

1. Selection + Copy/Cut → one-shot CPU clipboard transfer at boundary
2. Paste → new pixels on active layer or Paste as New Layer
3. Undoable as one transaction

### Flow: PSD import with disclosure (planned)

1. File → Open → choose PSD
2. Importer maps supported subset into graph v2
3. Compatibility Report lists unsupported effects/features
4. User acknowledges; document opens dirty or clean per policy
5. Native Save writes `.ptx` as authoritative

### Flow: Export flattened (current → expand)

1. File → Export → format + quality/path
2. Worker encodes flattened composite; progress for large images
3. Success toast/status or error dialog

## Taxonomy & Naming Conventions

| Concept | Preferred term | Avoid |
|---------|----------------|-------|
| App window workspace | Workspace | IDE, dashboard, site |
| Image editing surface | Canvas | Stage, artboard (unless multi-artboard later) |
| Document file (native) | PhotoTux Document / `.ptx` | Project pack, PSD (unless interchange) |
| Pixel stack entry | Layer | Track, clip |
| Folder of layers | Group | Folder (UI may say Group) |
| Alpha mask on layer | Mask / Layer Mask | Stencil (unless tool name) |
| Pixel selection | Selection | Region (except API) |
| Nondestructive color op | Adjustment / Adjustment Layer | LUT panel, develop module |
| Nondestructive effect | Filter / Effect | Smart Object (Adobe-only jargon in UI) |
| Text content layer | Text Layer | Label, caption |
| Tool parameter strip | Tool Options / Properties | Inspector (OK in docs), sidebar only |
| Layer list | Layers | Timeline |
| Undo list | History | Log |
| Brush library | Brush Presets | Skins |
| Color pair | Foreground / Background | Primary / Secondary (OK in code) |
| Import warning UI | Compatibility Report | Error dump |
| View transform | Zoom / Pan | Scale gesture (ok in impl notes) |
| Composite result | Composite | Flattened preview (export context) |
| Input device | Stylus / tablet | Mouse-only assumptions |
| Engine message | Command | Request (unless HTTP) |
| Bridge object | `AppController` / document session | ViewModel soup |
| Present path | Zero-copy / shared texture | Texture upload (forbidden steady-state) |

**UI copy**: sentence case for controls; menu bar may follow platform/Plasma conventions. User-facing strings via `qsTr(...)`.

## Screen Inventory

| Screen / surface | Purpose | Primary actions | Status |
|------------------|---------|-----------------|--------|
| Main workspace | Edit document | Tools, layers, canvas, docks | Current |
| Welcome / New Document | Create sized document | Preset cards, OK/Cancel | Current |
| Export dialog | Flattened raster out | Path, format, quality | Current |
| About / Shortcuts | Version & keymap | Close | Current / Planned |
| Preferences | App + tablet + performance | Apply/OK | Planned |
| Save / Save As | Persist `.ptx` | Path, confirm overwrite | Planned |
| Recovery | Restore autosave/journal | Restore / Discard | Planned |
| Adjustment / Filter dialog | Parametric edit | Preview, OK, Cancel | Planned |
| Compatibility Report | Disclose import limits | Continue / Cancel | Planned |
| Unsaved changes | Block data loss | Save / Discard / Cancel | Current |
| Progress overlay | Long I/O or filter | Cancel when safe | Planned |
| Brush Presets panel | Pick/save presets | Search, import/export | Planned |
| Color / Swatches | FG/BG & entry | Sample, HEX/RGB/HSV | Planned |
| History panel | Navigate transactions | Click step, clear (policy) | Planned |

No marketing pages, settings websites, or mobile shells.

## Menu / Navigation Structure

| Region | Contents | Behavior |
|--------|----------|----------|
| Menu bar | File, Edit, Image, Layer, Select, Filter, View, Window, Help | Always available; disable items when no document |
| Tool strip | Brush, Eraser, Select tools, Move/Transform, Crop, Fill, Gradient, Eyedropper, Text, Pan, Zoom, … | Exclusive; one primary tool |
| Tool Options | Schema for active tool | Updates instantly on tool change |
| Layers | Hierarchy, visibility, locks, masks, effects | Keyboard navigable; drag reparent planned |
| Properties | Layer + mask + effect + text contexts | Complements Tool Options |
| History | Transactions | Optional dock |
| Color / Presets | Painting aids | Optional docks |
| Status bar | Zoom, tool name, FPS/ms, dirty, messages | Read-mostly; zoom control allowed |
| Context menus | Canvas, layer row, mask, effect | Planned expansion |

Depth target: ≤3 steps for paint, select, transform, mask, save, export.

## Design System Reuse Map

| Need | Reuse | New? |
|------|-------|------|
| Color, type, space, radius, elevation | `DESIGN.md` tokens → QML Theme | Extend tokens only when required |
| Buttons, sliders, checkboxes, combos | Qt QC2 + PhotoTux styling | Per-control styles |
| Tool strip button | Icon button pattern | Shared `ToolButton` |
| Dock panel chrome | Shared header + scroll body | `Panel` / `DockColumn` |
| Layer / mask / effect row | List delegate family | Hierarchical delegate planned |
| Dialogs | Shared modal scaffold | `AppDialog` |
| Canvas overlays | Selection ants, transform handles | New overlay items planned |
| Icons | Phosphor under `assets/icons/` + `ICON_MAP.md` | Add mapped assets only |
| Status chips | Compact label / HUD | Small components |
| Compatibility / recovery copy | Alert + report list pattern | Planned |
| Empty canvas vs open doc | Distinct empty state vs document chrome | Keep simple |

**Rule**: Prefer shared QML components over one-off rectangles. New visual values go into `DESIGN.md` first.

## Implementation Notes

- Shell lives in `phototux_ui` + `qml/`; canvas in `phototux_canvas` / GPU crates — do not put wgpu in the UI crate.
- IA changes that imply multi-document UI, photography modules, or non-desktop surfaces require ADR amendments first.
- Update this file and `DESIGN_BRIEF.md` when a feature adds a tool mode, dock, dialog, document state, or primary flow.
- Checklists: `docs/03-checklists/development.md`, `blockers.md`.
