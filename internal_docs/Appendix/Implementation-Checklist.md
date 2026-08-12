# Implementation Checklist (alignment — historical)

**Status:** Alignment Phases 0–4 **complete**. Do not add new product work here.

**Living product tracker:** [Handbook-Parity-Checklist.md](Handbook-Parity-Checklist.md) · [Handbook-Parity-Roadmap.md](Handbook-Parity-Roadmap.md).

Former tracker for [Alignment Roadmap](Alignment-Roadmap.md).  
Legend: `[ ]` todo · `[~]` partial · `[x]` done · `[!]` blocked · `[P]` post-v1

**Tech stack:** frozen ([DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase)) — do not track toolkit swaps here.

---

## Phase 0 — Documentation lock

- [x] Former `/docs/` tree retired (once archived, then removed 2026-07-18; handbook is sole normative tree)
- [x] Handbook authoritative (`internal_docs/`)
- [x] Gap analysis written
- [x] Alignment roadmap + DR-023…026
- [x] Charter / lifecycle / developer guide stack language
- [x] This checklist seeded

---

## Phase 1 — Command spine

- [x] CommandId + registry + invoke in `phototux_engine`
- [x] Wrap core graph mutations (undo/redo, layer ops) as commands
- [x] `AppSession` routes core slots through router (behavior-stable)
- [x] Command Taxonomy lists shipped IDs
- [x] Headless command tests (no Qt)
- [x] Remaining document-authoritative AppSession mutations wrapped (selection/mask/filter/style/text/shape/raster; GPU-then-commit where needed)

---

## Phase 2 — Version + snapshot leases

- [x] Document generation / version on commit
- [x] Snapshot metadata lease for recomposite
- [x] Save pin generation + receipt (`mark_persisted` / dirty vs persisted)
- [x] History entries reference generation

---

## Phase 3 — Shell contracts (Qt)

- [x] Panel / tool descriptors (`shell.rs` + JSON props)
- [x] Preferences service + dialog (XDG `preferences.json`)
- [x] Window menu panel toggles + layer context menu v1
- [x] Theme tokens documented in `Theme.qml` (handbook Themes)
- [x] Workspace Reset → Essentials panel visibility

---

## Phase 4 — Engine depth

- [x] CPU reference composite (Normal/Multiply/Screen/… subset tests)
- [x] Text bake (`bake_text_rgba8` + Layer → Bake Text)
- [x] Character chrome (Properties Character section + live canvas preview)
- [x] Selection modify: feather / expand / contract (CPU + Select menu)
- [x] Color assign foundation (`document.assign-profile`)
- [x] Paths engine + stroke-to-layer (`PathDocument`, Paths panel descriptor)
- [x] Shape kind + tools (DR-027; rect/ellipse/line + rasterize)
- [x] Adjustment/filter wave 2 (Motion Blur + Emboss; GPU `EffectPass`)
- [x] Layer styles v1 (Drop Shadow + Stroke; GPU pre-pack + CPU ref)
- [x] Guides / grid / rulers / snap (View menu + overlays + prefs; snap on guide place)
- [x] Color convert (`document.convert-profile`; sRGB↔Display-P3)
- [x] `.ptx` v2 chunked writes + v1 read compat (DR-026)

**Phase 4 exit:** complete (2026-07-16). Phase 5 remains gated.

---

## Phase 5 — Gated (no code until gates fire)

- [!] Tiling / pyramid — large-doc benchmark evidence required
- [!] Multi-document — explicit amend of DR-024 required
- [P] Plugin capability seams (manifests only; ABI deferred)
- [P] History spill / retention UX

---

## Standing rules

- [ ] Update this file when starting/finishing a slice
- [ ] Update gap analysis rows for closed architecture items
- [ ] `rust-tc quick` green
- [ ] No paragraph-long workaround comments
