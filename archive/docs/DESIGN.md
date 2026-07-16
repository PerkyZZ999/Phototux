---
version: alpha
name: PhotoTux
description: >
  Dense dark KDE Plasma–aligned design system for PhotoTux, a GPU-first
  Linux image editor. Tokens target Qt Quick Controls 2 / QML chrome;
  the canvas viewport is a separate GPU surface and stays visually quiet.
colors:
  primary: "#3DAEE9"
  secondary: "#A0A0A8"
  tertiary: "#F67400"
  neutral: "#1E1E22"
  surface: "#2B2B30"
  surface-raised: "#323238"
  surface-sunken: "#121214"
  surface-overlay: "#232328"
  border: "#3D3D45"
  border-subtle: "#2F2F36"
  on-primary: "#0A1620"
  on-surface: "#EFF0F1"
  on-surface-muted: "#A0A0A8"
  on-surface-disabled: "#9A9AA3"
  focus-ring: "#3DAEE9"
  success: "#2ECC71"
  warning: "#FF9F1A"
  error: "#DA4453"
  selection: "#3DAEE933"
  canvas-letterbox: "#0C0C0E"
  tool-active-bg: "#3DAEE940"
  icon-selected-tint: "#3DAEE9"
typography:
  headline-md:
    fontFamily: Noto Sans
    fontSize: 16px
    fontWeight: 600
    lineHeight: 1.25
    letterSpacing: 0em
  headline-sm:
    fontFamily: Noto Sans
    fontSize: 13px
    fontWeight: 600
    lineHeight: 1.3
    letterSpacing: 0em
  body-md:
    fontFamily: Noto Sans
    fontSize: 12px
    fontWeight: 400
    lineHeight: 1.4
    letterSpacing: 0em
  body-sm:
    fontFamily: Noto Sans
    fontSize: 11px
    fontWeight: 400
    lineHeight: 1.35
    letterSpacing: 0em
  label-md:
    fontFamily: Noto Sans
    fontSize: 11px
    fontWeight: 500
    lineHeight: 1.2
    letterSpacing: 0.01em
  label-sm:
    fontFamily: Noto Sans
    fontSize: 10px
    fontWeight: 500
    lineHeight: 1.2
    letterSpacing: 0.02em
  mono-hud:
    fontFamily: Noto Sans Mono
    fontSize: 11px
    fontWeight: 400
    lineHeight: 1.2
    letterSpacing: 0em
  window-title:
    fontFamily: Noto Sans
    fontSize: 13px
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: 0em
rounded:
  none: 0px
  xs: 2px
  sm: 4px
  md: 6px
  lg: 8px
  full: 9999px
spacing:
  xxs: 2px
  xs: 4px
  sm: 8px
  md: 12px
  lg: 16px
  xl: 24px
  xxl: 32px
  tool-strip-width: 48px
  dock-width: 280px
  toolbar-height: 40px
  statusbar-height: 28px
  panel-padding: 12px
  control-gap: 8px
components:
  button-primary:
    backgroundColor: "{colors.primary}"
    textColor: "{colors.on-primary}"
    rounded: "{rounded.sm}"
    padding: 8px
    height: 28px
    typography: "{typography.label-md}"
  button-primary-hover:
    backgroundColor: "#5CB8ED"
    textColor: "{colors.on-primary}"
  button-secondary:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.sm}"
    padding: 8px
    height: 28px
    typography: "{typography.label-md}"
  button-secondary-hover:
    backgroundColor: "#3A3A42"
    textColor: "{colors.on-surface}"
  button-ghost:
    backgroundColor: "#00000000"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.sm}"
    padding: 6px
    height: 28px
  button-ghost-hover:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
  tool-button:
    backgroundColor: "#00000000"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.sm}"
    size: 36px
    padding: 4px
  tool-button-active:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.sm}"
    size: 36px
  tool-button-active-indicator:
    backgroundColor: "{colors.primary}"
    height: 2px
  tool-icon-selected:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.icon-selected-tint}"
    size: 20px
  selection-highlight:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.primary}"
  panel-surface:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.none}"
    padding: 12px
  window-chrome:
    backgroundColor: "{colors.neutral}"
    textColor: "{colors.on-surface}"
  canvas-viewport:
    backgroundColor: "{colors.surface-sunken}"
    textColor: "{colors.on-surface-muted}"
    rounded: "{rounded.none}"
  canvas-letterbox:
    backgroundColor: "{colors.canvas-letterbox}"
    textColor: "{colors.on-surface-muted}"
  status-bar:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface-muted}"
    height: 28px
    typography: "{typography.body-sm}"
  input-field:
    backgroundColor: "{colors.surface-overlay}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.xs}"
    padding: 6px
    height: 28px
    typography: "{typography.body-md}"
  input-field-disabled:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface-muted}"
    rounded: "{rounded.xs}"
    padding: 6px
    height: 28px
  slider-track:
    backgroundColor: "{colors.border}"
    height: 4px
    rounded: "{rounded.full}"
  slider-fill:
    backgroundColor: "{colors.primary}"
    height: 4px
    rounded: "{rounded.full}"
  list-item:
    backgroundColor: "#00000000"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.xs}"
    padding: 8px
    height: 28px
  list-item-selected:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.xs}"
    padding: 8px
    height: 28px
  list-item-selected-border:
    backgroundColor: "{colors.border-subtle}"
    textColor: "{colors.primary}"
  tooltip:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.on-surface}"
    rounded: "{rounded.sm}"
    padding: 8px
    typography: "{typography.body-sm}"
  toast-success:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.success}"
    rounded: "{rounded.sm}"
    padding: 8px
  toast-warning:
    backgroundColor: "{colors.surface-raised}"
    textColor: "{colors.warning}"
    rounded: "{rounded.sm}"
    padding: 8px
  focus-ring:
    backgroundColor: "{colors.focus-ring}"
    rounded: "{rounded.sm}"
  panel-border:
    backgroundColor: "{colors.border-subtle}"
    textColor: "{colors.on-surface}"
    height: 1px
  disabled-icon:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.on-surface-muted}"
    size: 16px
  caption-disabled:
    backgroundColor: "{colors.neutral}"
    textColor: "{colors.on-surface-disabled}"
    typography: "{typography.label-sm}"
  selection-overlay:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.selection}"
  tool-active-overlay:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.tool-active-bg}"
---

# PhotoTux DESIGN.md

Agent-facing design system for the PhotoTux desktop shell. Tokens above are **normative**. Prose below explains intent. Structural layout lives in [INFORMATION_ARCHITECTURE.md](./INFORMATION_ARCHITECTURE.md); product experience intent in [DESIGN_BRIEF.md](./DESIGN_BRIEF.md).

## Overview

PhotoTux looks and feels like a **professional KDE Plasma 6 creative tool**: matte dark panels, dense controls, one cool accent for interaction, and a quiet GPU canvas. The brand personality is **focused craftsmanship** — precise, calm, and fast — never playful marketing UI and never “developer dashboard neon.”

**Audience:** Digital artists and technical illustrators on Linux/Wayland who live in high-refresh, multi-pane editors.

**Emotional response:** Confidence and flow. The UI should disappear once the brush is moving. Chrome is legible at a glance and stays out of the stroke path.

**Density model:** Desktop-pro density (Breeze-like), not mobile touch targets. Prefer 28–36px controls and 11–12px UI type over spacious web padding.

**Canvas rule:** The center viewport is not a “card.” It is a **sunken work surface** (`surface-sunken`) that may show document pixels, letterboxing, or a neutral void — never decorative gradients.

## Colors

The palette is a **dark neutral stack** with a single **Plasma-blue primary** for selection, focus, and primary actions. Orange is reserved for warnings / destructive-attention tertiary moments — not for everyday chrome.

- **Primary (`#3DAEE9`):** Breeze-like interactive blue — selected tools, slider fills, focus rings, primary buttons, key links.
- **Secondary (`#A0A0A8`):** Muted slate for secondary labels, status text, inactive icons.
- **Tertiary (`#F67400`):** Warning / attention (export risk, unsaved emphasis). Do not use as a second brand accent for navigation.
- **Neutral / window (`#1E1E22`):** Application window base behind docks.
- **Surface (`#2B2B30`):** Toolbars, docks, status bar.
- **Surface raised (`#323238`):** Hover panels, menus, elevated chips.
- **Surface sunken (`#121214`):** Canvas letterbox and recessed wells.
- **Border (`#3D3D45`):** Hairline separators between chrome regions — prefer 1px solid over shadows.
- **On-surface (`#EFF0F1`):** Primary text/icons on dark surfaces.
- **Error / success:** Semantic only (failed export, successful save toast) — never as decoration.

**Rules:** One interactive accent per view region. Do not introduce purple gradients, glass blurs, or pure black `#000` full-bleed chrome. Selection use translucent primary (`selection`) rather than harsh invert.

## Typography

Use **system-native Linux fonts** so the app matches Plasma. Prefer **Noto Sans** (UI) and **Noto Sans Mono** (HUD metrics). If unavailable, fall back to the Qt/Plasma default sans.

- **Window / product title:** Semi-bold, compact — identity, not a marketing hero.
- **Panel titles (Properties, Layers):** `headline-sm` — short, scannable.
- **Control labels & body:** `body-md` / `body-sm` — dense but readable at 11–12px on dark UI.
- **Labels on forms:** `label-md` — slightly medium weight for field names.
- **HUD / FPS / coordinates:** `mono-hud` — tabular feel for changing numbers.

**Rules:** No display fonts, no italic brand wordmarks in chrome, no mixed type families beyond sans + mono. Avoid all-caps section headers except tiny HUD tags if needed.

## Layout

PhotoTux uses a **fixed multi-pane workspace grid**, not a fluid marketing layout.

- **Tool strip:** Fixed width `tool-strip-width` (48px), left edge.
- **Right docks:** Default `dock-width` (280px); collapsible to a handle.
- **Toolbar / status:** Fixed heights `toolbar-height` / `statusbar-height`.
- **Canvas:** Consumes all remaining space — always the flex grow region.
- **Spacing scale:** 4px base (`xs`); chrome padding `panel-padding` (12px); control stacks use `control-gap` (8px).
- **Separators:** 1px `border` lines between regions; no large card gaps.

**Rules:** Do not wrap the whole app in a centered max-width column. Do not add outer page margins like a website. Dock content uses containment via background + border, not drop shadows.

## Elevation & Depth

Depth is **tonal and linear**, not material shadow stacks.

- Chrome regions sit on `surface` against `neutral` window fill.
- Canvas sits *below* chrome via `surface-sunken`.
- Menus/tooltips use `surface-raised` + 1px border; soft shadow only if Qt style requires (keep blur ≤8px, low alpha).
- Active tool = `surface-raised` fill + primary indicator bar (or 1px primary border) — not translucent wash (keeps text contrast) and not glow bloom.
- Optional translucent `selection` / `tool-active-bg` may tint icons only; **never** place body text on those translucent fills.
- Modal dialogs dim the workspace slightly; dialog surface = `surface-raised`.

**Rules:** No multi-layer neon glows. No glassmorphism (`backdrop-blur` on entire shell). Hierarchy via value contrast and borders first.

## Shapes

**Architectural sharpness with light softening.**

- Default control radius: `sm` (4px).
- Inputs and list rows: `xs` (2px) for denser packing.
- Sliders/pills: `full` on tracks only.
- Dock panels and toolbars: `none` (flush to window edges).
- Canvas: `none`.

**Rules:** Do not mix pill-shaped primary buttons with sharp docks inconsistently — keep buttons at `sm`. Avoid 16px+ “web card” radii.

## Components

### Buttons

- **Primary:** Solid `primary` fill; use once per dialog or for the single affirmative action.
- **Secondary:** Raised surface; default for most chrome actions (“Reset view”).
- **Ghost / tool:** Transparent until hover; used in toolbars and tool strips.
- **Active tool:** `tool-button-active` wash + border; exclusive selection in the strip.

Height target **28px** for text buttons; **36px** hit target for icon tools inside the 48px strip.

### Sliders

- Track `border`, fill `primary`, height 4px.
- Value readout right-aligned mono or `body-sm` (e.g. `12px`, `100%`).
- Immediate update on move — no “Apply” for brush size.

### Lists (Layers)

- Row height ~28px; selected row uses `list-item-selected`.
- Leading visibility/lock icons; trailing optional opacity.
- No zebra stripes; use hover wash only.

### Inputs

- Sunken `surface-overlay` field, 1px `border`, focus ring `focus-ring`.
- Error state: border `error`, helper text `error`.

### Panels & chrome

- `panel-surface` for docks/toolbars; flush separators.
- Status bar uses muted text; HUD metrics may use `mono-hud` and `primary` sparingly for warnings (e.g. dropped frames).

### Canvas viewport

- Component `canvas-viewport`: sunken fill only — **document pixels are not styled by these tokens**.
- Placeholder copy (pre-GPU) uses `on-surface-muted`, centered, non-interactive decoration.
- Never place large CTAs on top of the canvas.

### Tooltips

- Raised surface, small type, short delay; describe tool name + shortcut.

### Focus

- Keyboard focus: 1–2px outline using `focus-ring` / `focus-ring` component token; offset outside control where possible.

## Do's and Don'ts

- **Do** keep the canvas region visually dominant and free of marketing chrome.
- **Do** use primary blue only for selection, focus, and true primary actions.
- **Do** prefer 1px borders and tonal steps over heavy shadows.
- **Do** design for stylus/mouse precision (dense targets, clear active tool).
- **Do** match Plasma dark conventions so the app feels installed, not embedded.
- **Don't** introduce a second brand accent for navigation (no purple + blue dual brand).
- **Don't** use light theme layouts or large Adwaita-style padding in v1.
- **Don't** animate brush strokes or block painting with page transitions.
- **Don't** put critical controls only in color (always icon + state + text/tooltip).
- **Don't** style GPU document content with QML opacity tricks that force extra copies.
- **Don't** invent new spacing values outside the token scale without updating this file.
- **Don't** use pure black full-window backgrounds that crush shadowless panel edges — use `neutral` / `surface` stack.

## Related documents

| Doc | Role |
|-----|------|
| [DESIGN_BRIEF.md](./DESIGN_BRIEF.md) | Experience problem, principles, scope |
| [INFORMATION_ARCHITECTURE.md](./INFORMATION_ARCHITECTURE.md) | Workspace map, flows, naming |
| [../SPEC.md](../SPEC.md) | Product architecture & phases |
| [01-decisions/](./01-decisions/) | Locked engineering ADRs |
