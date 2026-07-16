# Information Architecture: PhotoTux Workspace

> Structural map of the PhotoTux desktop editor: navigation, content hierarchy, user flows, naming, and reuse.  
> Complements [DESIGN_BRIEF.md](./DESIGN_BRIEF.md), [DESIGN.md](./DESIGN.md), [SPEC.md](../SPEC.md), and ADRs.  
> Target shell and feature set derived from [PREFERED_IA.md](./PREFERED_IA.md) (vendor-neutral pro-raster IA).  
> **Capability status:** **Current** = shipped in code today · **Planned** = production-ready target · **Blocked** = needs ADR amendment · **Deferred** / **`[P]`** = post-MVP or explicit later track.  
> **Source of truth:** the **codebase** for what is built; this IA + [development.md](./03-checklists/development.md) for the production-ready destination.

---

## Capability legend

| Status | Meaning |
|--------|---------|
| **Current** | Present in the running desktop app / crates today |
| **Planned** | Required for IA parity / production-ready; no ADR conflict |
| **Blocked** | Desired by IA but forbidden or gated until ADR amendment |
| **Deferred** | Explicitly later (large subsystem or `[P]` post-MVP) |

---

## Application Map

```
PhotoTux (desktop window)                          [Current shell]
├── Welcome Screen                                 [Current: New + Open; Planned: Recent, Templates, Prefs entry]
│   ├── New File / Open File
│   ├── Recent Files                               [Planned]
│   ├── Templates                                  [Planned]
│   └── Preferences                                [Planned]
├── Workspace
│   ├── Menu Bar                                   [Current; expand to full IA menus]
│   ├── Tool Options Bar                           [Current: Properties dock; Planned: dedicated options strip]
│   ├── Toolbar (document actions)                 [Current top bar]
│   ├── Document Tabs                              [Blocked: ADR-013 single-doc]
│   ├── Left Tool Strip                            [Current + Planned tools]
│   ├── Canvas                                     [Current: zero-copy GPU]
│   ├── Side Panels                                [Current + Planned set]
│   ├── Status Bar                                 [Current]
│   └── Workspace Manager                          [Planned: presets + reset; Window menu]
├── Documents — New/Open/Save/Save As/Export/Close [Current; Print Planned]
├── Editing System — layers, masks, adjustments… [Mixed: see modules]
├── Export — raster + PSD subset                   [Current]; SVG/PDF [Planned]
├── Automation — Actions / Batch / Scripts / Plugins [Deferred / [P]]
└── Preferences — General…Themes                   [Planned]
```

### Target main workspace (from preferred IA)

```
+--------------------------------------------------------------+
| Menu Bar                                                     |
+--------------------------------------------------------------+
| Tool Options Bar                                             |  [Planned chrome; today: Properties]
+--------------------------------------------------------------+
| Tool   |           Document Canvas         | Panels          |
| Strip  |                                   | Layers          |
|        |                                   | History         |
|        |                                   | Properties      |
|        |                                   | Color/Swatches  |
|        |                                   | Brushes         |  [Planned]
|        |                                   | Navigator       |
+--------------------------------------------------------------+
| Status Bar                                                   |
+--------------------------------------------------------------+
```

| Entry | Destination | Status |
|-------|-------------|--------|
| Desktop launcher / `cargo run -p phototux` | Main window | Current (dev run ≠ product CLI) |
| Welcome / File → New | New Document dialog | Current (presets + custom) |
| File → Open | Native dialog | Current (raster, `.ptx`, PSD subset) |
| File → Save / Save As | `.ptx` atomic | Current |
| File → Export | Flattened raster + PSD subset | Current |
| File → Print | Print dialog | Planned |
| Document tabs | Multi-doc session | **Blocked** (ADR-013) |
| Workspace presets | Essentials / Painting / … | Planned |
| Tool strip | Exclusive tool mode | Current |
| Layers / History / Swatches / Navigator | Docks | Current |
| Brushes / Character / Paths / Channels | Docks | Planned |
| Close / Quit | Unsaved changes | Current |

---

## Menu Architecture (target)

Status marks the **menu surface**, not every submenu item.

| Menu | Target contents (preferred IA) | Status |
|------|--------------------------------|--------|
| **File** | New, Open, Open Recent, Save, Save As, Export, Print, Document Properties, Exit | Current core; Recent/Print/Doc Props Planned |
| **Edit** | Undo, Redo, Cut, Copy, Paste, Fill, Stroke, Free Transform, Preferences | Current undo/clipboard/transform; Fill/Stroke/Prefs Planned |
| **Image** | Image Size, Canvas Size, Rotate, Flip, Crop, Trim, Duplicate, Color Mode, Bit Depth, Adjustments | Partial Current (crop/flip/rotate); rest Planned |
| **Layer** | New, Duplicate, Delete, Group, Merge, Flatten, Mask, Clipping, Arrange, Styles, Smart Layer | Partial Current; Merge/Flatten/Styles/SO Planned/Blocked |
| **Select** | All, Deselect, Reselect, Inverse, Modify, Feather, Expand, Contract, Color Range | Partial Current; Modify/Color Range Planned |
| **Filter** | Blur, Sharpen, Noise, Distort, Stylize, Pixelate, Render, Other | Partial (Gaussian + adjust kinds); suites Planned |
| **View** | Zoom, Guides, Grid, Rulers, Snap, Screen Mode, Fullscreen | Partial Current; Grid/Rulers/Snap/Fullscreen Planned |
| **Window** | Panels, Workspaces, Reset Workspace | Planned |
| **Help** | Documentation, Shortcuts, About | Partial Current |

---

## Toolbar / Tool Strip (target)

| Family | Tools | Status |
|--------|-------|--------|
| **Selection** | Move, Marquee, Lasso, Polygon Lasso, Magnetic Lasso, Magic Wand, Quick Selection | Move/Marquee/Lasso/Polygon Current; Magnetic/Wand/Quick Planned |
| **Crop** | Crop, Perspective Crop, Slice | Crop Current; Perspective/Slice Planned |
| **Retouch** | Spot Healing, Healing, Clone, Patch, Blur, Sharpen, Smudge, Dodge, Burn, Sponge | Planned |
| **Paint** | Brush, Pencil, Color Replacement, Mixer, Paint Bucket, Gradient | Brush/Eraser/Bucket/Linear Gradient Current; rest Planned |
| **Drawing** | Pen, Freeform Pen, Path/Direct Selection | Planned (needs Path engine; Shape kind → ADR-017) |
| **Text** | Horizontal, Vertical, Text Mask | Horizontal create Current (no bake); Vertical/Mask Planned |
| **Shapes** | Rect, Round Rect, Ellipse, Polygon, Line, Custom | Planned (**Blocked** until Shape kind ADR) |
| **Navigation** | Hand, Rotate View, Zoom | Hand/Zoom Current; Rotate View Planned |
| **Colors** | FG / BG / Swap / Defaults | Current (Swatches + Properties) |

---

## Panels (target)

| Group | Panels | Status |
|-------|--------|--------|
| Document | Layers, Channels, Paths, History | Layers/History Current; Channels/Paths Planned |
| Properties | Properties, Layer Styles, Adjustments | Properties Current; Styles/Adjustments panel Planned |
| Painting | Brushes, Brush Settings, Patterns, Gradients | Brushes presets exist in engine; dedicated panel Planned |
| Color | Color Picker, Swatches | Swatches Current; full picker Planned |
| Typography | Character, Paragraph, Glyphs | Planned (after text bake) |
| Information | Navigator, Histogram, Info | Navigator Current (geometric); Histogram/Info Planned |
| Automation | Actions, Tool Presets | Deferred / `[P]` |

---

## Document / Layer Architecture (target)

```
Document session
├── Canvas (+ optional Artboards)          [Artboards Deferred]
├── Layer Groups
│   ├── Pixel / Raster Layers              [Current]
│   ├── Text Layers                        [Current metadata; bake Planned]
│   ├── Shape Layers                       [Blocked: ADR-017 kind]
│   ├── Fill Layers                        [Planned]
│   └── Adjustment Layers                  [Current Brightness/Levels GPU]
├── Channels                               [Planned]
├── Paths                                  [Planned]
└── History                                [Current]
```

**Layer fields (target):** name, visible, locked, opacity, blend, mask, clipping, effects (styles), metadata — most Current for raster; styles Planned.

---

## Functional Modules

| Module | Purpose | Status |
|--------|---------|--------|
| Document Manager | Open/save/export | Current |
| Workspace Manager | Layout presets | Planned |
| Tool System | Editing tools | Partial Current |
| Layer System | Nondestructive stack | Partial Current |
| History Engine | Undo/redo | Current |
| Brush Engine | Painting | Current |
| Selection Engine | Pixel selection | Current (core) |
| Path Engine | Bézier paths | Planned |
| Text Engine | Typography | Planned (bake + panels) |
| Filter Engine | Image processing | Partial Current |
| Color Engine | Color management / ICC | Planned (depth/ICC ADR later) |
| Plugin Manager | Extensions | Deferred / `[P]` |
| Automation | Actions & scripts | Deferred / `[P]` |
| Preferences | User configuration | Planned |

---

## Content Hierarchy (as implemented → target)

```
Document session (single document v1 — ADR-013)     [Current]
├── Document metadata — size, name, path, dirty
├── History — HistoryService + stroke/transform/selection stacks
├── Selection — GPU R8 mask + combine + undo
├── Canvas viewport — zoom, pan, overlays, guides types
└── Layer graph (ADR-017)
    ├── RasterLayer — pixels on GPU, opacity, blend, transform, locks, mask, clip, effects
    ├── Group — children, pass-through
    ├── TextLayer — TextContent metadata (glyph bake Planned)
    ├── AdjustmentLayer — Brightness/Levels GPU; other kinds Planned
    ├── Layer mask — R8 paint + composite
    └── FilterEffect stack — Gaussian Blur Current; styles Planned
```

**Presentation priority**

1. Canvas composite  
2. Active tool affordance  
3. Active layer / mask / effect target  
4. Selection / transform state  
5. Dirty / save / recovery  
6. Secondary docks  
7. HUD metrics  

---

## User Flows

### Flow: First run → paint — **Current**

1. Launch → Welcome / File → New → preset → zoom-to-fit  
2. Brush + Properties → paint → stroke undo  

### Flow: Native Save / Recovery — **Current Save**; Recovery **Planned UI polish**

1. Dirty → Save / Save As → atomic `.ptx`  
2. Autosave/recovery APIs exist; richer Recover UX Planned  

### Flow: Selection → transform / mask — **Current**

1. Marquee/Lasso → ants → Free Transform / Crop / Mask paint / Clip  

### Flow: Fill / Gradient / Eyedropper — **Current**

1. Paint Bucket / Linear Gradient / Eyedropper on active raster (selection-aware)  

### Flow: Nondestructive adjustment / filter — **Partial Current**

1. Layer → New Adjustment (Brightness/Levels) or Filter → Gaussian Blur  
2. Properties sliders; undo via graph/transform history  
3. Full Filter menu suites + dialog chrome Planned  

### Flow: Text creation — **Partial Current**

1. Text tool → creates TextLayer (no glyph bake yet)  
2. Planned: bake + Character/Paragraph panels  

### Flow: PSD import — **Current**

1. Open PSD → subset map → Compatibility Report → prefer Save as `.ptx`  

### Flow: Multi-document tabs — **Blocked**

Requires ADR-013 amendment before implementation.

### Flow: Path / Shape edit — **Planned / Blocked**

Paths Planned; Shape layers need ADR-017 kind amendment.

---

## Context Menus (target)

Dynamic by hit-target (preferred IA). **Planned** expansion; today mostly menu-bar / dock actions.

Targets: Canvas, Layer, Group, Multi-select, Selection, Guide, Ruler, Path, Shape, Text, Brush, Gradient, Swatch, History, Panel, Tab, Document, Empty workspace.

Priority ship order for context menus: Layer row → Canvas → Selection → History.

---

## Taxonomy & Naming Conventions

| Concept | Preferred term | Avoid |
|---------|----------------|-------|
| App window workspace | Workspace | IDE, dashboard |
| Image editing surface | Canvas | Stage (unless artboard later) |
| Native document | PhotoTux Document / `.ptx` | Project pack |
| Pixel stack entry | Layer | Track, clip |
| Folder of layers | Group | Folder (UI may say Group) |
| Alpha mask | Layer Mask | Stencil |
| Pixel selection | Selection | Region (except API) |
| Nondestructive color op | Adjustment Layer | Develop module |
| Nondestructive effect | Filter / Effect | Smart Object (Adobe-only UI jargon) |
| Vector outline | Path | Curve tool (unless tool name) |
| Vector filled object | Shape Layer | Shape object (OK in docs) |
| Text content layer | Text Layer | Label |
| Tool parameter surface | Tool Options / Properties | Inspector-only |
| Layer list | Layers | Timeline |
| Undo list | History | Log |
| Brush library | Brush Presets | Skins |
| Color pair | Foreground / Background | Primary / Secondary (OK in code) |
| Import warning | Compatibility Report | Error dump |
| Present path | Zero-copy / shared texture | Steady-state CPU upload |

**UI copy:** sentence case; `qsTr(...)`.

---

## Screen Inventory

| Surface | Purpose | Status |
|---------|---------|--------|
| Main workspace | Edit document | Current |
| Welcome / New Document | Create sized document | Current |
| Export dialog | Flattened + PSD subset | Current |
| Compatibility Report | Import disclosure | Current |
| Unsaved changes | Block data loss | Current |
| About / Shortcuts | Version & keymap | Current / Planned polish |
| Preferences | App + tablet + performance | Planned |
| Recovery | Restore autosave | Planned UX |
| Adjustment / Filter dialog | Parametric edit | Planned (live Properties today) |
| Progress overlay | Long I/O | Planned polish |
| Brush Presets panel | Pick/save presets | Planned |
| Character / Paragraph | Typography | Planned |
| Channels / Paths | Document channels & vectors | Planned |
| Workspace Manager | Layout presets | Planned |
| Print dialog | Print | Planned |
| Document tabs | Multi-doc | **Blocked** (ADR-013) |

No marketing pages, web shells, or mobile products (ADR-014).

---

## Design System Reuse Map

| Need | Reuse | New? |
|------|-------|------|
| Tokens | `DESIGN.md` → `Theme.qml` | Extend when required |
| Controls | Qt QC2 + PhotoTux styling | Per-control |
| Tool strip / docks | Shared patterns in `Main.qml` | Extract components as density grows |
| Icons | Phosphor + `ICON_MAP.md` | Map before inventing |
| Overlays | Selection ants, transform, gradient preview | Extend carefully |

**Rule:** Shared QML over one-offs; new visual values → `DESIGN.md` first.

---

## ADR / IA tensions (must not ship silently)

| Preferred IA item | Conflict | Gate |
|-------------------|----------|------|
| Document tabs / multi-doc | ADR-013 single document | Amend ADR-013 |
| Shape layers as first-class kind | ADR-017 kind set | Amend ADR-017 |
| Plugins / script store as product | New major subsystem | New ADR |
| Cloud / non-desktop surfaces | ADR-001 / 014 | Forbidden for v1 |
| Steady-state CPU canvas thumbnails | ADR-005 | Geometric nav OK; thumbs only downscaled/debug |

Log amendments in [04-journal/conflicts.md](./04-journal/conflicts.md).

---

## Implementation Notes

- Shell: `phototux_ui` + `qml/`; canvas/GPU: `phototux_canvas` / `phototux_gpu`; pure graph: `phototux_engine`.  
- Living production checklist: [03-checklists/development.md](./03-checklists/development.md) (slices track IA parity).  
- Wish-list inventory: [FEATURES_TODO.md](./FEATURES_TODO.md) — status lives in the checklist.  
- Aspirational source kept for reference: [PREFERED_IA.md](./PREFERED_IA.md) (merged into this file 2026-07-16).  
- Update this file when adding a tool mode, dock, dialog, document state, or primary flow.
