# Implementation Checklist (alignment)

Living tracker for [Alignment Roadmap](Alignment-Roadmap.md).  
Legend: `[ ]` todo · `[~]` partial · `[x]` done · `[!]` blocked · `[P]` post-v1

**Tech stack:** frozen ([DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase)) — do not track toolkit swaps here.

---

## Phase 0 — Documentation lock

- [x] Archive former `/docs/` → `archive/docs/`
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
- [ ] Remaining AppSession mutations (masks, filters, selection…) still direct — wrap in later slices

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
- [x] Theme tokens documented in `Theme.qml` (archived DESIGN.md source)
- [x] Workspace Reset → Essentials panel visibility

---

## Phase 4 — Engine depth

- [x] CPU reference composite (Normal/Multiply/Screen/… subset tests)
- [x] Text bake (`bake_text_rgba8` + Layer → Bake Text)
- [x] Character chrome (Properties Character section + live canvas preview)
- [x] Selection modify: feather / expand / contract (CPU + Select menu)
- [x] Color assign foundation (`document.assign-profile`; convert TBD)
- [x] Paths engine + stroke-to-layer (`PathDocument`, Paths panel descriptor)
- [!] Shape kind + tools (blocked on graph amend / DR-020)
- [x] Adjustment/filter wave 2 (Motion Blur + Emboss params + Filter menu; GPU shaders stub keys)
- [x] Layer styles v1 (Drop Shadow + Stroke metadata + CPU `apply_styles_rgba8`)
- [x] Guides / grid / rulers / snap (View menu + overlays + prefs; snap on guide place)
- [~] `.ptx` chunk/integrity evolution (compat — deferred with DR-026 evolve-in-place)

**Phase 4 exit (v1):** foundation + Character/guides chrome. Remaining: Shape kind, full GPU style/filter passes, `.ptx` integrity chunks.

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
- [ ] `./scripts/check-rust.sh` green
- [ ] No paragraph-long workaround comments
