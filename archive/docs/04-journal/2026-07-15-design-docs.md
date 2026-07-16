# Journal: Design documentation + scaffold removal — 2026-07-15

## Actions

1. Added experience/structure/visual docs:
   - `docs/DESIGN_BRIEF.md` (design-brief skill template)
   - `docs/INFORMATION_ARCHITECTURE.md` (IA skill template, desktop-adapted)
   - `docs/DESIGN.md` ([google-labs-code/design.md](https://github.com/google-labs-code/design.md) token + section format)
2. Removed early implementation scaffold so the repo is **docs-only** again:
   - `crates/`, `qml/`, root `Cargo.toml`, `Cargo.lock`, `target/`
3. Reset Phase 1 checklist items to open; Phase 0 design readiness marked complete.

## Rationale

Owner requested stronger design planning before further development. Keeping an ad-hoc QML scaffold risked locking visual debt ahead of tokens/IA.

## Next

- Human review of design docs
- Optional `npx @google/design.md lint docs/DESIGN.md`
- Re-implement Phase 1 strictly from ADRs + DESIGN.md + IA
