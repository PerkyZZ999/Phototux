# Codebase ↔ Handbook Alignment Roadmap

| Field | Value |
| --- | --- |
| Status | **Complete** (handbook-ready exit 2026-07-16); Phase 5 product gates remain Deferred |
| Handbook | [`internal_docs/`](../README.md) |
| Gap inventory | [Codebase-Handbook-Gap-Analysis.md](Codebase-Handbook-Gap-Analysis.md) |
| Decisions | [Decision-Register.md](Decision-Register.md) |
| Tech stack | **Frozen to current codebase** ([DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase)) |

This roadmap is the living plan to keep the **Engineering Handbook** and the **shipping editor** aligned. It records locked choices, phase order, exit criteria, and what must not be rewritten.

---

## 1. Non-negotiable: Tech stack (codebase wins)

The following match the live workspace and **MUST NOT** be replaced or “re-deferred” by handbook prose. Handbook text is updated to **Accepted** for these items ([DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase)).

| Layer | Locked choice | Where in code |
| --- | --- | --- |
| Platform | Linux / Wayland desktop GUI only | App + ADRs archive |
| Language | Rust, edition **2024**, workspace `rust-version` as in root `Cargo.toml` | workspace |
| License | GPL-3.0-or-later | workspace |
| UI toolkit | **Qt 6.10+** QML, Controls 2 first | `phototux_ui`, `qml/` |
| FFI / shell bridge | **`qtbridge` 0.2** for app logic | `phototux_ui` |
| Canvas interop | Thin C++ canvas item + QML AOT registration only | `phototux_canvas`, `phototux` |
| GPU API | **`wgpu` 30**, Vulkan-first | `phototux_gpu` |
| Interactive present | **Zero-copy** GPU present (no steady-state full-frame CPU upload) | canvas / GPU |
| Workspace crates | `phototux`, `phototux_ui`, `phototux_engine`, `phototux_gpu`, `phototux_canvas`, `phototux_io` (+ spike as evidence only) | `Cargo.toml` |
| Native document | **`.ptx`** as editable persistence (evolve encoding in place) | `phototux_io` |
| Icons | Phosphor under `assets/icons/phosphor/` | assets |

**Explicit non-goals for stack:**

- No toolkit swap (GTK, egui, iced, web shell, Electron).
- No GPU API swap (Vulkan-via-wgpu stays).
- No immediate 18-crate split from handbook §32 — that text is a **logical ownership map**, not a package rename mandate ([DR-025](Decision-Register.md#dr-025--crate-topology-coarse-workspace)).
- No async runtime mandate; keep today’s workers / channels unless a measured need appears (stack choice, not architecture freeze on every concurrency pattern).

---

## 2. Alignment decisions (everything else)

Owner asked for agent decisions beyond the tech stack. Summary: **handbook owns contracts and target systems**; **codebase owns what already ships and must not regress**. Details below.

### 2.1 Decision matrix

| Topic | Winner | Decision |
| --- | --- | --- |
| Tech stack (above) | **Codebase** | Frozen ([DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase)) |
| Document owns truth | **Handbook** | Keep; code already close |
| Command mutation spine | **Handbook** | Route user-visible mutations through named commands; `AppSession` becomes host adapter |
| History = transactions | **Handbook** | Converge stacks into one transaction timeline (keep stroke coalescing) |
| Immutable render snapshots | **Handbook** | Introduce generation + snapshot leases incrementally; no overnight full pixel clone |
| GPU-first + CPU reference | **Handbook** | Interactive path stays zero-copy GPU; add CPU reference for tests / degraded |
| Tiling / pyramid | **Handbook (later)** | Design for it; implement when large-doc evidence demands ([DR-006](Decision-Register.md) Provisional knobs) |
| Crate topology | **Codebase** | Stay coarse; grow modules inside crates ([DR-025](Decision-Register.md#dr-025--crate-topology-coarse-workspace)) |
| Single vs multi document | **Codebase (v1)** | Single document until explicit multi-doc project ([DR-024](Decision-Register.md#dr-024--single-document-session-v1)) |
| Workspace / docking / panels | **Handbook** | Semantic models + descriptors; Qt implements presentation |
| Toolbars / shortcuts / context menus | **Handbook** | Action/command IDs drive chrome |
| Preferences / themes | **Handbook** | Services + dialogs; tokens may migrate from archived `DESIGN.md` |
| Color management | **Handbook** | Assign ≠ convert; ship foundation after command spine |
| Paths / shapes / text bake | **Handbook** | Paths first; Shape kind when graph amended; text bake before Character panel |
| Native format evolution | **Hybrid** | Keep `.ptx` identity; evolve toward chunked/integrity handbook model without greenfield rename ([DR-026](Decision-Register.md#dr-026--native-ptx-container-v1)) |
| PSD / interchange | **Codebase + Handbook** | Keep subset + report; deepen via adapters (22/27) |
| Plugins / ABI | **Handbook deferred** | No marketplace; capability seams only after command spine ([DR-009](Decision-Register.md)) |
| AI / cloud | **Both exclude** | Out of product boundary |
| Accessibility projection | **Handbook** | Grow semantic descriptors on Qt/AT-SPI |
| Performance budgets | **Hybrid** | Keep measured gates; treat handbook ledger as Provisional until fixtures promote |
| Living checklist | **Handbook tree** | New checklist under `internal_docs/`; archive checklist is historical |

### 2.2 Why these choices (short)

- **Command spine / snapshots / workspace models** prevent the QML shell from becoming the second source of truth — highest long-term leverage, fits Qt without replacing it.
- **Single-doc + coarse crates** avoid rewriting a working editor for paper purity.
- **`.ptx` keep + evolve** protects user files and ADR-016 investment.
- **CPU reference** unlocks headless conformance without touching the interactive present path.
- **Plugins last** — handbook agrees ABI is deferred; seams need a real command router first.

---

## 3. Phased roadmap

Each phase ends with: code green (`./scripts/check-rust.sh`), handbook/DR updates, gap-analysis row closures, and a short journal note under `archive/docs/04-journal/` (or a future `internal_docs/journal/` if created).

### Phase 0 — Documentation lock (this delivery)

**Goal:** Handbook reflects frozen stack and alignment decisions.

| Work | Exit |
| --- | --- |
| DR-023…026 Accepted / Provisional as below | Decision Register matches stack |
| Charter / lifecycle / developer guide no longer “toolkit deferred” | No DR-008 Deferred for Qt |
| This roadmap + gap analysis point here | Agents use one plan |

**Status:** Complete (2026-07-16).

---

### Phase 1 — Command spine (architecture, no UI redesign)

**Direction:** Handbook  
**Stack impact:** None (same crates)

| Slice | Work | Exit |
| --- | --- | --- |
| 1.1 | `CommandId` + registry + invoke API in `phototux_engine` (or module) | Named IDs for undo, layer ops, save triggers |
| 1.2 | Wrap existing graph mutations as commands with typed results | No silent `AppSession`-only graph writes for those ops |
| 1.3 | `AppSession` slots call router; keep QML bindings | Behavior unchanged for user |
| 1.4 | Map IDs into [Command Taxonomy](Command-Taxonomy.md) | Taxonomy lists shipped commands |
| 1.5 | Headless tests: invoke commands without Qt | DR-022 progress |

**Do not:** rename crates; rewrite brush hot path until router batching exists.

**Status:** Complete (commit `8ad2f51`).

---

### Phase 2 — Document version + snapshot leases

**Direction:** Handbook (incremental)

| Slice | Work | Exit |
| --- | --- | --- |
| 2.1 | Monotonic document generation / version on commit | Render/UI can detect stale |
| 2.2 | Snapshot metadata lease (graph revision + size + active ids) | Recomposite keyed by generation |
| 2.3 | Save/export pin a generation (receipt) | Dirty clears only on matching persist |
| 2.4 | History entries reference transaction/generation | Timeline inspectable |

**Do not:** full immutable pixel snapshots every stroke.

**Status:** Complete (with Phase 1, commit `8ad2f51`).

---

### Phase 3 — Shell contracts on Qt

**Direction:** Handbook models, codebase presentation

| Slice | Work | Exit |
| --- | --- | --- |
| 3.1 | Panel / tool descriptors (IDs, titles, default region) | Driven list; `Main.qml` consumes |
| 3.2 | Preferences service + dialog (XDG) | Survives restart |
| 3.3 | Action-driven menus / shortcuts / context menus v1 | Layer + canvas + selection |
| 3.4 | Theme tokens: migrate archived `DESIGN.md` → handbook Themes + QML | One token source |
| 3.5 | Workspace preset record (even if only Reset + Essentials) | Layout restore without doc history pollution |

**Do not:** full tear-off docking in first pass; topology model may precede drag UX.

**Status:** Complete (commit `7fe594f`).

---

### Phase 4 — Engine depth (handbook features on stack)

Order is priority, not parallel forever.

| Slice | Work | Exit |
| --- | --- | --- |
| 4.1 | CPU reference composite (subset blends) for tests | Fixture diffs vs GPU within tolerance |
| 4.2 | Text bake + Character chrome | Editable text → pixels path |
| 4.3 | Selection modify (feather/expand/…) + notify concepts hygiene | DR-011 closer |
| 4.4 | Color assign / convert foundation | DR-012 visible in UI |
| 4.5 | Paths engine + Paths panel (stroke to raster) | No Shape kind yet |
| 4.6 | Shape kind + tools (graph amend) | DR-020 in graph |
| 4.7 | Adjustment/filter wave 2 | More GPU kinds + dialogs |
| 4.8 | Layer styles v1 (shadow + stroke) | Nondestructive stack |
| 4.9 | Guides / grid / rulers / snap | View chrome |
| 4.10 | `.ptx` chunk/integrity evolution | DR-026; open old files |

**Status:** Complete including follow-ups (2026-07-16) — GPU styles/filters, color convert, Shape (DR-027), `.ptx` v2. Journals under `archive/docs/04-journal/2026-07-16-*.md`.

---

### Phase 5 — Scale & multi-doc (gated)

| Slice | Gate | Work |
| --- | --- | --- |
| 5.1 Tiling / sparse residency | Large-doc benchmark fails without it | Tile store + pyramid |
| 5.2 Multi-document tabs | Explicit amend of [DR-024](Decision-Register.md#dr-024--single-document-session-v1) | Session registry + tabs |
| 5.3 Plugin capability seams | Phase 1 solid; product need | Manifests only; ABI still deferred |
| 5.4 History spill / budgets | Memory pressure evidence | Retention policy UX |

**Status:** Deferred / gated (2026-07-16). Journal: `archive/docs/04-journal/2026-07-16-alignment-phase5-gated.md`.

---

## 4. Documentation alignment workstream

| Task | Status | Owner artifact |
| --- | --- | --- |
| Decision Register authoritative for stack + session model | **Done** | `Decision-Register.md` |
| Close / relabel gap-analysis rows to shipped v1 | **Done** | `Codebase-Handbook-Gap-Analysis.md` |
| Living implementation checklist | **Done** | `Implementation-Checklist.md` |
| Archived ADRs → DR map (evidence only) | **Done** | [Archived-ADR-to-DR-Map.md](Archived-ADR-to-DR-Map.md) |
| Root `SPEC.md` / `CONSTRAINTS.md` demoted to bridges | **Done** | root banners → handbook |
| Journal phase exits | **Ongoing** | `archive/docs/04-journal/` |

---

## 5. Definition of “aligned” — exit (2026-07-16)

Alignment for **handbook-ready development** is **Complete**. A release is “aligned enough” when:

| # | Criterion | Status |
| --- | --- | --- |
| 1 | Tech stack statements match [DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase) | **Met** |
| 2 | Document-authoritative edits are named commands (or documented host-only exemptions) | **Met** (`SessionState::invoke`) |
| 3 | Document generation + save receipts exist | **Met** (DR-005 v1 leases) |
| 4 | Shell: panel/tool descriptors + prefs; menus/shortcuts QML-hardcoded = Accepted v1 ([DR-015](Decision-Register.md#dr-015--workspace-state-separate-from-documents)) | **Met** (not full action registry) |
| 5 | Gap analysis open **A** items are Deferred/Provisional with phase (or Closed) | **Met** |
| 6 | No second normative tree; root SPEC/CONSTRAINTS are bridges | **Met** |

**Product work from here** follows handbook chapters + Decision Register. Phase 5 remains **gated** (tiling, multi-doc, plugins, history spill).

---

## 6. Anti-patterns (reject)

- Rewriting the editor to match handbook crate names.
- Reopening Qt / wgpu / Wayland / zero-copy debates without catastrophic evidence.
- Implementing multi-doc or plugin ABI “because the handbook mentions them.”
- Growing `Main.qml` business logic instead of commands.
- Steady-state CPU canvas upload “for convenience.”
- Silent contradiction between handbook MUST and code — update DR or code in the same change set.

---

## 7. Near-term (post-alignment)

Alignment sequence finished. Next work is **handbook-driven product slices**, not more contract bootstrapping. Prefer:

1. Features as new `command_id` + graph/GPU/I/O changes.
2. Phase 5 only when a listed gate fires.
3. Optional hardening: QML consume `*DescriptorsJson`; action-driven menus (target, not alignment).

---

## 8. Cross references

- [Codebase-Handbook-Gap-Analysis.md](Codebase-Handbook-Gap-Analysis.md)
- [Decision-Register.md](Decision-Register.md)
- [Archived-ADR-to-DR-Map.md](Archived-ADR-to-DR-Map.md)
- [Implementation-Checklist.md](Implementation-Checklist.md)
- [08-Command-System.md](../08-Command-System.md)
- [10-Document-Model.md](../10-Document-Model.md)
- [17-Rendering-Engine.md](../17-Rendering-Engine.md)
- [32-Developer-Guide.md](../32-Developer-Guide.md)
- Archived ADRs: [`archive/docs/01-decisions/`](../../archive/docs/01-decisions/)
