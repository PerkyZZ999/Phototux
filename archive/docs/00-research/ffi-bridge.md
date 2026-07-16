# Research: Rust ↔ Qt FFI Bridge

## Candidates

### qtbridge-rust (Qt Bridges / official)

- **Version evaluated**: `qtbridge` **0.2.0** (crates.io); repo `qt/qtbridge-rust` (public beta)
- **Maturity**: **Beta / pre-release** (announced 2025–2026). ~197 GitHub stars; active Qt Group development
- **Community**: Official Qt path; forum category “Qt Bridges”
- **License**: Qt Bridges pre-release terms for commercial Qt licensees; OSS usage per repo LICENSES + CXX MIT/Apache
- **Requirements**: Rust ≥ 1.87, **Qt ≥ 6.10**, `qmake` on PATH, C++ toolchain. Host: Qt **6.11.1**, rustc **1.95** → **compatible**
- **Compatibility with constraints**:
  - Pure Rust QObjects/`#[qobject]`/`#[qslot]`/`#[qproperty]`: **Pass**
  - No C++ for app logic shell: **Pass** (documented)
  - Custom QQuickItem / private RHI APIs: **Unknown / likely Fail pure** — docs point to **CXX-Qt** when C++ API modules needed
- **Performance**: Control path only (commands/properties). Ownership: `Rc<RefCell<_>>` with runtime borrow rules (panic on conflict)
- **Learning curve**: Low for QML-bound backend; medium for threading (`QmlMethodInvoker` + tokio examples)
- **Vendor lock-in**: Medium (official but beta API churn)
- **Pros**: Matches SPEC; zero manual C++ for chrome; Cargo-first; models traits; serde_json feature
- **Cons**: Beta; Pre-Release Code warnings; custom Scene Graph items may force hybrid; future plans mention CXX-Qt interop (not done)
- **Risk level**: **High for Phase 2 canvas**, Low–Med for Phase 1 shell

### CXX-Qt (KDAB)

- **Version evaluated**: **0.9.1** (Jul 2026)
- **Maturity**: Multi-year; KDAB-backed; book + examples; Cargo + CMake
- **Community**: Established; comparison table vs qmetaobject on README
- **License**: MIT/Apache-style (check crate)
- **Compatibility**:
  - Qt 5.15 + all Qt 6: **Pass**
  - QObject in Rust, QML: **Pass**
  - Mix C++ (QQuickRhiItem subclass): **Pass** — designed for this
- **Pros**: Production-ish for embedded/Qt shops; clear thread story; can implement custom items in C++ bound to Rust
- **Cons**: More C++/CMake surface; not “official”; slightly more ceremony than qtbridge macros
- **Risk level**: Medium

### qmetaobject-rs

- **Version evaluated**: crates.io (maintenance mode relative to peers)
- **Maturity**: Older pure-Rust QML approach (`cpp!` macros)
- **Compatibility**: QML yes; Widgets no; incomplete type coverage
- **Pros**: No separate C++ project historically
- **Cons**: Incomplete APIs; less active vs CXX-Qt/qtbridge; harder long-term
- **Risk level**: High (stagnation)

### Manual CXX / bindgen over Qt C++

- **Maturity**: DIY
- **Pros**: Full control for RHI item
- **Cons**: Huge surface; safety burden
- **Risk level**: High

## Compatibility Matrix

| Candidate | Pure Rust shell | Qt 6.11 host | Custom RHI item path | Official | Risk |
|-----------|-----------------|--------------|----------------------|----------|------|
| qtbridge 0.2 | Pass | Pass | Unproven | Yes (beta) | High (canvas) |
| CXX-Qt 0.9.1 | Pass (with gen) | Pass | Pass | No (KDAB) | Medium |
| qmetaobject | Pass | Pass | Hard | No | High |
| Manual CXX | Fail ergonomics | Pass | Pass | N/A | High |

## Recommendation

**Primary:** `qtbridge` 0.2 for Phase 1 UI/logic (SPEC + host fit).  
**Fallback / hybrid:** Isolate canvas as thin C++ `QQuickRhiItem` (or CXX-Qt module) if qtbridge cannot register custom items. Do **not** rewrite whole app to qmetaobject.

## Open Questions

1. Can qtbridge register a custom QML type implemented in C++ / as `QQuickItem`?
2. Planned CXX-Qt interop timeline vs our Phase 2?
3. Pin exact `qtbridge` crate version + changelog stability for 0.2.x?
