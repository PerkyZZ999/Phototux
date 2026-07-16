# Journal: Lasso + GPU Edge Ants Slice (2026-07-16)

## Scope

Polygonal and freehand lasso selection (shared even-odd polygon rasterize + combine/undo) plus GPU marching ants that sample the selection mask edge in the canvas present shader.

## Delivered

- **Engine:** `SelectionShape::Mask`, `polygon_bounds`, `set_mask_polygon`; `tool.select.polygon`.
- **GPU:** `SelectionMask::apply_polygon` (even-odd), `texture_vk_handle` for canvas import.
- **Canvas:** `selection_apply_polygon`, selection VkImage export; second R8 texture binding; fragment edge detect + animated ants; `selectionAnts` QML property.
- **UI/QML:** Lasso + polygonal tools, live path preview, Enter/double-click/Esc; QML Shape ants hidden when shape is `mask`.

## Out of scope

Magnetic lasso, magic wand, selection-clipped paint, clipboard crop to selection, `.ptx` selection channel, unifying rect/ellipse onto GPU ants.

## Verification

`./scripts/check-rust.sh` green; `cargo test -p phototux_engine shape_parse_mask polygon_bounds`; `cargo test -p phototux_gpu polygon`.
