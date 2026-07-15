# Information Architecture: PhotoTux

> Structural skeleton for the PhotoTux **desktop workspace** (not a multi-page website).  
> Builds on [DESIGN_BRIEF.md](./DESIGN_BRIEF.md). Implementation-facing terms align with ADRs and `SPEC.md`.

## Site Map (Application Map)

PhotoTux is a **document-centric single-window app**. “Routes” are **workspace modes and transient surfaces**, not URLs.

```
PhotoTux Application
├── Main Workspace                          [always-on primary surface]
│   ├── Menu Bar / Global Menu hooks        [Phase 5: KDE integration]
│   ├── Top Tool Bar                        [document + common actions]
│   ├── Left Tool Strip                     [exclusive tool modes]
│   ├── Center Canvas Viewport              [document view; 80% time-on-task]
│   ├── Right Dock Column
│   │   ├── Properties Inspector            [context: tool | layer | selection]
│   │   └── Layers Panel                    [stack; visibility; selection]
│   ├── Optional Bottom / Floating
│   │   ├── Timeline / Animation            [out of MVP]
│   │   └── Brush Presets drawer            [Phase 4+]
│   └── Status / HUD Bar                    [zoom, tool, metrics]
│
├── Document Lifecycle Surfaces
│   ├── New Document                        [dialog]
│   ├── Open Image / Project                [XDG portal — Phase 5]
│   ├── Save / Save As / Export             [XDG portal — Phase 5]
│   └── Close / Unsaved changes             [confirm dialog]
│
├── Preferences                             [dialog; later]
│   ├── Input (tablet curves)
│   ├── Performance / GPU
│   └── Appearance (theme tokens later)
│
├── Help
│   ├── Keyboard Shortcuts                  [overlay or dialog]
│   └── About                               [dialog]
│
└── Debug / Dev (non-user)
    └── FPS / latency / GPU HUD toggles     [dev builds; ADR-008]
```

**MVP focus (Phases 1–2):** Main Workspace chrome + Canvas Viewport (placeholder → GPU).  
**Later phases** attach Layers depth, tools, portals, menus without changing the core map.

## Navigation Model

This is **spatial workspace navigation**, not hierarchical site nav.

| Layer | What | Rules |
|-------|------|-------|
| **Primary** | Main Workspace always visible | No “home page.” Opening the app *is* the workspace. |
| **Secondary** | Docks (Properties, Layers) | Collapsible; order fixed in v1; user rearrange later |
| **Tool navigation** | Left tool strip + shortcuts | Exclusive selection (radio group). Max ~8–12 tools visible; overflow menu later |
| **Utility** | Menu bar, preferences, help, portals | Document lifecycle and system integration |
| **Transient** | Dialogs, confirmations, shortcut overlay | Modal only when data loss risk |
| **Mobile** | N/A for v1 | Desktop Linux only (ADR-001) |

**Depth limit:** ≤2 levels of chrome (window → dock panel). No nested “apps inside apps.”

**80% time surface:** **Center Canvas Viewport.** IA and layout protect this region first.

## Content Hierarchy

### Main Workspace (default)

1. **Canvas / image content** — Why first: core job is create/edit pixels  
2. **Active tool + immediate parameters** (tool strip + key properties) — Why second: continuous control while painting  
3. **Layer stack state** — Why third: non-destructive structure (grows Phase 3–4)  
4. **Document meta / status** (filename, zoom %, FPS) — Below fold of attention; always available in status bar  
5. **Menus and secondary tools** — On demand  

### Properties Inspector

1. Context title (Tool name or Layer name)  
2. Primary parameter (e.g. brush size, layer opacity)  
3. Secondary parameters (hardness, blend mode…)  
4. Advanced / collapsed sections  

### Layers Panel

1. Active layer highlight  
2. Visibility / lock affordances  
3. Layer name + thumbnail (thumbnail later)  
4. Blend mode / opacity summary  
5. Layer group structure (Phase 3+)  

### Open / Export dialogs (system)

Content owned by **XDG portals** — PhotoTux supplies filters and last paths only.

## User Flows

### F1 — Cold start to first stroke (MVP path)

1. User launches PhotoTux from the **desktop** (app menu / icon) — **GUI only** (ADR-014; no CLI/TUI product)  
2. **New Document** flow (ADR-013): user picks size via dialog or presets (**720p / 1080p / 2K / 4K**), then workspace opens; **zoom-to-fit**  
3. Brush tool pre-selected (when tools exist; Phase 1 may show shell only)  
4. User adjusts brush size in Properties (or shortcuts) when painting lands  
5. User strokes on canvas → paint appears (Phase 4 full; Phase 2 may be pan/zoom only)  
6. Status bar reflects tool + zoom  

### F2 — Zoom and pan while inspecting

1. User focuses canvas  
2. Scroll / zoom control → zoom around cursor; status updates  
3. Space+drag or middle-drag → pan  
4. Frame rate stays ≥60 (Phase 2 gate)  
5. Properties/layers remain interactive without hitching chrome  

### F3 — Layer-aware edit (Phase 3–4)

1. User selects layer in Layers panel  
2. Properties show layer opacity/blend  
3. User paints → only active layer mutates  
4. User toggles visibility → composite updates on GPU  
5. Undo → graph/transaction reverts; selection preserved if possible  

### F4 — Open existing image (Phase 5)

1. File → Open (or Ctrl+O)  
2. XDG portal file picker  
3. Decode → document graph + GPU textures  
4. Canvas frames image; zoom-to-fit optional  
5. If unsaved changes on previous doc → confirm  

### F5 — Export (Phase 5)

1. File → Export  
2. Portal + format options (PNG/JPEG… minimal set)  
3. Progress if large; cancelible  
4. Status: “Exported path”  

### F6 — Tool switch mid-work

1. User presses `V` / clicks transform (example)  
2. Tool strip selection moves  
3. Properties inspector swaps schema  
4. Canvas cursors/overlays change  
5. Brush dynamics idle until brush re-selected  

## Naming Conventions

| Concept | Label in UI | Notes |
|---------|-------------|-------|
| Application | PhotoTux | Product name |
| Open document | Document | Not “Project” in v1 (single-doc) |
| Pixel stack item | Layer | Industry standard |
| Non-destructive graph | (internal) Image State Graph | Not exposed as jargon in UI |
| Drawing surface | Canvas | Viewport shows the canvas |
| Tool mode | Tool | Brush, Eraser, Select, Transform, Eyedropper… |
| Right panel parameters | Properties | Not “Inspector” in chrome title (Properties is clearer) |
| Layer list | Layers | Panel title |
| Left exclusive tools | Tools | Tool strip / toolbox |
| Top bar | Toolbar | Document + frequent actions |
| Bottom bar | Status | Includes optional HUD metrics |
| Blend operation | Blend mode | Multiply, Overlay… |
| History step | Undo / Redo | Not “History panel” in MVP |
| Stylus input | Tablet | Wayland tablet |
| File open/save OS UI | (system) | Never rebrand portal chrome |
| Performance overlay | HUD | Dev/debug; “Performance” in prefs later |

**Glossary rule:** One term per concept. Prefer **Layer**, **Canvas**, **Tool**, **Properties**, **Document**.

## Component Reuse Map

| Structural component | Used on | Behavior differences |
|---------------------|---------|----------------------|
| `ApplicationWindow` shell | Always | Theme tokens from DESIGN.md |
| Top Toolbar | Main workspace | Actions enable/disable by document state |
| ToolStrip | Main workspace | Tool set grows by phase |
| CanvasViewport | Main workspace | Placeholder → GPU item; same slot in layout |
| DockColumn | Right side | Hosts Properties + Layers; collapsible |
| PropertiesForm | Properties dock | Schema switches by tool/layer context |
| LayerList | Layers dock | Model-backed list (qtbridge model later) |
| StatusBar | Always | FPS/HUD only when enabled |
| DialogShell | New/Open/Save/Prefs/About | Shared margins/buttons |
| ConfirmDialog | Close unsaved, destructive | Danger emphasis on confirm |

## Content Growth Plan

| Area | Growth over time | IA accommodation |
|------|------------------|------------------|
| Tools | Many tools/plugins later | Tool strip + overflow; categories; search later |
| Layers | Hundreds on large docs | Virtualized list; filter; groups |
| Brush presets | Large libraries | Drawer/panel with search; not all in Properties |
| Documents | Multi-doc tabs later | Optional tab bar above canvas; out of MVP |
| History | Long undo stacks | Command list panel optional; memory caps |
| Preferences | Many keys | Grouped pages; search |
| Locale strings | i18n | All user-visible strings via `qsTr` |

## URL Strategy

**Not applicable** as HTTP routes. Desktop equivalents:

| Concern | Rule |
|---------|------|
| Deep link | Optional later: `phototux:` URI or `phototux /path/to/file` CLI |
| CLI | `phototux [file]` opens document in Main Workspace |
| Session restore | Window geometry + dock visibility + last paths (Phase 5+) |
| Query-like state | Not in URLs; in-session: active tool, selection, zoom — **not** serialized to path |

### State that is *not* navigation

Zoom level, pan offset, active layer id, tool id — **view state**, restored with session or document as appropriate, not separate “pages.”

## Workspace Layout Blueprint (structure only)

```
┌─────────────────────────────────────────────────────────────────────────┐
│ Menu (optional global) │  Toolbar (File, Edit, …, tool actions)        │
├────┬──────────────────────────────────────────────────────┬─────────────┤
│ T  │                                                      │ Properties  │
│ o  │                                                      │ (context)   │
│ o  │              Canvas Viewport                         ├─────────────┤
│ l  │              (document pixels)                       │ Layers      │
│ s  │                                                      │             │
├────┴──────────────────────────────────────────────────────┴─────────────┤
│ Status: document · tool · zoom · [HUD: FPS / latency]                   │
└─────────────────────────────────────────────────────────────────────────┘
```

**Reference sizes (from design tokens / brief):** tool strip ~48px; right dock ~280px default; window ≥1280×720 usable.

## Priority ranking (user jobs)

| Rank | Job to be done | Primary surface |
|------|----------------|-----------------|
| 1 | Paint / edit pixels with low latency | Canvas + Tool |
| 2 | Navigate canvas (zoom/pan) | Canvas |
| 3 | Adjust tool parameters | Properties |
| 4 | Manage layer stack | Layers |
| 5 | Open/save/export | Utility / portals |
| 6 | Configure app | Preferences |

## Alignment with roadmap

| Phase | IA unlock |
|-------|-----------|
| 1 | Main Workspace chrome; Properties bind to Rust state; canvas placeholder |
| 2 | Canvas becomes true document viewport (GPU) |
| 3 | Layers + graph semantics behind Layers panel |
| 4 | Full tool set + tablet flows |
| 5 | Portals, menus, session — outer IA ring |

## Out of IA scope

- Marketing site IA  
- Multi-user / cloud spaces  
- Plugin store taxonomy  
- Mobile navigation patterns  
