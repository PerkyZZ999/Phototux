# Decision Dependency Graph

## Graph Format

```
[ADR-001: Platform Linux/Wayland]
  ├─ Forces → [ADR-002: Qt 6 QML]
  ├─ Forces → [ADR-004: wgpu Vulkan-first]
  └─ Forces → [ADR-005: Zero-copy strategy]

[ADR-002: Qt 6 QML]
  ├─ Depends on → [ADR-001]
  ├─ Forces → [ADR-003: FFI bridge]
  └─ Forces → [ADR-006: Workspace layout]

[ADR-003: qtbridge + hybrid canvas]
  ├─ Depends on → [ADR-002]
  ├─ Forces → [ADR-006: crate split ui vs canvas]
  ├─ Forces → [ADR-007: Threading / RefCell rules]
  └─ Blocks → [ADR-005 full validation]

[ADR-004: wgpu]
  ├─ Depends on → [ADR-001]
  ├─ Forces → [ADR-005 interop approach]
  └─ Forces → [ADR-008 composite measurement]

[ADR-005: Zero-copy]
  ├─ Depends on → [ADR-002, ADR-003, ADR-004]
  └─ Blocks → Phase 2 exit / ADR-008 composite gate

[ADR-006: Workspace layout]
  ├─ Depends on → [ADR-003, ADR-004]
  └─ Forces → [ADR-009 test layering]

[ADR-007: Threading]
  ├─ Depends on → [ADR-003]
  └─ Blocks → Phase 4 brush path

[ADR-008: Performance SLOs]
  ├─ Depends on → [ADR-004, ADR-005]
  └─ Forces → phase exit checklists

[ADR-009: Testing & profiling]
  ├─ Depends on → [ADR-006, ADR-008]
  └─ Standalone tooling choices

[ADR-010: Interop spike before Phase 2]
  ├─ Depends on → [ADR-003, ADR-004, ADR-005]
  ├─ Informs → [ADR-003 hybrid vs pure, ADR-005 path choice]
  └─ Blocks → Phase 2 production canvas “done”
```

## Load-Bearing vs. Reversible Decisions

### Load-Bearing (Hard to Reverse)

| ADR | Decision | Reversal Cost |
|-----|----------|---------------|
| ADR-001 | Linux/Wayland v1 | High — product identity |
| ADR-002 | Qt 6 QML | High — full UI rewrite |
| ADR-004 | wgpu engine | High — shader/engine rewrite |
| ADR-005 | Zero-copy only | High — architecture identity |

### Reversible (Can Pivot Without Major Pain)

| ADR | Decision | Reversal Cost |
|-----|----------|---------------|
| ADR-003 | qtbridge vs hybrid canvas detail | Medium — isolate canvas crate |
| ADR-006 | Exact crate names | Low–Med |
| ADR-007 | Sync Phase 1 → worker later | Medium if APIs are commands |
| ADR-009 | Tracy vs puffin | Low |

## Critical Path

1. ADR-001 Platform  
2. ADR-002 UI + ADR-004 GPU (parallel)  
3. ADR-003 FFI  
4. ADR-005 Zero-copy  
5. ADR-006 Layout + ADR-007 Threading  
6. ADR-008 SLOs + ADR-009 Testing  

## Grill ritual checklist

- [x] No ADR contradicts hard constraints (Linux, Rust, Qt QML, zero-copy)
- [x] MVP Phases 1–2 still achievable (shell + viewport; interop risk explicit)
- [x] Hybrid FFI softens “no C++” to “no C++ for app logic”
- [x] Spike skipped → risk logged on ADR-005
- [x] All Status = Accepted
