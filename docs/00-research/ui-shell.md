# Research: UI Shell / Frontend

## Candidates

### Qt 6.11 + QML (Qt Quick) + Kirigami/Plasma styling

- **Version evaluated**: Qt 6.11.1 (host), QML modules via `qt6-declarative`
- **Maturity**: Production, 15+ years QML; Plasma 6 stack mature
- **Community**: Dominant Linux desktop toolkit for dense pro apps
- **License**: LGPLv3 / commercial (system packages on Arch OK for OSS)
- **Compatibility with constraints**:
  - Linux Wayland: **Pass** — first-class
  - KDE HIG dense dark multi-pane: **Pass** — native alignment
  - Zero-copy GPU path via RHI: **Pass** — `QQuickRhiItem` / Scene Graph
- **Performance**: Scene Graph + RHI (Vulkan) suitable for high-refresh compositing of UI chrome; canvas is separate
- **Learning curve**: Medium if QML known; high for custom RHI items
- **Vendor lock-in**: Medium–Hard (QML investment)
- **Pros**: Docks, toolbars, menus, portals ecosystem; matches SPEC vision
- **Cons**: Heavy dep tree; private headers sometimes needed for advanced RHI
- **Risk level**: Low for shell; Medium for custom canvas item

### GTK 4 + libadwaita

- **Version evaluated**: GTK 4.x / Adwaita
- **Maturity**: Production GNOME stack
- **Community**: Large on Linux
- **License**: LGPL
- **Compatibility**:
  - KDE-native dense shell: **Fail** — GNOME patterns, not Plasma HIG
  - Hard constraint “KDE alignment”: soft-fail on product vision
- **Pros**: Excellent Wayland; `gtk-rs` mature
- **Cons**: Wrong desktop aesthetic; custom GPU canvas still hard
- **Risk level**: Medium (product mismatch)

### iced / egui / Slint (pure Rust UI)

- **Version evaluated**: iced 0.13-class, egui, Slint commercial/OSS
- **Maturity**: Growing; not KDE-native shells
- **Compatibility**:
  - KDE multi-pane HIG: **Fail**
  - Zero-copy into Qt RHI: **N/A** (different stack)
- **Pros**: Pure Rust; simpler build
- **Cons**: Violates SPEC shell pillar; rebuild all chrome
- **Risk level**: High for this product

## Compatibility Matrix

| Candidate | Wayland | KDE shell | Custom GPU canvas path | Reversibility | Risk |
|-----------|---------|-----------|------------------------|---------------|------|
| Qt 6 QML  | Pass    | Pass      | Pass                   | Hard          | Low–Med |
| GTK4/Adw  | Pass    | Fail      | Partial                | Hard          | Med |
| iced/egui/Slint | Pass | Fail   | Own path               | Medium        | High |

## Recommendation

**Qt 6 QML** only viable match for hard/soft constraints. Prefer Plasma-aligned styling (Breeze dark, dense spacing) over Kirigami mobile patterns.

## Open Questions

1. Kirigami vs pure Qt Quick Controls 2 for desktop-dense layout?
2. Does `qtbridge` expose enough for custom `QQuickItem`, or need C++/`cxx-qt` for canvas only?
