# Codebase ↔ Engineering Handbook Gap Analysis

| Field | Value |
| --- | --- |
| Date | **2026-08-12** (refresh; prior snapshot 2026-07-18) |
| Handbook | [`internal_docs/`](../README.md) (authoritative Engineering Handbook) |
| Codebase | workspace crates under `crates/` as of this date |
| Living product tracker | [Handbook-Parity-Checklist.md](Handbook-Parity-Checklist.md) |
| Alignment history | [Implementation-Checklist.md](Implementation-Checklist.md) (Phases 0–4 — do not reopen) |
| Interactive QA | [Interactive-Stability-Checklist.md](Interactive-Stability-Checklist.md) |
| Former ADR map | [Archived-ADR-to-DR-Map.md](Archived-ADR-to-DR-Map.md) (index only; source files removed) |

**Verdict first:** **Spine parity is shipped.** Keep the shipping stack (Qt 6 + qtbridge + wgpu zero-copy, `phototux_engine` graph, `.ptx` / PSD). Treat the handbook as contracts + depth target. Remaining work is **DR-028 chapter depth**, **DR-029 gated scale/plugins**, and **ungated polish `[~]`** — not architecture rewrites.

**Stack locked:** [DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase).  
**Depth deferral:** [DR-028](Decision-Register.md#dr-028--engine-depth-deferred-beyond-p5p10-slices).  
**Scale/plugin gates:** [DR-029](Decision-Register.md#dr-029--p11p12-remain-gated-no-ungated-impl).

---

## 1. Scope and method

Compared:

- Handbook chapters 00–32 + Decision Register + parity checklist (2026-07-17 status snapshot).
- Live crates: `phototux`, `phototux_ui`, `phototux_engine`, `phototux_gpu`, `phototux_canvas`, `phototux_io`.
- Interactive stability pass (issues T-009–T-016 closed).

Legend:

| Severity | Meaning |
| --- | --- |
| **A — Architecture** | Ownership / mutation / truth model differs |
| **F — Feature gap** | Handbook capability missing or stub-only |
| **C — Contract** | Naming / Provisional evidence / deferred encoding |
| **P — Process** | Docs/ops hygiene |

Status column for open rows:

| Status | Meaning |
| --- | --- |
| **Closed (v1)** | Shipping spine matches handbook Accepted v1 |
| **Partial** | Spine shipped; residual polish or chapter depth |
| **Deferred (DR-028)** | Explicit engine/CMS/depth deferral |
| **Gated (DR-029)** | No impl until evidence / product need |
| **Open** | Ungated polish still worth shipping |

---

## 2. What already matches (keep)

| Area | Handbook | Codebase |
| --- | --- | --- |
| Local-first, no cloud/AI | DR-001, 00 | Product surface matches |
| Document owns truth | DR-002 | `DocumentGraph` + GPU textures; UI projects |
| wgpu GPU path | DR-006 | `phototux_gpu` Vulkan-first + canvas present |
| Portable engine vs UI | DR-007 | Engine has no Qt; UI in `phototux_ui` |
| Layered stack | 10 / 11 / DR-027 | Raster/Group/Text/Adjustment/Shape/Fill; masks, clip, blends |
| Command spine | DR-003, 08 | `SessionState::invoke` + taxonomy; host-only exemptions |
| Snapshot leases + pixel publish | DR-005 | Generation + leases + `SnapshotPublisher` (64 MiB) |
| Color assign/convert + soft-proof | DR-012, 16 | Commands + Image menu + display ICC discovery |
| Preferences / Theme packs | 24, 25 | XDG prefs schema 5+; `Theme.qml` density/contrast |
| `.ptx` v2 | DR-026 | Typed chunks + CRC diagnostics; v1 read |
| Guides / Character chrome | 03 / 18 | View guides/grid/rulers; Character + text bake |
| History timeline | DR-004 | Unified timeline + jump + retention UI |
| Atomic save | DR-014 | `.ptx` staged write |
| Multi-doc tabs | DR-024 v2 | `DocumentRegistry` + TabBar (max 8) |
| Docking tear-off / auto-hide | 04, DR-015 | Topology model + QML floating + persist |
| Action chrome | 06–09 | Descriptors → menus / strip / shortcuts / palette |
| A11y projection | 29, DR-016 | `accessibilityTreeJson` + AT-SPI projection + Qt Accessible |
| Clipboard | 21 | RGBA + selection/mask R8 + OS image; 64 MiB refuse |
| Verification | DR-022, 31 | `command_conformance` + soft CI / Tier M proxies |
| Desktop Linux host | 00, 02 | Wayland / Qt 6; no web/CLI product |

Do **not** discard zero-copy present, brush path, or `.ptx` round-trip without a measured replacement.

---

## 3. Differences (handbook vs code) — current

### 3.1 Architecture / mutation spine — **A**

| # | Handbook says | Code has | Status |
| --- | --- | --- | --- |
| A1 | Named commands for document mutations (DR-003) | `SessionState::invoke` + host-only exemptions | **Closed (v1)** |
| A2 | Immutable snapshots / deltas (DR-005, 17) | Leases + bounded pixel publisher; dense deltas Provisional | **Closed (v1)**; dense deltas → **Deferred (DR-028)** |
| A3 | Workspace / docking transactions (03–05, DR-015) | Descriptors + tear-off/auto-hide + builtins + **user-named presets**; split-graph polish | **Closed (v1)**; split graph → **Partial** |
| A4 | Lifecycle controller, recovery, renderer gens (02) | Autosave + restore chooser + safe-start + `renderer_generation`; no formal controller type | **Closed (v1)**; formal controller → **Deferred (DR-028)** |
| A5 | Multi-doc first-class (DR-010 / DR-024) | Tabs Accepted (DR-024 v2); multi-window / multi-view open | **Closed (v1)** tabs; multi-window → **Open** (not DR-029) |
| A6 | GPU-first + CPU ref + tiling (DR-006) | CPU composite ref + parity fixtures; tiling ungated only with evidence | CPU ref **Closed (v1)**; tiling → **Gated (DR-029)** |
| A7 | UI toolkit | DR-023 Qt 6 + qtbridge | **Closed** |
| A8 | Fine crate split (32) | Coarse crates (DR-025); §32 = ownership map | **Closed (DR-025)** |
| A9 | Extension seams; ABI deferred (DR-009) | `extension_data` opaque seam; no host/ABI | Seam **Closed (v1)**; ABI → **Gated (DR-029)** |

### 3.2 Document / layers / engines — **F** / **A**

| # | Handbook says | Code has | Status |
| --- | --- | --- | --- |
| D1 | Rich resources / version vectors (10, 27) | Graph + ICC + `extension_data`; full resource registry depth | **Deferred (DR-028)** |
| D2 | Shape/text engines (18, 19) | Shape kinds, boolean, live vector, path-edit, text frame/wrap | **Closed (v1)**; GPU live tiles / curves → **Deferred (DR-028)** |
| D3 | Full CMS / lcms2 (16) | Assign/convert tags, soft-proof, embed, display discovery | Soft-proof **Closed (v1)**; lcms2 → **Deferred (DR-028)** |
| D4 | Filter plans + gallery (15) | `FilterPlan` + gallery + cancel + GPU pack | **Closed (v1)**; fuller adjustment set → **Deferred (DR-028)** |
| D5 | Object vs pixel vs edit target (DR-011) | Distinct chrome + status; multi-object polish residual | **Closed (v1)**; multi-object → **Partial** |
| D6 | Vector masks / refine / apply (13) | Vector mask create + apply/refine attrs; deep path-edit residual | **Partial** |
| D7 | History retention + spill (20) | Retention UI shipped; spill-to-disk | Retention **Closed (v1)**; spill → **Gated (DR-029)** |
| D8 | `.ptx` integrity + sparse (27) | v2 write / v1 read + diagnostics; sparse/incremental | Integrity **Closed (v1)**; sparse → **Gated (DR-029)** |

### 3.3 Shell / UX systems — **F**

| # | Handbook says | Code has | Status |
| --- | --- | --- | --- |
| U1 | Dock tear-off / auto-hide / topology (04) | Shipped + persisted; topology v2 adds tab groups with a v1 migration | **Closed (v1)** |
| U2 | Panel descriptors (05) | `panels_json` + visibility/titles; Paths/Character body depth | **Closed (v1)**; panel body parity → **Partial** |
| U3 | Registry-driven tools (06) | `tools_json` shelf in conventional order + overflow; tools registered as actions with letter keys; `ToolOptionsBar.qml` | **Closed (v1)** |
| U4 | Context menus from actions (07) | Layer/canvas/selection/mask from registry | **Closed (v1)**; path context / selection-preserve → **Partial** |
| U5 | Customizable shortcuts (09) | Action map + conflict UI + persist + yield | **Closed (v1)** |
| U6 | Themes / density / contrast (25) | `Theme.qml` packs; `densityScale` drives spacing, control heights, hit targets, and chrome extents (not type alone) | **Closed (v1)**; full token audit → **Partial** |
| U9 | Progressive disclosure, four levels (01, 28) | `default_disclosure_groups()` registry → `DisclosureGroup.qml`; Properties regrouped from a flat `visible:` stack; expansion persisted as prefs schema 7 `disclosure_open`; all ten groups carry a collapsed summary; badges derived host-side by `inspector_badges()`; layout order asserted against the registry; expand/collapse-all on the panel header **and** in the View menu | **Closed (v1)** |
| U7 | Dialogs / palette / workspace presets (03, 26) | Palette + builtins + last-saved + **user-named save/delete** | **Closed (v1)** |
| U8 | A11y tree + AT-SPI (29) | Semantic JSON + projection + Qt Accessible | **Closed (v1)**; custom D-Bus tree → **Deferred (DR-028)** |

### 3.4 I/O / formats / clipboard — **F** / **C**

| # | Handbook says | Code has | Status |
| --- | --- | --- | --- |
| I1 | Hard limits + loss disclosure (22) | Dimension/byte limits; PSD truncation messages; codec set PNG/JPEG/WebP/TIFF/BMP/GIF | Limits **Closed (v1)**; broader loss reports → **Partial** |
| I2 | Capability clipboard (21) | RGBA + selection/mask + OS image | **Closed (v1)**; SVG/layer MIME → **Deferred (DR-028)** |
| I3 | Recovery ~60s + restore UX (02) | Autosave + startup chooser + safe-start | **Closed (v1)** |

### 3.5 Performance / testing / process — **C** / **P**

| # | Handbook says | Code has | Status |
| --- | --- | --- | --- |
| P1 | Promote Provisional budgets with fixtures (DR-017, 30) | Soft CI + Tier M CPU proxies Accepted; interactive present still Provisional | **Partial** |
| P6 | UI thread MUST NOT wait for GPU completion (28, Thread Ownership Map) | Composite submits and returns; shared-queue barrier orders present ([DR-030](Decision-Register.md#dr-030--shared-queue-barrier-not-host-wait-orders-composite-and-present)); paint path no longer stalls every fourth dab | **Closed (v1)** |
| P7 | GPU budgets measured as GPU time (30, DR-017) | `TIMESTAMP_QUERY` around the blend pass ([DR-031](Decision-Register.md#dr-031--gpu-gates-measured-by-timestamp-query)); 10×4K = 1.79 ms GPU | Composite **Closed (v1)**; input→present latency → **Provisional** (stroke metric is input→submit) |
| P5 | Cold boot < 1,000 ms gate (ADR-008, 30) | Self-reported phase timings; ~558 ms to first interactive frame after deferring fontconfig and unopened presentation surfaces; composition recorded in [Performance-Budget-Ledger](Performance-Budget-Ledger.md#b3--startup-and-restore) | Gate **Closed (v1)**; 250 ms stretch → **Open** (bounded by ~190 ms Qt/QML module floor) |
| P2 | Headless command conformance (DR-022) | `command_conformance` suite | **Closed (v1)** |
| P3 | Decision Register vs archived ADRs | [Archived-ADR-to-DR-Map.md](Archived-ADR-to-DR-Map.md) + live DRs | **Closed (v1)** |
| P4 | Handbook under `internal_docs/` | Normative tree; root bridges; former `/docs/` removed | **Closed (v1)** |

---

## 4. Code / archive items once contradictory — resolved

| Item | Resolution |
| --- | --- |
| Qt 6 + qtbridge | **DR-023 Accepted**; DR-008 superseded |
| Zero-copy interactive present | Kept; CPU = tests/degraded only |
| Single-doc → multi-doc tabs | **DR-024 v2 Accepted** (tabs); multi-window still open |
| `.ptx` bytes | **DR-026 Accepted**; sparse gated DR-029 |
| Shape layer kind | Shipped under DR-027 / DR-020 |
| Crate topology | **DR-025** coarse; §32 ownership map |
| Root `SPEC` / `CONSTRAINTS` / `AGENTS` | Non-normative bridges → handbook + DR |
| Production checklist | [Handbook-Parity-Checklist.md](Handbook-Parity-Checklist.md) |

---

## 5. Decision conflicts — closed (historical)

Alignment choices from 2026-07-16 are **done**. Do not reopen toolkit, `.ptx`, or crate topology. Live gates:

| Topic | Binding DR | Note |
| --- | --- | --- |
| Tiling / pyramid / brush tile planner | DR-029 + DR-006 | Evidence before impl |
| History spill | DR-029 + DR-004 | Memory-pressure evidence |
| Sparse / incremental `.ptx` | DR-029 + DR-026 | Spike before freeze |
| Plugin ABI / marketplace | DR-009 / DR-029 | Seams only; no product need |
| Engine chapter depth (curves, lcms2, …) | DR-028 | Per-engine milestones |
| Present-path FPS / cold-boot promotion | DR-017 | Device evidence packs |

---

## 6. Open backlog (ungated, ranked)

Prefer these over gated P11/P12 work. Sync checklist checkboxes when closing a row.

| Priority | Gap / checklist | Why | Suggested slice |
| --- | --- | --- | --- |
| **1** | U3 tool-options ↔ edit-target | Mask vs layer still easy to miss while painting | Mirror edit-target chip on options bar |
| **2** | U2 Paths / Character body parity | Navigator has a body; Paths/Character still thin vs descriptors | One real Paths list or Character completeness pass |
| **3** | D6 vector-mask path-edit depth | Create/apply exist; deep edit residual | Path-edit ↔ vector mask round-trip |
| **4** | I1 / P8 codec + loss-report polish | Codecs already in `phototux_io`; broaden disclosure UX | Structured loss dialog for PSD/export |
| **5** | P1 DR-017 evidence | Soft CI green; present budgets Provisional | Release cold-boot + zoom/pan FPS pack |
| **6** | Assign ≠ convert disclosures | Distinct actions exist; handbook wants consequence copy | ToolTips / announce strings on Image menu |
| **7** | Font discovery is lazy and blocking | The missing-font badge cannot fire before `fc-list` has run, so a text layer whose family is uninstalled shows no warning until the Character body is first built | Discover off the UI thread after first frame, then re-evaluate the badge |
| **8** | Combo drop-down lists are light-on-dark (T-031) | No Controls style is configured, so the shell runs Basic, which hardcodes a light palette. Field chrome is themed; the popup is not, because overriding `popup` + `delegate` left the row at `currentIndex` blank in every combo — the list reserved its slot but painted neither label nor highlight, through both DelegateModel access and direct array indexing | Understand why the current row does not resolve in a hand-rolled popup, or evaluate adopting a palette-aware style (Fusion) for the whole shell — the latter is a broad visual change and wants a DR |
| **9** | `LayerMask::feather` affects no pixel | The field is stored, clamped by `clamp_refine`, recorded in undo by `mask.set-attributes`, and bound to a QML property — and `feather` appears nowhere in `phototux_gpu`. Unlike density/contrast/shift it is a neighbourhood operation, so `LayerMask::coverage` cannot express it per sample. The engine already has `feather_mask_r8`, used for selections | Either apply `feather_mask_r8` to the mask on both the composite and bake paths, or remove the field and its QML property. Do not leave it half-present |
| **10** | No CPU↔GPU parity test for mask semantics | `LayerMask::coverage` is now the single definition and is covered headlessly, but nothing proves the WGSL block in `composite.rs` still agrees with it. The existing `parity.rs` compares blur/sharpen only | A device-backed case under `--features gpu-tests` that composites a masked layer across a `{density, contrast, shift, inverted}` sweep and compares against `coverage` |
| **11** | A layer with both a drop shadow and an outer glow loses the glow | Both reach the renderer through one `drop_shadow` slot — a glow is a shadow at zero offset — so the drop shadow wins and the glow is silently dropped. Now visible in `phototux_engine::LayerRenderPlan`, where the precedence is documented and pinned by a test, rather than buried in the GPU crate | Represent the two separately in `LayerRenderPlan` and give the renderer a second shadow pass. Not local to the plan, which is why it was recorded rather than patched |
| — | P11 tiling / spill / sparse | Large-doc / memory evidence | **Do not start** (DR-029) |
| — | P12 plugin host | No product need | **Do not start** (DR-029) |

**Closed this refresh:** U7 user-named workspace presets (Preferences Save/Delete; prefs schema 6); U9 progressive disclosure (summaries, host-derived badges, registry-order conformance, expand/collapse-all); right-dock height distribution (T-024 / T-026 — tabbed dock groups show one panel per group, so the collapse-to-header case no longer arises); dialog and control contrast (T-029).

---

## 7. How to keep this file honest

1. After each parity slice: update the matching row status here **and** [Handbook-Parity-Checklist.md](Handbook-Parity-Checklist.md).
2. When promoting a Provisional budget or Deferred depth: amend Decision Register first, then close the row.
3. Do not mark gated rows Closed without DR-029 evidence.
4. Prefer measured shipping code + promoted DR over silent drift (AGENTS.md).

---

## 8. Owner decisions (resolved 2026-07-16; still binding)

1. **Qt 6 + qtbridge** → **Accepted** ([DR-023](Decision-Register.md#dr-023--tech-stack-frozen-to-shipping-codebase)).
2. **`.ptx`** → **Accepted** ([DR-026](Decision-Register.md#dr-026--native-ptx-container-v1)); sparse gated.
3. **Document session** → **DR-024 v2 tabs Accepted** (2026-07-17); multi-window open.
4. **Zero-copy interactive present** stays hard; CPU = tests/degraded only.
5. Product tracker → [Handbook-Parity-Checklist.md](Handbook-Parity-Checklist.md); alignment history → [Implementation-Checklist.md](Implementation-Checklist.md).

---

## 9. Cross references

- Handbook index: [README](../README.md)
- Decision Register: [Decision-Register.md](Decision-Register.md)
- Parity roadmap: [Handbook-Parity-Roadmap.md](Handbook-Parity-Roadmap.md)
- Archived ADR map: [Archived-ADR-to-DR-Map.md](Archived-ADR-to-DR-Map.md)
- Developer guide: [32-Developer-Guide.md](../32-Developer-Guide.md)
- Interactive stability: [Interactive-Stability-Checklist.md](Interactive-Stability-Checklist.md)
