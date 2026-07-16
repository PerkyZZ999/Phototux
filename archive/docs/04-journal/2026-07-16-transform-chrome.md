# Journal: Transform Chrome Slice (2026-07-16)

## Scope

Usable crop, free-transform with live preview + bake, flip H/V, and rotate canvas 90° CW — each undoable via full-document layer snapshots.

## Delivered

- **Engine:** `Affine2`, `LayerTransform` matrix helpers, `TransformSession`; `GraphCommand::SetMask` (fixes mask history misuse).
- **GPU:** Composite samples per-layer inverse affine (preview); CPU commit bake (`bake_affine_rgba`, `crop_rgba`, `flip_rgba`, `rotate_rgba_90_cw`).
- **Canvas:** `bake_layer_transform`, `crop_document`, `flip_layer`, `rotate_canvas_90_cw`, `snapshot_document_layers` / `restore_document_layers` (no `open_gpu_document` teardown for crop).
- **UI:** Transform/crop props + slots; `HistoryKind::Transform` undo/redo restores graph + pixels.
- **QML:** Crop overlay, transform handles, Properties Apply/Cancel, Image menu flip/rotate, Enter/Esc.

## Defaults

Bilinear bake; clip to document (no auto-expand); single active raster layer for free transform.

## Out of scope

Skew/distort/perspective/warp, quality picker, Move tool, Image Size dialogs, persistent non-destructive transforms after commit.

## Verification

`./scripts/check-rust.sh` green; `cargo test -p phototux_engine transform`; `cargo test -p phototux_gpu transform`.
