# Journal — Alignment Phase 4 engine depth (2026-07-16)

## Intent

Execute handbook [Alignment Roadmap](../../../internal_docs/Appendix/Alignment-Roadmap.md) Phase 4 on the frozen tech stack (DR-023): CPU reference composite, text bake, selection morph, color assign, paths, filter wave 2, layer styles — without toolkit or crate renames.

## Shipped

- `cpu_composite` — straight RGBA8 subset blends for headless fixtures
- `color_mgmt` + `document.assign-profile` command + Image menu
- Selection: `feather_mask_r8`, `expand_mask_r8`, `contract_mask_r8` + Select menu
- `text_bake` — deterministic 5×7 ASCII bake; Layer → Bake Text uploads pixels
- `paths` — `PathDocument` / stroke-to-raster; Layer → Stroke Path to Layer
- Filter wave 2: `MotionBlur`, `Emboss` params + menu (GPU shader keys stubbed)
- `layer_style` — Drop Shadow + Stroke metadata + CPU apply
- Shell descriptors: `panel.paths`, `panel.character`

## Deferred / gated

- Shape layer kind (DR-020 graph amend)
- Character panel chrome UI (descriptor only)
- Full GPU passes for new styles/filters
- `.ptx` chunk/integrity evolution (DR-026, later)
- Phase 5 tiling / multi-doc (evidence + DR-024 amend)

## Checks

`./scripts/check-rust.sh` green; `phototux_engine` lib tests 62 passed.
