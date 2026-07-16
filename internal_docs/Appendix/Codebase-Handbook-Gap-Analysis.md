# Codebase ↔ Engineering Handbook Gap Analysis

| Field | Value |
| --- | --- |
| Date | 2026-07-16 |
| Handbook | [`internal_docs/`](../README.md) (authoritative Engineering Handbook) |
| Codebase | workspace crates under `crates/` as of this date |
| Archived prior docs | [`archive/docs/`](../../archive/docs/) (former `/docs/`; retain until explicit delete) |
| Goal | Align handbook and code **without** a rewrite mess |

**Verdict first:** Keep the **shipping codebase spine** (Qt 6 + qtbridge + wgpu zero-copy canvas, `phototux_engine` graph, `.ptx` / PSD adapters). Treat the handbook as the **target architecture and contracts**. Close gaps with **incremental refactors and Decision Register promotions** — do **not** big-bang rewrite to the proposed 18-crate layout or defer Qt.

**Locked 2026-07-16:** Tech stack frozen to codebase ([DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase)). All other alignment calls + phases: **[Alignment-Roadmap.md](Alignment-Roadmap.md)**. Living tracker: [Implementation-Checklist.md](Implementation-Checklist.md).

---

## 1. Scope and method

Compared:

- Handbook charter, IA, lifecycle, workspace/docking/panels, command system, document/layers, render, history, I/O, plugins, preferences, Decision Register, developer guide.
- Live crates: `phototux`, `phototux_ui`, `phototux_engine`, `phototux_gpu`, `phototux_canvas`, `phototux_io` (+ spike).
- Archived ADRs in `archive/docs/01-decisions/` (historical locks that still describe what the code implements).

Legend for severity:

| Severity | Meaning |
| --- | --- |
| **A — Architecture** | Ownership / mutation / truth model differs; alignment needs deliberate design work |
| **F — Feature gap** | Handbook requires capability code lacks (or only stubs) |
| **C — Contract** | Naming, crate topology, deferred vs locked decisions disagree |
| **P — Process** | Docs/ops (ADR vs DR, checklists, evidence) not hooked to handbook |

---

## 2. What already matches (keep)

| Area | Handbook | Codebase |
| --- | --- | --- |
| Local-first, no cloud/AI product | DR-001, 00 | Matches product surface |
| Document as editable truth (intent) | DR-002 | `DocumentGraph` / GPU layer textures are authority for pixels; UI projects |
| wgpu GPU path | DR-006 | `phototux_gpu` + Vulkan-first canvas present |
| Portable engine vs UI | DR-007 (spirit) | `phototux_engine` has no Qt; UI in `phototux_ui` |
| Layered raster stack | 10 / 11 | Raster, Group, Text, Adjustment; masks, clip, opacity, blends |
| Undo / history panel | DR-004 (partial) | `HistoryService` + stroke/transform/selection stacks |
| Staged / atomic save | DR-014 | `.ptx` atomic write path in `phototux_io` |
| Interchange ≠ native | DR-013 (spirit) | PSD behind adapter + compatibility report |
| Desktop Linux host | 00, 02 | Wayland / Qt 6 app; no web/CLI product |
| Vendor-neutral naming (direction) | DR-019 | UI strings mostly generic; handbook is stricter |

These are assets. Alignment must **not** discard zero-copy present, working brush path, or `.ptx` round-trip without a measured replacement.

---

## 3. Differences (handbook vs code)

### 3.1 Architecture / mutation spine — **A**

| # | Handbook says | Code has | Severity |
| --- | --- | --- | --- |
| A1 | Every user-visible mutation enters a **named command** with validation → transaction → history (DR-003, 08) | Mutations mostly via **`AppSession` QObject slots** + direct engine/GPU calls; `EngineCommand` is **paint-worker only** | **A** |
| A2 | Render consumes **immutable versioned snapshots / deltas** (DR-005, 17) | Canvas holds live GPU document; recomposite from mutable graph; no snapshot lease API | **A** |
| A3 | Workspace / docking / panels are first-class with **workspace transactions** separate from document history (03–05, DR-015) | Single large `qml/Main.qml` shell; docks are layout regions, not a topology model | **A** |
| A4 | Lifecycle orchestrates session, multi-window, recovery, renderer generations (02) | App start → New Document / open; recovery APIs partial; no formal lifecycle controller | **A** |
| A5 | Per-document mutation serialization; multi-doc first-class (DR-010, 02–03) | **Single document** session (archived ADR-013); no doc registry | **A** / **C** |
| A6 | GPU-first **with mandatory CPU reference/fallback** + tiling / pyramid (DR-006, 17) | GPU-first interactive path; **no** full CPU compositor; **no** sparse tile pyramid; full-layer textures | **A** |
| A7 | Presentation model toolkit-neutral; UI toolkit **Deferred** (DR-008) | **Qt 6 + qtbridge locked in code** and archived ADR-002/003 | **C** (see §5) |
| A8 | Proposed crate split: domain / commands / history / snapshot / render-graph / linux-host… (32) | Coarse crates: `engine`, `gpu`, `canvas`, `ui`, `io`, `phototux` | **C** |
| A9 | Extension seams + capability model now; ABI deferred (DR-009, 23) | No plugin host; no extension opaque chunks in `.ptx` yet | **F** / **C** |

### 3.2 Document / layers / engines — **F** / **A**

| # | Handbook says | Code has | Severity |
| --- | --- | --- | --- |
| D1 | Rich document aggregate: resources, profiles, version vectors, opaque extension objects (10, 27) | `DocumentGraph` + size + layers; limited metadata; no ICC pipeline | **F** |
| D2 | Shape engine + Shape layers; text bake boundaries (18, 19, DR-020) | Text metadata create only; **no Shape kind**; no path engine | **F** |
| D3 | Color management: assign ≠ convert; soft-proof (16, DR-012) | FG/BG/swatches; no document profile / convert | **F** |
| D4 | Filter engine as declarative plans + CPU/GPU executors (15) | Brightness/Levels + Gaussian effect; contracts for more | **F** |
| D5 | Selection concepts distinct: object vs pixel vs focus vs edit target (DR-011, 12) | Pixel selection + active layer; concepts collapsed in UI | **A** / **F** |
| D6 | Mask system with vector masks, refine, apply semantics (13) | Raster layer masks + clipping; no vector mask / refine | **F** |
| D7 | History = transaction records with retention budgets / spill (20) | Mixed: graph undo commands + GPU pixel snapshots for strokes/transforms | **A** |
| D8 | Native container: chunked, tile-addressable, integrity, incremental strategies; **bytes deferred** (27, DR-013) | Concrete **`.ptx`** (archived ADR-016) — works, but not handbook’s full chunk/tile model | **C** / **A** |

### 3.3 Shell / UX systems — **F**

| # | Handbook says | Code has | Severity |
| --- | --- | --- | --- |
| U1 | Docking system with tear-off, auto-hide, topology validation (04) | Fixed multi-pane layout | **F** |
| U2 | Panel system descriptors, contributions, placeholders (05) | Layers, History, Properties, Swatches, Navigator hardcoded | **F** |
| U3 | Toolbar / tool options as registry-driven (06) | Tool strip + Properties in QML | **F** |
| U4 | Context menus from action/command registry (07) | Mostly menu bar / dock buttons | **F** |
| U5 | Shortcut system with customizable bindings (09) | Partial hardcoded shortcuts | **F** |
| U6 | Preferences + themes as persisted services (24, 25) | Theme tokens in QML; no Preferences dialog | **F** |
| U7 | Dialogs / command search / workspace presets (03, 26) | New/Export/About/unsaved; no workspace manager | **F** |
| U8 | Accessibility semantic tree projection + AT-SPI host (29, DR-016) | Basic Qt a11y; no handbook descriptor projection | **F** |

### 3.4 I/O / formats / clipboard — **F** / **C**

| # | Handbook says | Code has | Severity |
| --- | --- | --- | --- |
| I1 | Format adapters with hard allocation limits, loss disclosure (22) | Rasters + PSD subset + report; limits uneven | **F** |
| I2 | Clipboard as capability-scoped host bridge (21) | Selection copy/paste-as-layer; no full handbook model | **F** |
| I3 | Recovery bound (~60s) + lifecycle restore (02, 00) | Recovery helpers exist; UX incomplete | **F** |

### 3.5 Performance / testing / process — **C** / **P**

| # | Handbook says | Code has | Severity |
| --- | --- | --- | --- |
| P1 | Budgets **Provisional** until fixtures promote gates (DR-017, 30) | Archived ADR-008 hard gates; some measured (composite, boot, FPS) | **C** |
| P2 | Headless core command tests mandatory (DR-022, 31) | Engine unit tests yes; no command-router conformance suite | **F** / **P** |
| P3 | Decision Register is index; high-cost locks via DR/ADR process | Handbook DRs vs archived ADRs **diverge** (toolkit, format, single-doc) | **P** |
| P4 | Handbook README directory map still says `docs/` | Lives in `internal_docs/` | **P** (doc hygiene) |

---

## 4. Code (or archived ADRs) not reflected / contradicted by handbook

| Item | Where | Handbook stance | Note |
| --- | --- | --- | --- |
| Qt 6 + qtbridge + QML AOT | `phototux_ui`, `phototux` | DR-008 **Deferred** | **Strongest conflict** — code already shipped |
| Zero-copy Vulkan present only | archived ADR-005, canvas | Allows CPU fallback for correctness; present path not spelled as ADR-005 | Compatible if CPU is **non-interactive** path |
| Single document v1 | archived ADR-013 | Lifecycle/workspace assume multi-doc | Promote single-doc as **Provisional** or amend DR |
| Concrete `.ptx` format | `phototux_io`, ADR-016 | Native bytes **Deferred** (DR-013) | Promote `.ptx` as Provisional/Accepted with migration story |
| Phosphor icons, new-doc presets, zoom-to-fit | UI + ADR-013 | Not contradicted; under-specified in handbook | Keep; document under prefs/IA |
| `phototux-spike-interop` | crates | Spike done historically | Keep as evidence; handbook wants measured spikes before freezes |
| Root `SPEC.md` / `CONSTRAINTS.md` / `AGENTS.md` | repo root | Not part of handbook series | Need re-home or “bridge” docs |
| Production checklist / FEATURES_TODO | `archive/docs/` | No living checklist in handbook | Need handbook-linked checklist or revive under `internal_docs/` |

---

## 5. Decision conflicts (must resolve explicitly)

| Topic | Archived ADR / code | Handbook DR | Recommendation |
| --- | --- | --- | --- |
| UI toolkit | Qt 6 Accepted (ADR-002/003) | DR-008 Deferred | **Promote Qt+qtbridge to Accepted** in Decision Register with measured evidence (boot, FPS, a11y baseline). Do not “unchoose” Qt. |
| Multi-document | Single-doc (ADR-013) | Multi-doc lifecycle/workspace | Keep **single-doc Provisional** until intentional ADR/DR amend; delay tabs. |
| Native format | `.ptx` locked (ADR-016) | Bytes deferred (DR-013) | **Accept `.ptx` v1** as Provisional container; evolve toward chunk/tile model without breaking open. |
| Zero-copy present | ADR-005 ship gate | GPU-first + CPU fallback | Keep zero-copy **interactive** present; add CPU path for tests/degraded only. |
| Plugin ABI | Deferred / forbidden product store | DR-009 seams now, ABI later | Align: seams OK later; no marketplace. |
| Shape layers | ADR-017 kinds exclude Shape | Shape engine Accepted (DR-020) | Amend graph kinds when Paths/Shapes slice starts. |
| Crate topology | Coarse workspace | Fine-grained proposal (32) | Treat 32 as **target modularization**, not immediate split. |

---

## 6. Recommendations (how to align without a mess)

### 6.1 Strategic choice

**Hybrid: codebase direction for platform + present path; handbook direction for contracts and future systems.**

| Stick with codebase | Adopt from handbook |
| --- | --- |
| Qt 6 / qtbridge / QML shell | Command spine (route mutations through named commands) |
| wgpu Vulkan zero-copy canvas | Immutable snapshot/delta for render invalidation (incremental) |
| Current crate set (split only when pain is real) | Workspace/docking/panel **models** before full tear-off UI |
| `.ptx` + raster/PSD adapters | Chunk/tile evolution of `.ptx`; stronger import limits |
| Single-doc until deliberate multi-doc project | Selection/focus/target concept hygiene |
| Shipping vertical features | Decision Register as living ADR index |

**Do not:** pause features for an 18-crate rewrite, replace Qt, or throw away GPU document ownership for a paper-pure snapshot design overnight.

### 6.2 Immediate process (this week)

1. **Handbook is authoritative** for engineering intent (`internal_docs/`).
2. Keep `archive/docs/` read-only historical; delete only after DR migration + owner OK.
3. Update root `AGENTS.md` / `README.md` to point at handbook; conflict log → `internal_docs/Appendix/` or journal under archive.
4. **Migrate decisions:** for each Accepted archived ADR that code implements, add/update a Decision Register entry (or mark DR-008/DR-013 accordingly).
5. Add a living **implementation checklist** under `internal_docs/` (or Appendix) that replaces `archive/docs/03-checklists/development.md` as the work tracker.

### 6.3 Implementation alignment order (low mess → high leverage)

Phase α — **Contracts without UI rewrite**

1. Introduce a thin **command registry + router** in `phototux_engine` (or `phototux_commands` module): wrap existing ops (layer opacity, undo, fill, etc.) as named commands; keep AppSession as host adapter that *invokes* commands.
2. Document **version / dirty / save receipt** on `DocumentGraph` closer to handbook language.
3. Snapshot **handle** (even if initially “clone metadata + generation counter”) for recomposite invalidation — full immutable pixel snapshots later.

Phase β — **Shell systems as models**

4. Extract panel/tool **descriptors** from `Main.qml` (IDs, titles, default docks) matching 05/06 — still Qt presentation.
5. Preferences service + dialog (24) wired to disk under XDG paths.
6. Context menus + shortcut map driven by action IDs (07/09).

Phase γ — **Engine depth toward handbook**

7. CPU reference composite for tests (subset of blend modes).
8. Color assign/convert foundation (16).
9. Paths → then Shape kind DR amend (19).
10. Tile/sparse storage only when large-doc benchmarks demand it (do not invent tiling early).

Phase δ — **Multi-doc / plugins**

11. Multi-doc only after DR/ADR amend + session registry.
12. Extension seams after command spine is real; ABI stays deferred.

### 6.4 What “done aligning” means

- Every shipped mutation path listed in Command Taxonomy (or explicitly exempted as transient preview).
- Decision Register matches reality (Qt, `.ptx`, single-doc, zero-copy).
- Handbook MUST requirements either: implemented, Provisional with evidence plan, or consciously Deferred with DR.
- No second competing “official” doc tree outside `internal_docs/` + thin root bridges (`README`, `AGENTS`, `SPEC`/`CONSTRAINTS` until absorbed).

---

## 7. Risk if we pick the wrong extreme

| Extreme | Risk |
| --- | --- |
| **Handbook purity first** | Months of crate/workspace rewrite; lose working editor; DR-008 thrash |
| **Codebase only, ignore handbook** | Shell/features grow as one-off QML; harder multi-doc, a11y, plugins, large docs later |
| **Hybrid (recommended)** | Some dual-writing during transition; managed via command adapter + DR promotions |

---

## 8. Owner decisions (resolved 2026-07-16)

1. **Qt 6 + qtbridge** → **Accepted** ([DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase)); DR-008 superseded.
2. **`.ptx` v1** → **Accepted** ([DR-026](Decision-Register.md#dr-026--native-ptx-container-v1)); evolve encoding in place.
3. **Single-document** v1 → **Accepted** ([DR-024](Decision-Register.md#dr-024--single-document-session-v1)).
4. **Zero-copy interactive present** stays hard; CPU = tests/degraded only (roadmap Phase 4.1).
5. Production tracker → [Implementation-Checklist.md](Implementation-Checklist.md); plan → [Alignment-Roadmap.md](Alignment-Roadmap.md).

---

## 9. Cross references

- Handbook index: [README](../README.md)
- Decision Register: [Decision-Register.md](Decision-Register.md)
- Developer guide (crate proposal): [32-Developer-Guide.md](../32-Developer-Guide.md)
- Archived ADRs / old IA / checklists: [`archive/docs/`](../../archive/docs/)
- This analysis should be updated when a DR is promoted or a Phase α–δ slice lands.
