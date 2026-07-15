# Research: Backend Engine Architecture Patterns

## Decision point

How to structure image state, undo, and thread boundaries between QML UI thread and GPU work.

## Candidates

### Document graph + command DAG (non-destructive)

- Layer nodes, blend ops, effects as graph; undo = graph transactions / reverse ops
- **Pros**: Matches Phase 3; memory-efficient undo via structural sharing possible
- **Cons**: More design up-front
- **Risk**: Medium

### Classic raster stack + command pattern undo

- Mutable layer bitmaps; undo stores tiles/diffs
- **Pros**: Simple MVP; easy mental model
- **Cons**: Harder non-destructive later; memory spikes
- **Risk**: Low short-term, Medium long-term

### ECS (e.g. bevy_ecs) for tools/layers

- **Pros**: Flexible tools
- **Cons**: Overkill; Qt already owns UI lifecycle
- **Risk**: High complexity

## Threading model candidates

| Model | UI thread | Engine | GPU |
|-------|-----------|--------|-----|
| A. UI-thread engine | All sync | Same | Submit only | Simple; risk jank |
| B. Engine worker + command queue | Events only | Worker | Worker submits | Best latency isolation |
| C. tokio multi + invoker | Async I/O | Mixed | Careful | Fits qtbridge examples |

**Recommendation lean:** B for stroke path; light properties sync on UI via qtbridge signals. Avoid holding `RefCell` mut borrows across awaits.

## Persistence / file formats (later)

Out of MVP: OpenRaster, PSD-like, custom `.phototux`. Research deferred to Phase 5-adjacent.

## Testing strategy candidates

| Layer | Approach |
|-------|----------|
| Pure Rust graph/undo | `cargo test`, proptest |
| Shaders | Golden image GPU tests (optional CI GPU) |
| QML shell | Manual + later Qt Test / screenshot |
| Perf SLOs | In-app HUD + Tracy frame marks |

## Observability

- **Tracy** or **puffin** for CPU frames
- wgpu timestamp queries for composite budget
- Structured logs via `tracing`

## Recommendation

- **Phase 1–2:** Mutable canvas texture + camera (pan/zoom); no full graph yet
- **Phase 3:** Document graph + transactional undo
- **Threading:** command queue off UI thread before brush work
- **Obs:** `tracing` + Tracy hooks early

## Open Questions

1. Tile-based vs full-texture layers for 4K×N?
2. Undo granularity: stroke vs dab?
