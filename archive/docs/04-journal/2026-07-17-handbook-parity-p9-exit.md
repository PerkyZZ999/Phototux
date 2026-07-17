# Handbook Parity P9 exit — Preferences, themes, UX polish

**Date:** 2026-07-17  
**Status:** Met (exit)

## Shipped

- Effective preference spine: `resolve_layered` / `PrefSource` + `prefEffectiveJson`
- Safe-start: `Preferences.safe_start_next`, Preferences checkbox, `PHOTOTUX_SAFE_START=1` (essentials layout, clear keymap, skip last-tool restore)
- Mixed-value inspector: opacity/blend show **Mixed** when multi-select disagrees
- Progressive disclosure: Properties “advanced color” (soft-proof / ICC) behind toggle

## Deferred

- Full handbook preference schema / transaction engine
- Complete reduced-motion + 200% scale evidence audit
- Field-level reset for every preference domain

## Evidence

- Engine tests: precedence + mixed detection
- Prefs test: safe-start chrome
- `./scripts/check-rust.sh` green on exit commit

## Next

Ungated: **P10** AT-SPI host adapter (semantic tree already exists).
