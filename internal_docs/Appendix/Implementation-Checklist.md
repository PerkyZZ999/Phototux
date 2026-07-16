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

- [ ] CommandId + registry + invoke in `phototux_engine`
- [ ] Wrap graph mutations (undo, layer ops, fill, …) as commands
- [ ] `AppSession` routes through router (behavior-stable)
- [ ] Command Taxonomy lists shipped IDs
- [ ] Headless command tests (no Qt)

---

## Phase 2 — Version + snapshot leases

- [ ] Document generation / version on commit
- [ ] Snapshot metadata lease for recomposite
- [ ] Save/export pin generation + receipt
- [ ] History entries reference generation/transaction

---

## Phase 3 — Shell contracts (Qt)

- [ ] Panel / tool descriptors
- [ ] Preferences service + dialog
- [ ] Action menus / shortcuts / context menus v1
- [ ] Theme tokens migrated (archive DESIGN → handbook Themes + QML)
- [ ] Workspace preset / Reset (minimal)

---

## Phase 4 — Engine depth

- [ ] CPU reference composite (test subset)
- [ ] Text bake + Character chrome
- [ ] Selection modify ops
- [ ] Color assign / convert foundation
- [ ] Paths engine + panel
- [ ] Shape kind + tools (after graph amend)
- [ ] Adjustment/filter wave 2
- [ ] Layer styles v1
- [ ] Guides / grid / rulers / snap
- [ ] `.ptx` chunk/integrity evolution (compat)

---

## Phase 5 — Gated

- [!] Tiling / pyramid (evidence gate)
- [!] Multi-document (amend DR-024)
- [P] Plugin capability seams (after Phase 1; ABI deferred)
- [P] History spill format

---

## Standing rules

- [ ] Update this file when starting/finishing a slice
- [ ] Update gap analysis rows for closed architecture items
- [ ] `./scripts/check-rust.sh` green
- [ ] No paragraph-long workaround comments
