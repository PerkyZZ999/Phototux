# Journal: Mask Paint + Clipping Slice (2026-07-16)

## Scope

End-to-end layer masks (R8 GPU channel, brush/eraser paint target, composite multiply, disable/delete, `.ptx` round-trip) plus minimal Photoshop-style clipping (`clips_to_below`) in the same composite pass.

## Delivered

- **Engine:** `PaintTarget`, `Layer.clips_to_below`, `SessionState.mask_edit_layer`, `GraphCommand::SetClipsToBelow`, mask/clip flag joins for QML.
- **GPU:** `LayerMaskChannel` (R8), `MaskStamper`, composite `masks_tex` array + `has_mask` / `mask_enabled` / `mask_inverted` / `clips_to_below` uniforms; clip-base alpha in fragment loop.
- **Canvas:** `ensure_mask` / `remove_mask`, stamp by `PaintTarget`, stroke undo clones layer or mask texture, `read_all_mask_r8` / `write_mask_r8`.
- **UI/QML:** Layers dock mask badge/edit/disable, Create Clipping Mask, Properties mask controls; Save/Open/autosave persist masks.
- **IO:** `PtxDocument.masks: HashMap<u64, Raster>` with `#[serde(default)]`; grayscale as RGBA PNG (`R=G=B`); no format version bump.

## Out of scope

Vector mask, Refine Mask, Apply Mask bake, density/feather GPU blur, linked mask transform, PSD mask import, canvas mask overlay visualization.

## Verification

`./scripts/check-rust.sh` green; `cargo test -p phototux_engine mask_flags`; `cargo test -p phototux_gpu mask_multiply`; `cargo test -p phototux_io ptx_mask_roundtrip`.
