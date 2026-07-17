# Handbook Parity P8 exit — Clipboard & interchange I/O

**Date:** 2026-07-17  
**Status:** Met (exit)

## Shipped

- Selection mask clipboard: copy R8 coverage (+ OS grayscale preview); paste restores selection
- Layer mask clipboard: copy active layer mask; paste creates mask slot if needed and uploads R8
- Edit actions: `copy-selection-mask`, `copy-layer-mask`, `paste-selection`, `paste-mask`
- Copy with active selection prefers selection payload (Ctrl+C)
- `.ptx` integrity diagnostics: `load_ptx_with_diagnostics` / `ptx_integrity_report` (magic, version, CRC stored vs computed, hints)
- Open failure dialog shows multi-line mono report

## Deferred

- SVG / rich layer MIME negotiation
- Fuller import progress UX / broader loss reports
- Sparse / incremental `.ptx` → P11

## Evidence

- I/O tests: CRC mismatch + bad-magic integrity reports
- `./scripts/check-rust.sh` green on exit commit
- Checklist / Roadmap / Command-Taxonomy updated

## Next

Ungated: **P9** prefs/themes UX (mixed-value inspector; safe-start).
