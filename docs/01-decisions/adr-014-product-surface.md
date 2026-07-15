# ADR-014: Product Surface — Desktop GUI Only (v1)

## Status

Accepted

## Context

PhotoTux is a professional image **editor for the Linux desktop**. Owner confirmation (2026-07-15): MVP/v1 is **only a desktop application** — not a CLI tool, not a TUI, not a web app.

## Options Considered

### Option 1: Desktop GUI only

- **Pros**: Matches vision, design system, KDE integration, tablet/canvas UX
- **Cons**: No headless batch CLI in v1
- **Reversibility**: Medium (CLI could be added later as separate binary)

### Option 2: GUI + first-class CLI

- **Pros**: Scripting, CI image ops
- **Cons**: Scope split; different UX/testing surface
- **Reversibility**: Medium

### Option 3: TUI / terminal editor

- **Pros**: None for this product
- **Cons**: Contradicts GPU canvas + QML shell
- **Reversibility**: N/A

## Decision

**Option 1.**

| Surface | v1 / MVP |
|---------|----------|
| **Desktop GUI** (Qt Quick window) | **Yes — the product** |
| **CLI** (interactive or batch tool as product) | **No** |
| **TUI** | **No** |
| **Web / Electron** | **No** |
| `cargo run -p phototux` / `cargo test` | **Dev-only** tooling, not end-user product |
| Desktop file open (`.desktop` / MIME / optional argv from DE) | **Later OK** as OS integration (Phase 5), not a “CLI app” |

End users launch PhotoTux from the desktop environment (icon, app menu). No separate `phototux-cli` or terminal UI.

## Consequences

- **Positive**: One UX surface; docs and IA stay canvas-first
- **Negative**: Batch automation deferred
- **Neutral**: Engine crate remains testable without GUI via unit tests

## Revisit Date

Post–Phase 5 if batch processing demand appears (would be a new binary ADR).

## Dependencies

- **Depends on**: ADR-001, ADR-002
- **Blocks**: any “CLI product” feature requests without amendment

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| 2026-07-15 | Accepted | Owner explicit: desktop app only |
