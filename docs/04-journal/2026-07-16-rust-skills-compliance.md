# Journal: Rust skills compliance pass (2026-07-16)

## Scope

Align first-party crates with AGENTS.md Rust skill set: `ms-rust`, `rust` / `rust-optimise`, `rust-skills`, `rust-reference` (unsafe), `rust-doctor`.

## Changes

- **Typed errors:** `DocumentError` (`thiserror`) replaces stringly `Result<_, String>` on graph add paths; UI maps via `Display`.
- **No lib-path panics:** removed production `.expect()` from `.ptx` / PSD parsers and file-worker PSD placeholder; fallible byte readers + `Result`.
- **Unsafe hygiene:** split multi-op Vulkan HAL borrow into single-op `unsafe` blocks with `// SAFETY:`; documented recovery test env overrides.
- **Ownership / hot path:** `HistoryKind: Copy`; reuse encode buffers in `.ptx` asset loop; selection mask uses `get_mut` instead of indexing.
- **Lint policy:** `#[expect(..., reason = "...")]` for intentional channel/posterize casts; float compare via epsilon in color recent list.
- **Codecs:** dropped `unreachable!` in raster encode by per-format helper arms.

## Verification

`./scripts/check-rust.sh` green (fmt, clippy `-D warnings`, rust-doctor errors = 0).

## Residual (accepted / deferred)

- Pedantic doctor warnings (`must_use_candidate`, `missing_const_for_fn`, cast noise on UI i32 bridges) remain non-gating; many already covered by crate `clippy.toml` test allowances.
- `multiple_crate_versions` / geiger timeouts stay in `rust-doctor.toml` ignore / known residual list.
- Steady-state `eprintln!` in UI/GPU status paths: migrate to `tracing` when the logging stack is introduced workspace-wide.
