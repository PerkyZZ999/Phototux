# ADR-003: FFI Bridge — qtbridge primary, hybrid canvas allowed

## Status

Accepted

## Context

Rust backend must drive QML without traditional hand-written C++ for application logic. SPEC names `qtbridge-rust`. Custom GPU item may need C++ Qt APIs.

## Devil's advocate

**Case for CXX-Qt everywhere:** Proven 0.9.x, explicit C++ interop for `QQuickRhiItem`, KDAB support path.  
**Case for qmetaobject:** Older pure-Rust path.  
**Hidden cost of qtbridge:** Public **beta** (0.2.0); API churn; Pre-Release terms for commercial Qt; custom items **unproven**; `Rc<RefCell>` panic on re-entrancy.  
**Failure mode:** Beta blocks Phase 1 compile or Phase 2 item registration → rewrite bridge mid-project.  
**Reversibility:** Medium for logic QObjects; Hard if UI deep into qtbridge macros.

**Defense:** Official path matches SPEC and host (Qt 6.11, Rust 1.95). Phase 1 only needs QObjects. Canvas hybrid keeps escape hatch without abandoning shell.

## Options Considered

### Option 1: qtbridge only (pure)

- **Pros**: SPEC; no app C++
- **Cons**: May not support custom QQuickItem
- **Reversibility**: Medium

### Option 2: CXX-Qt only

- **Pros**: Canvas path clear; mature relative to qtbridge
- **Cons**: More C++/CMake; not SPEC first choice
- **Reversibility**: Medium

### Option 3: Hybrid — qtbridge for app logic; thin C++/cxx-qt for canvas item only

- **Pros**: Best of both; isolates risk
- **Cons**: Two bridge styles in one repo
- **Reversibility**: Medium

### Option 4: qmetaobject-rs

- **Pros**: Historical pure Rust
- **Cons**: Incomplete; weaker momentum
- **Reversibility**: Medium

## Decision

**Option 3 (hybrid strategy), with Option 1 as Phase 1 implementation.**

**Owner lock (grill 2026-07-15):** **G3 = C** — hybrid accepted as recommended.

1. **Phase 1:** `qtbridge = "0.2"` (pin minor) for windows, models, properties, tools UI.
2. **Before Phase 2 production canvas:** time-boxed interop spike (ADR-010) may prove whether pure qtbridge custom item works; if not, hybrid C++ is **expected**, not a surprise.
3. **Phase 2 canvas:** Prefer minimal C++ `QQuickRhiItem` (or CXX-Qt module) under `crates/phototux-canvas/` when qtbridge cannot own the item — **do not** spread C++ into app logic.
4. **Forbidden:** Full rewrite of shell to qmetaobject without ADR amendment; C++ outside canvas interop without amendment.

## Consequences

- **Positive**: Unblocks Phase 1 immediately; keeps zero-copy path realistic
- **Negative**: Possible dual-bridge complexity later
- **Neutral**: Future qtbridge↔CXX-Qt interop may simplify (upstream roadmap)

## Revisit Date

End of Phase 1 bootstrap; immediately if qtbridge blocks build; end of Phase 2 interop.

## Dependencies

- **Depends on**: ADR-002
- **Blocks**: ADR-005, ADR-006, ADR-007

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| 2026-07-15 | Confirmed hybrid (G3=C); canvas C++ planned capability | Interactive grill + ADR-010 spike |
