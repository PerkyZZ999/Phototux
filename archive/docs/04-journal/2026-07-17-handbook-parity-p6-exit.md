# Handbook Parity P6 exit — Color & rendering contracts

**Date:** 2026-07-17  
**Status:** Met (exit)

## Shipped

- ICC embed foundation: `DocumentColorState.embedded_icc`; `validate_icc_profile` (empty / 4 MiB / `acsp`); `document.set-icc`
- Persist via `.ptx` graph JSON; PNG export embeds `iCCP` when present (JPEG skips embed for Met)
- UI: Embed/Clear ICC + Properties status (`hasEmbeddedIcc`); actions `action.image.embed-icc` / `clear-icc`
- GPU↔CPU parity: `phototux_gpu::parity` fixtures (Normal/Multiply/Screen; gaussian structural; sharpen); device path behind `--features gpu-tests`
- Device/surface loss: `GpuError::DeviceLost` / `SurfaceLost`; wgpu lost callback; `renderer_generation`; canvas reject-while-lost; status “Graphics device lost — document preserved”; `recoverGpu` / `action.app.recover-gpu`

## Deferred (explicit, DR-028 / P11)

- Linux display ICC / colord discovery
- Dense immutable pixel blob + delta publisher (leases remain)
- Soft-proof GPU display transform beyond tags
- Full lcms2 CMS pipeline
- Dirty-region / overlay polish
- Tiling / pyramid (P11)
- Full Event-Catalog `DeviceLost` lifecycle orchestration (P7 depth); injected CI device-loss without device

## Evidence

- Engine: ICC validate/serde; `document_set_icc_embeds_and_clears`
- I/O: `png_embeds_icc_profile`; `ptx_roundtrip_preserves_embedded_icc`
- GPU: parity CPU tests always; `gpu-tests` blend/gaussian/sharpen; device-loss recover bumps generation
- Canvas: `simulate_device_loss_recover_restores_composite`
- `./scripts/check-rust.sh` green on exit commit
- Checklist / Roadmap / Command-Taxonomy / gap-analysis updated

## Next

Ungated: **P8** clipboard & interchange I/O depth (see Roadmap §7).
