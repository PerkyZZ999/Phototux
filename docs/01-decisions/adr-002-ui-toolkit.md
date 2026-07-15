# ADR-002: UI Toolkit — Qt 6 QML

## Status

Accepted

## Context

Need dense multi-pane editor chrome (toolbars, docks, properties, menus) aligned with KDE Plasma 6.

## Devil's advocate

**Case for GTK4/libadwaita:** Stronger pure-Rust (`gtk-rs`) story on Linux.  
**Case for iced/Slint:** Simpler build, pure Rust.  
**Hidden cost of Qt:** Huge dep; LGPL packaging care; private headers for advanced RHI.  
**Failure mode:** QML Scene Graph fights custom canvas item → months lost.  
**Reversibility:** Hard once QML UI large.

**Defense:** Hard product constraint for KDE-native shell. Alternatives fail HIG alignment. Custom canvas risk accepted under ADR-003 hybrid.

## Options Considered

### Option 1: Qt 6 QML (Qt Quick Controls 2)

- **Pros**: Docks/menus/ecosystem; RHI; Plasma look
- **Cons**: Heavy; FFI required
- **Reversibility**: Hard

### Option 2: GTK4 + libadwaita

- **Pros**: gtk-rs; Wayland native
- **Cons**: Wrong desktop language for KDE-dense pro layout
- **Reversibility**: Hard

### Option 3: Pure Rust (iced / egui / Slint)

- **Pros**: No Qt FFI
- **Cons**: Rebuild chrome; no KDE HIG
- **Reversibility**: Medium

## Decision

**Option 1.** Qt **6.10+** (host 6.11.1). Prefer **Controls 2** dense desktop patterns; Breeze-dark inspired palette. Kirigami only if a desktop pattern needs it — not mobile-first.

## Consequences

- **Positive**: Matches SPEC vision; system Qt on Arch
- **Negative**: Bridge complexity; packaging weight
- **Neutral**: QML design system can be themed

## Revisit Date

End of Phase 1 (if shell ergonomics fail) or major Qt 7 migration.

## Dependencies

- **Depends on**: ADR-001
- **Blocks**: ADR-003, ADR-006

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| | | |
