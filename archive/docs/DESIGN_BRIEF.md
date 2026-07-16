# Design Brief: PhotoTux Workspace

> Product-level design brief for the native Linux image editor shell and canvas experience.  
> Complements [SPEC.md](../SPEC.md), [CONSTRAINTS.md](../CONSTRAINTS.md), [INFORMATION_ARCHITECTURE.md](./INFORMATION_ARCHITECTURE.md), and locked ADRs in `docs/01-decisions/`.  
> Capability status uses **current / planned / deferred** (not stale Phase 1–5-only labels).

## Problem

You are painting or compositing on Linux. The canvas feels a half-second behind your stylus. Panels waste space. Menus fight you. Tools that are powerful still feel like they were ported from another decade — heavy, chatty, never quite “desktop native” on Plasma. You want the density and confidence of a pro editor with the responsiveness of a game engine viewport, without leaving Wayland or KDE.

## Solution

PhotoTux is a **single, focused workspace**: a zero-latency GPU canvas at the center, wrapped in a **dense, dark, multi-pane shell** that behaves like a first-class KDE Plasma 6 application. Chrome stays quiet; the image stays sharp at high refresh rates. Controls send light commands; pixels stay on the GPU. The product evolves from a GPU-painting MVP into a **professional raster creation and compositing editor**—native layered documents, selections, transforms, masks, text, adjustments, filters, and later layered PSD interchange—while staying desktop-native and photography-workflow-free.

## Experience Principles

1. **Canvas first, chrome second** — Every layout choice maximizes uninterrupted image area and input fidelity. Chrome densifies; it never competes with the stroke path.
2. **Immediate feedback over decorative motion** — State changes (brush size, layer opacity, tool mode) update labels and the canvas without theatrical animation. Motion only clarifies structure (dock collapse, panel show/hide), never delays paint.
3. **Pro density with calm hierarchy** — High information per pixel (Breeze-inspired dark surfaces, tight spacing) without visual noise. One accent color for selection/focus; neutrals do the rest.
4. **Nondestructive by default** — Adjustments, filters, masks, and text remain editable until the user explicitly applies or rasterizes. Destructive actions require clear commit or confirmation when data loss is possible.
5. **Preview / commit / cancel** — Transient modes (transform, crop, filter dialogs) show live preview; Escape or Cancel restores prior state; Apply/Enter commits one undoable transaction.
6. **Selection and mask visibility** — Active selection (marching ants) and mask-target mode are always readable without hiding document content; never communicate target state by color alone.
7. **Contextual tool options** — The Properties / Tool Options surface switches schema with the active tool; advanced sections collapse by default.
8. **Long-operation calm** — Save, filters, PSD import, and large I/O show progress, remain cancellable where safe, and never freeze the shell or steal canvas focus without recovery.
9. **Large-document calm** — Hierarchy, virtualized lists, and quiet status messaging keep hundreds of layers and long histories usable without chrome noise.

## Aesthetic Direction

- **Philosophy**: **Industrial creative tool** — KDE Plasma 6 / Breeze Dark meets modern GPU-first editors (Affinity / Krita density, not Adobe skeuomorphism). Matte panels, precise edges, minimal glow.
- **Tone**: Focused, professional, calm under pressure. Confident, not playful. Technical without terminal-cosplay.
- **Reference points**:
  - KDE Plasma 6 Breeze Dark (colors, control chrome, window integration)
  - Krita / GIMP multi-dock workspaces (information density, layer stack mental model)
  - Affinity Photo / modern desktop editors (clean tool strips, property inspectors)
  - High-end GPU viewports (stable pan/zoom, HUD metrics when needed)
- **Anti-references**:
  - Electron / VS Code “web app in a frame” padding and flat cards
  - GNOME/libadwaita large touch-first spacing and titlecase mobile patterns
  - Neon cyberpunk dashboards, glassmorphism, marketing landing-page gradients
  - Skeuomorphic brushes, textured faux-leather panels
  - Photography DAM / Lightroom-style catalogs and develop modules

## Existing Patterns

| Layer | Source | Notes |
|-------|--------|-------|
| Product vision | `SPEC.md` | Architecture pillars + SLOs |
| Hard constraints | `CONSTRAINTS.md` | Linux/Wayland, Rust, Qt 6 QML, zero-copy, desktop GUI only |
| Stack / UI toolkit | ADR-002 | Qt 6 QML Controls 2, dense desktop |
| Bridge | ADR-003 | `qtbridge` for logic; hybrid canvas item allowed |
| Visual tokens | `DESIGN.md` | Normative colors, type, spacing, components |
| Structure | `INFORMATION_ARCHITECTURE.md` | Workspace regions, flows, naming |
| Roadmap | IA parity slices (`development.md`) | Code = Current; IA = production target; ADRs gate Blocked items |

- **Typography**: System / Plasma fonts (Noto Sans or Breeze default) — no web font stack
- **Colors**: Dark charcoal surfaces + single Plasma-like accent (see `DESIGN.md`)
- **Spacing**: 4px base grid, dense 8/12/16 chrome rhythm
- **Components**: Qt Quick Controls 2 styled to PhotoTux tokens; custom canvas item separate

## Capability status legend

| Status | Meaning |
|--------|---------|
| **Current** | Shipped in the desktop app today (codebase truth) |
| **Planned** | Required for IA parity / production-ready ([INFORMATION_ARCHITECTURE.md](./INFORMATION_ARCHITECTURE.md)) |
| **Blocked** | Desired by IA; needs ADR amendment before ship |
| **Deferred** / `[P]` | Explicit later track |

Roadmap slices: [03-checklists/development.md](./03-checklists/development.md).

## Component Inventory

| Component | Status | Notes |
| --------- | --------------------- | -------- |
| Application shell (window, menu bar) | Current → Planned | Core menus Current; full IA menus Planned |
| Welcome / New Document | Current → Planned | Presets Current; Recent/Templates/Prefs entry Planned |
| Top tool bar | Current | Document actions |
| Left tool strip | Current → Planned | Brush/eraser/select/transform/fill/gradient/text/eyedropper/hand/zoom Current set; retouch/paths/shapes Planned |
| Tool Options Bar | Planned | Dedicated strip; Properties dock is Current fallback |
| Tool Options / Properties | Current → Planned | Brush + blend + FG Current; full per-tool schemas Planned |
| Center canvas viewport | Current | Zero-copy GPU composite |
| Selection overlay | Current | Marching ants + combine modes |
| Transform / crop overlay | Current | Handles, Apply/Cancel |
| Rulers / guides / grid | Planned | Guides toggle partial; rulers/grid/snap Planned |
| Right Properties dock | Current → Planned | Tool + layer contexts Current; text/styles Planned |
| Layers panel (hierarchical) | Current → Planned | Groups, masks, clip, effects Current; thumbs/lock polish Planned |
| History panel | Current | Named transactions via HistoryService |
| Brush Presets panel | Planned | Engine JSON presets exist; dedicated dock Planned |
| Color / Swatches | Current | FG/BG, HEX, recent, eyedropper |
| Navigator | Current | Geometric viewport (no GPU thumb) |
| Character / Paragraph | Planned | After text bake |
| Channels / Paths | Planned | Paths need vector engine |
| Workspace Manager | Planned | Presets + Reset; Window menu |
| Document tabs | Blocked | ADR-013 single-doc |
| Adjustment / Filter surfaces | Current → Planned | Brightness/Levels + Gaussian Current; dialog suites Planned |
| Status / HUD bar | Current | Zoom, tool, FPS/composite; dirty/GPU hints |
| Save / Save As | Current | Native `.ptx` atomic |
| Recovery UX | Planned | Autosave APIs exist; launch Recover polish Planned |
| Export dialogs | Current → Planned | Flattened rasters + PSD subset Current; quality UI Planned |
| Compatibility report | Current | PSD/import unsupported-feature disclosure |
| Progress / cancel surfaces | Planned | Long filters, I/O polish |
| Keyboard shortcut overlay | Planned | Help → Shortcuts |
| Preferences | Planned | Tablet, performance, appearance, file handling |
| Context menus | Planned | Layer / Canvas / Selection first |
| Print | Planned | File → Print |
| Modal alerts | Current | Unsaved, I/O errors, About |

## Key Interactions

| User action | Interface response |
|-------------|-------------------|
| Hover tool strip icon | Tooltip + status hint; no layout shift |
| Select tool | Exclusive selection highlight; Properties / Tool Options switches schema |
| Drag brush size slider | Immediate value label + engine command; canvas cursor preview when painting |
| Scroll/pinch or zoom control | Canvas zoom around cursor/center; status shows %; ≥60 FPS target |
| Space+drag / middle-drag | Pan canvas; cursor grab |
| Paint stroke (tablet) | Dabs → GPU path; UI thread not blocked (ADR-007) |
| Select rect / ellipse / lasso | Selection channel updates; marching ants; boolean mode in Tool Options |
| Modify selection (add/subtract/intersect) | Live ants update; status shows mode |
| Free Transform drag | Live preview warp; Escape cancels; Enter/Apply commits one undo step |
| Crop drag | Crop overlay; commit resizes document; cancel restores |
| Layer row click | Active layer; Properties show opacity/blend/mask |
| Drag layer / reparent into group | Hierarchy updates; composite refresh; one undo step |
| Toggle layer visibility / lock | Icon state + GPU composite; paint blocked when locked |
| Create mask from selection | Mask thumbnail appears; mask-target mode optional |
| Edit mask vs layer | Explicit target toggle in Layers/Properties; never ambiguous |
| Reorder / hide filter effect | Stack updates live; nondestructive until “Apply” |
| Eyedropper click | Samples current or all layers per option; FG color updates |
| Text tool click-drag | Creates Text Layer; inline edit; Properties show typography |
| Save / Save As | Atomic `.ptx` write; progress for large docs; failed save keeps prior file |
| Open PSD (subset) | Compatibility report lists unsupported items; no silent loss |
| Long filter / import | Progress + Cancel where safe; canvas remains navigable when possible |
| Undo / Redo | Single transaction; layers/canvas/selection restore; no full reload |
| Collapse dock | Animated width to handle strip (instant if reduce-motion); canvas expands |

## Responsive Behavior

**Desktop-first native app** (not a responsive website).

| Class | Behavior |
|-------|----------|
| ≥1440×900 (reference) | Full multi-pane: tool strip + canvas + docks (+ optional History/Color) |
| 1280×720 | Docks may auto-narrow; tool strip stays 48px; secondary docks collapse to icons |
| Narrow / single display small | User may undock/hide panels; canvas never below minimum usable size |
| Multi-monitor | Single main window initially; floating docks later (deferred) |
| HiDPI / fractional scale | Wayland scale-aware Qt; pixel-perfect canvas via GPU |
| Touch | Secondary; stylus/tablet primary. No redesign for pure finger-first |

## Accessibility Requirements

Native desktop accessibility (Qt / AT-SPI), not WCAG web checklists alone — but contrast still matters.

- **Contrast**: Text/icons on surfaces ≥ **4.5:1** for body; ≥ **3:1** for large UI chrome glyphs
- **Keyboard**: Full tool switching, menu, layer focus order, selection ops, transform commit/cancel; Escape cancels transient modes
- **Focus**: Visible focus ring on controls (accent outline); canvas focus separate from chrome
- **Screen reader**: Labels on tool buttons, layer/mask/effect rows, sliders, dialogs (Accessible.name / description)
- **Motion**: Respect system reduce-motion → dock/transform preview animations instant or static
- **Color**: Tool/selection/mask state never by color alone (icon + selected state + tooltip/label)
- **Tablet**: Pressure optional enhancement; size/opacity still adjustable via UI
- **Progress**: Long ops announce busy state; Cancel reachable by keyboard

## Out of Scope

- Marketing website, account system, cloud sync
- **CLI product, TUI, headless batch tool** (desktop GUI only — ADR-014)
- Plugin marketplace / script store UI until a dedicated ADR (`[P]` in checklist)
- Full vector illustration studio, 3D viewport, video timeline / animation
- Photography workflows: RAW develop, camera/lens correction, catalogs/DAM, tethering, panorama/HDR merge
- Generative AI features (`[P]`)
- Mobile / tablet-OS apps
- Light theme v1 (dark-only first; tokens may reserve light later)
- Multi-document tabs until ADR-013 single-doc decision is amended
- Shape layers until ADR-017 kind amendment (Paths may ship earlier)
- Full lossless PSD parity (documented subset + compatibility report only)
- Onboarding tour / empty-state mascot theater
- Internationalization of all locales (architecture allows Qt `qsTr`; full locales later)

## Success (design)

- User can open the app and understand **where to paint, which tool is active, and how to change brush size** within 10 seconds without docs
- Canvas region is visually and behaviorally dominant
- Chrome feels **native on Plasma**, not “cross-platform generic”
- Design tokens in `DESIGN.md` are sufficient for an agent to implement QML styles without inventing a second palette
- Every planned professional-raster feature has a documented workspace location, primary flow, cancel/undo behavior, and accessibility expectation in this brief + IA before QML lands
- Nondestructive stacks (masks, adjustments, filters, text) remain discoverable in Layers without jargon overload
