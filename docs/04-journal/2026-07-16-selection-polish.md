# Journal: Selection Polish Slice (2026-07-16)

## Scope

Wire GPU `SelectionMask` into the live session; rectangular + elliptical marquee with combine modes, QML marching ants, drag preview, and selection undo/redo.

## Delivered

- **Engine:** `SelectionShape`, ellipse setters, combine parse, bounds union/intersect for outline metadata.
- **GPU:** `SelectionMask::apply_ellipse`, CPU snapshot/restore; unit tests for rect/ellipse/combine.
- **Canvas:** `DocGpu` owns selection mask; FFI for apply/clear/select-all/invert/snapshot/restore.
- **UI:** Selection props (bounds/shape/combine/preview); mask undo stack; slots for rect/ellipse/invert/combine/preview.
- **QML:** Ellipse tool, animated ants + drag preview, Properties combine toggles, Shift/Alt modifiers, Invert Selection menu.

## Out of scope (follow-ups)

- Lasso / polygonal / magnetic
- Feather / expand / contract
- Brush clipped to selection
- Masked clipboard crop
- Selection channel in `.ptx`
- GPU fragment edge-ants (needed for irregular masks)

## Verification

`./scripts/check-rust.sh` green; `cargo test -p phototux_engine selection`; `cargo test -p phototux_gpu selection`.
