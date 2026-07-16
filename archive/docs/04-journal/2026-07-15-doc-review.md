# Documentation alignment review — 2026-07-15

## Scope

Review of product docs, ADRs, design set, research, checklists vs locked grill decisions (ADR-001…013) + owner clarifications (desktop-only surface → ADR-014).

## Findings & resolutions

| # | Issue | Severity | Resolution |
|---|--------|----------|------------|
| 1 | Product described as if **CLI** open path (`phototux [file]`) were first-class | Medium | **ADR-014**: desktop GUI only; no CLI/TUI product. IA + ADR-013 updated |
| 2 | IA F1: “default blank canvas” vs ADR-013 **New Document ask + presets** | Medium | F1 → New Document dialog / presets first |
| 3 | `development.md` Phase 2 still said “spike skipped” | Medium | Spike is **required** (ADR-010); checklist rewritten |
| 4 | SPEC verify path `qml-interface/Cargo.toml` outdated | Medium | SPEC §6 → workspace `cargo run -p phototux` + Qt6 PATH |
| 5 | README/ADRs tag stale `decisions-locked-v1` only | Low | Point to grill R1–R3 / ADR-001…014 |
| 6 | CHANGELOG inception still “spike skipped” without later context | Low | Historical; R1 section already reverses for interop |
| 7 | DOSSIER “open questions” pre-grill (Kirigami, spike) | Info | Research snapshot — superseded by ADRs; left as historical |
| 8 | Soft constraint “qtbridge only” vs hybrid G3 | Low | CONSTRAINTS soft text clarified |
| 9 | Checklist too implementation-precise | Process | Rewritten phase-level; plans per phase during build |
| 10 | “rust-doctor CLI” wording in AGENTS | Noise | Means **tool binary**, not product CLI — clarified |

## Alignment check (post-fix)

| Area | Status |
|------|--------|
| Stack (Qt QML, Rust, qtbridge hybrid, wgpu, zero-copy) | Aligned |
| Phases 1 → 1.5 spike → 2…5 | Aligned |
| Design brief / DESIGN.md / IA structure | Aligned (F1 fixed) |
| Single doc, icons pack, undo, zoom-to-fit, new-doc presets | Aligned |
| Desktop GUI only | Aligned (ADR-014) |
| SLOs as gates | Aligned |
| No Electron/GTK/CPU-upload product path | Aligned |

## Residual non-issues

- **Raster + vector** in vision (SPEC/README) vs vector **out of MVP**: intentional roadmap, not conflict.
- Research docs keep rejected alternatives for audit trail.
- Journal of removed scaffold is historical.

## Follow-up

- Owner: icon pack under `assets/icons/` when ready  
- Optional: human design review of DESIGN.md vs Plasma  
