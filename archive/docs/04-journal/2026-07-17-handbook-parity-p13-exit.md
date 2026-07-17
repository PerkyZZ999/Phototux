# Handbook Parity P13 exit — Verification & budget promotion

**Date:** 2026-07-17  
**Status:** Met (verification spine; interactive budgets still Provisional)

## Shipped

- Command-router conformance (prior): `command_conformance`
- Budget fixture harness: `phototux_engine::budget_harness` + soft CI suite
- Soft-gate promotions (ledger): B2-proxy CPU composite, B9 history retention, B1-proxy command invoke
- GPU device-loss suite: `phototux_gpu` loss/recover generation tests + skip matrix in ledger
- CPU↔GPU tolerance fixtures: `phototux_gpu::parity` (`gpu-tests` feature)
- Thread ownership map: implementation table for shipping crates
- A11y evidence spine: semantic + AT-SPI projection JSON (full bus → DR-028)

## Still Provisional / gated

- Interactive present budgets (B1/B2 photon), boot (B3), large-doc (B5 → P11 gate)
- Hostile I/O fuzz corpus (dimension/alloc unit tests remain)
- Full AT-SPI evidence pack on real AT clients

## Evidence

- `cargo test -p phototux_engine budget_harness`
- `./scripts/check-rust.sh` green on exit commit
- Ledger § Soft CI gates; DR-017 amendment

## Next

Ungated: **P7** retention budget UI + safe-start checklist close (spill gated).
