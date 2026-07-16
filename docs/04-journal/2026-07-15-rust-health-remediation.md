# Rust Health Remediation — 2026-07-15

## Scope

First-party correctness, paint-path performance, Clippy gate, and Rust Doctor config.
Third-party / publication / pedantic API noise retained as reviewed exclusions.

## Baseline → after

| Metric | Before | After |
|--------|--------|-------|
| Rust Doctor score | 93/100 | **95/100 (Great)** |
| Doctor warnings | 538 | ~249 (mostly pedantic Clippy + dep-graph noise) |
| Workspace Clippy `-D warnings` | fail (`too_many_arguments` on `BrushStamper::stamp`) | pass |
| `./scripts/check-rust.sh` | blocked | pass |

## First-party fixes

- `StampRequest` + batched `stamp_batch` (one GPU submit per dab group).
- Incremental dirty-slice array repack (no full-layer copy every dab).
- Non-blocking GPU poll on mid-stroke stamps; wait only when compositing/presenting.
- Vulkan-only adapter gate before Qt interop.
- `MAX_LAYERS` (16) enforced in engine + compositor sync + UI add-layer.
- Paint/file workers: recoverable start/send; EndStroke propagates stamp errors.
- Camera-only sync for pan/zoom (no layer-string rebuild / GPU mutex).
- Composite/poll/undo-snapshot/export errors surfaced to UI status.
- Validated `u32`↔`i32` at Qt export boundary.
- Manifest: `publish = false`, workspace-aligned `pollster`/`cc`.

## Reviewed exclusions (`rust-doctor.toml`)

- `cargo_common_metadata`, `multiple_crate_versions` — unreleased app / dep graph noise.
- `unused-dependency` — workspace `cc`/`pollster` false positives.
- `unsafe-block-audit` — intentional FFI in canvas/main (ADR-003/005).
- Files: `**/build.rs`, spike crate — tooling/throwaway.
- Residual accepted noise: pedantic Clippy from doctor's pass; third-party
  `unsafe-dependency` / geiger timeouts / coverage skipped-pass (not in ignore allowlist).

## Validation

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `./scripts/check-rust.sh`
- `rust-doctor . --offline --json`
