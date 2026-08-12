@AGENTS.md

## Claude Code

Path-scoped rules: `.claude/rules/` (`paths:` frontmatter). They load when matching files are in context.

Project settings: `.claude/settings.json`. Personal overrides (gitignored): `CLAUDE.local.md`, `.claude/settings.local.json`.

Nested `CLAUDE.md` files import the sibling `AGENTS.md` in each crate and in `qml/`.
