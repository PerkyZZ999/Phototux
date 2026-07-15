# ADR-013: Product Preferences (Grill Round 3)

## Status

Accepted

## Context

Secondary product UX and process preferences that do not redefine the core stack but guide defaults for document lifecycle, chrome assets, history, and CI. Locked in interactive grill Round 3 (2026-07-15).

## Decisions

### G13 — New document size

**Choice: C — ask every time, with resolution presets.**

- Opening **New Document** shows a dialog (or Phase 1 minimal stand-in) for width × height (and DPI later if needed).
- **Presets (at minimum):**
  | Preset | Size (px) |
  |--------|-----------|
  | 720p | 1280 × 720 |
  | 1080p | 1920 × 1080 |
  | 2K | 2560 × 1440 |
  | 4K | 3840 × 2160 |
- Custom width/height allowed when dialog exists.
- **Phase 1 temporary:** if full dialog is not ready, still **must not** silently invent a size without UI — use a minimal chooser or preset list, not a hidden default-only path. Prefer 1080p highlighted as “recommended” preset in the UI.

### G14 — Multi-document

**Choice: A — single document only for v1.**

- One open document per process until a later ADR.
- CLI: `phototux [file]` replaces/opens that single document (confirm if unsaved).
- Multi-tab / multi-window explicitly **out of MVP**.

### G15 — Icons

**Choice: B — bundled FOSS icon pack under `assets/`.**

- Owner will select a pack and place it under **`assets/`** (e.g. `assets/icons/`).
- App chrome and tool strip use the bundled set for consistency (not mixed system theme drift).
- Must be **FOSS-license-compatible** with GPL-3.0-or-later product (ADR-012); record pack name + license in `assets/icons/README.md` (or similar) when added.
- Until pack lands: geometric/placeholder glyphs OK in scaffolding only.
- **App icon / brand mark** may still be custom later; tool glyphs come from the pack.

### G16 — Undo granularity (Phase 3+)

**Choice: A — one undo step = one committed action / gesture.**

- Examples of one step: one tablet stroke (pen-down → pen-up), one fill, one transform commit, one filter apply.
- Not dab-level; not multi-stroke bundles unless user explicitly groups later.
- Matches “1 action / committed gesture per undo step.”

### G17 — CI timing

**Choice: A — local Arch/CachyOS only for now.**

- No requirement for GitHub Actions / cloud CI until public OSS or multi-dev need.
- Still run `cargo test -p phototux_engine` (etc.) locally before commits when code exists.
- Revisit when approaching public release (ADR-012).

### G18 — Zoom on open / new

**Choice: A — zoom to fit** viewport on open and after new document creation.

- User can switch to 100% via shortcut/toolbar later.
- Per-document “remember zoom” deferred (session restore Phase 5+).

## Options considered (summary)

| ID | Chosen | Rejected highlights |
|----|--------|---------------------|
| G13 | Ask + presets | Fixed silent default only |
| G14 | Single doc | Early multi-tab complexity |
| G15 | Bundled FOSS pack | System-only or full custom set first |
| G16 | Stroke/gesture undo | Dab-level / coarse-only |
| G17 | Local only | Early full CI |
| G18 | Zoom-to-fit | Always 100% / remember early |

## Consequences

- **Positive**: Clear UX defaults; icons consistent; history mental model simple
- **Negative**: New-doc dialog is real UI work; icon pack must be chosen/vendored by owner
- **Neutral**: CI can be added later without architecture change

## Revisit Date

- G13/G18: when New Document dialog ships  
- G14: post–Phase 5 if multi-doc requested  
- G15: when pack is added to `assets/`  
- G16: start of Phase 3 undo implementation  
- G17: public release prep  

## Dependencies

- **Depends on**: ADR-001, ADR-002, ADR-011, ADR-012
- **Blocks**: none hard; guides Phase 1 shell and Phase 3 history

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| 2026-07-15 | Accepted G13–G18 | Interactive grill R3 |
