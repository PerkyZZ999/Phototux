# ADR-007: Threading & Command Queue

## Status

Accepted

## Context

qtbridge holds QObjects in `Rc<RefCell<_>>`. Long mut borrows and GPU work on UI thread risk jank and panics. Latency SLO <8 ms needs isolated stroke path eventually.

## Devil's advocate

**Case for all-on-UI-thread:** Simpler Phase 1; no queues.  
**Hidden cost:** Paint storms; RefCell panics on re-entrant QML.  
**Failure mode:** 4K brush freezes UI.  
**Reversibility:** Medium if commands already message-shaped.

## Options Considered

### Option 1: UI thread only

- **Pros**: Simple
- **Cons**: Won't hit SLOs later
- **Reversibility**: Easy early

### Option 2: Command queue → engine worker; UI applies light state via signals

- **Pros**: Matches latency goals; clear ownership
- **Cons**: More plumbing
- **Reversibility**: Medium

### Option 3: Full tokio as core

- **Pros**: Async I/O
- **Cons**: Easy to misuse with Qt thread affinity
- **Reversibility**: Medium

## Decision

**Option 2**, phased. Owner lock (grill R2): **G7 = B**.

- **Phase 1:** Synchronous slots OK for sliders/labels; **no** heavy work in slots. Shape APIs as commands.
- **Phase 2+:** Dedicated engine/render worker; UI sends `EngineCommand`; results/signals marshalled back via `QmlMethodInvoker` or qtbridge-safe path.
- **Phase 4:** Brush/stroke path **must** be off UI thread (worker); hard expectation for &lt;8 ms latency gate.
- **Rule:** Never `await` while holding RefCell mut borrow across QML re-entry.

## Consequences

- **Positive**: Path to SLO; fewer panics
- **Negative**: Command protocol design cost
- **Neutral**: tokio allowed for file I/O later, not for Scene Graph

## Revisit Date

Start of Phase 4 brush work (must be on worker by then).

## Dependencies

- **Depends on**: ADR-003
- **Blocks**: brush subsystem, undo transactions

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| 2026-07-15 | G7=B confirmed | Interactive grill R2 |
