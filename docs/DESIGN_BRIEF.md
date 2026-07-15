# Design Brief: PhotoTux Workspace

> Product-level design brief for the native Linux image editor shell and canvas experience.  
> Complements [SPEC.md](../SPEC.md), [CONSTRAINTS.md](../CONSTRAINTS.md), and locked ADRs in `docs/01-decisions/`.

## Problem

You are painting or compositing on Linux. The canvas feels a half-second behind your stylus. Panels waste space. Menus fight you. Tools that are powerful still feel like they were ported from another decade — heavy, chatty, never quite “desktop native” on Plasma. You want the density and confidence of a pro editor with the responsiveness of a game engine viewport, without leaving Wayland or KDE.

## Solution

PhotoTux is a **single, focused workspace**: a zero-latency GPU canvas at the center, wrapped in a **dense, dark, multi-pane shell** that behaves like a first-class KDE Plasma 6 application. Chromium chrome stays quiet; the image stays sharp at high refresh rates. Controls send light commands; pixels stay on the GPU. The experience prioritizes *staying in flow on the canvas* over wizard-style onboarding or web-app navigation patterns.

## Experience Principles

1. **Canvas first, chrome second** — Every layout choice maximizes uninterrupted image area and input fidelity. Chrome densifies; it never competes with the stroke path.
2. **Immediate feedback over decorative motion** — State changes (brush size, layer opacity, tool mode) update labels and the canvas without theatrical animation. Motion only clarifies structure (dock collapse, panel show/hide), never delays paint.
3. **Pro density with calm hierarchy** — High information per pixel (Breeze-inspired dark surfaces, tight spacing) without visual noise. One accent color for selection/focus; neutrals do the rest.

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

## Existing Patterns

Greenfield product (no production UI library in-repo yet). Authority sources:

| Layer | Source | Notes |
|-------|--------|-------|
| Product vision | `SPEC.md` | Phases 1–5, SLOs |
| Hard constraints | `CONSTRAINTS.md` | Linux/Wayland, Rust, Qt 6 QML, zero-copy |
| Stack / UI toolkit | ADR-002 | Qt 6 QML Controls 2, dense desktop |
| Bridge | ADR-003 | `qtbridge` for logic; hybrid canvas item allowed |
| Visual tokens | This brief + `DESIGN.md` | Normative tokens live in `DESIGN.md` |
| Structure | `INFORMATION_ARCHITECTURE.md` | Workspace regions & flows |

- **Typography**: System / Plasma fonts (Noto Sans or Breeze default) — no web font stack
- **Colors**: Dark charcoal surfaces + single Plasma-like accent (see `DESIGN.md`)
- **Spacing**: 4px base grid, dense 8/12/16 chrome rhythm
- **Components**: Qt Quick Controls 2 styled to PhotoTux tokens; custom canvas item separate

## Component Inventory

| Component | Status | Notes |
| --------- | --------------------- | -------- |
| Application shell (window, menu bar hooks) | New | Phase 1 shell; Phase 5 KDE global menu |
| Top tool bar | New | Document + tool-mode actions |
| Left tool strip | New | Primary tool exclusives (brush, select, transform…) |
| Center canvas viewport | New | GPU `QQuickItem` / RHI; placeholder until Phase 2 |
| Right properties inspector | New | Context-sensitive tool/layer props |
| Layers panel | New | List + visibility/lock; full interactivity Phase 4 |
| Status / HUD bar | New | Zoom, tool, FPS/latency debug |
| Collapsible docks | New | Show/hide without losing workspace model |
| Color / brush widgets | New | Phase 4; design tokens ready earlier |
| Dialogs (open/save/export) | New | XDG portals Phase 5 |
| Modal alerts / progress | New | Lightweight Qt dialogs |
| Keyboard shortcut overlay | New | Later; document shortcuts in IA |

## Key Interactions

| User action | Interface response |
|-------------|-------------------|
| Hover tool strip icon | Tooltip + status hint; no layout shift |
| Select tool | Exclusive selection highlight; properties inspector switches context |
| Drag brush size slider | Immediate value label + engine command; optional canvas cursor preview |
| Scroll/pinch or zoom control | Canvas zoom around cursor/center; status shows %; ≥60 FPS target |
| Space+drag / middle-drag | Pan canvas; cursor grab |
| Paint stroke (tablet) | Dabs → GPU path; UI thread not blocked (ADR-007) |
| Layer row click | Selection; properties show layer opacity/blend |
| Toggle layer visibility | Eye icon + composite update on GPU |
| Collapse dock | Animated width to handle strip; canvas expands |
| Undo/Redo | Transaction applied; layers/canvas refresh; no full reload |

## Responsive Behavior

**Desktop-first native app** (not a responsive website).

| Class | Behavior |
|-------|----------|
| ≥1440×900 (reference) | Full multi-pane: tool strip + canvas + docks |
| 1280×720 | Docks may auto-narrow; tool strip stays 48px |
| Narrow / single display small | User may undock/hide panels; canvas never below minimum usable size |
| Multi-monitor | Single main window initially; floating docks later (post-MVP) |
| HiDPI / fractional scale | Wayland scale-aware Qt; pixel-perfect canvas via GPU |
| Touch | Secondary; stylus/tablet primary. No redesign for pure finger-first |

## Accessibility Requirements

Native desktop accessibility (Qt / AT-SPI), not WCAG web checklists alone — but contrast still matters.

- **Contrast**: Text/icons on surfaces ≥ **4.5:1** for body; ≥ **3:1** for large UI chrome glyphs
- **Keyboard**: Full tool switching, menu, layer focus order; Escape cancels transient modes
- **Focus**: Visible focus ring on controls (accent outline); canvas focus separate from chrome
- **Screen reader**: Labels on tool buttons, layer rows, sliders (Accessible.name / description)
- **Motion**: Respect system reduce-motion → dock animations instant
- **Color**: Tool state never communicated by color alone (icon + selected state + tooltip)
- **Tablet**: Pressure optional enhancement; size/opacity still adjustable via UI

## Out of Scope

- Marketing website, account system, cloud sync
- **CLI product, TUI, headless batch tool** (desktop GUI only — ADR-014)
- Plugin marketplace UI, asset store
- Full vector studio, 3D viewport, video timeline
- Mobile / tablet-OS apps
- Light theme v1 (dark-only first; tokens may reserve light later)
- Onboarding tour / empty-state mascot
- Internationalization of all strings (architecture allows Qt `qsTr`; full locales later)
- Implementation of wgpu interop (engineering; not this brief)
## Success (design)

- User can open the app and understand **where to paint, which tool is active, and how to change brush size** within 10 seconds without docs
- Canvas region is visually and behaviorally dominant
- Chrome feels **native on Plasma**, not “cross-platform generic”
- Design tokens in `DESIGN.md` are sufficient for an agent to implement QML styles without inventing a second palette
