# ADR-012: License & Distribution Intent

## Status

Accepted

## Context

Need a default license for repo crates and a realistic distribution story. Public open-source release is **late** (owner), not blocking private development.

## Devil's advocate

**Case for MIT-only app:** Easier corporate contribution optics.  
**Hidden cost:** Qt (LGPL) linking/distribution still requires care; “MIT” does not remove Qt obligations.  
**Case for defer forever:** License bikeshed later; risk of mixed/unstated terms in deps.

## Options Considered

### Option 1: GPL-3.0-or-later application; system Qt LGPL; public OSS late

- **Pros**: Clear product FOSS story; common for desktop editors
- **Cons**: Stronger copyleft may deter some
- **Reversibility**: Hard after many external contributions

### Option 2: MIT/Apache for first-party code only

- **Pros**: Permissive
- **Cons**: Still document Qt; may conflict with some copyleft deps later
- **Reversibility**: Medium early

### Option 3: No license until public release

- **Pros**: Delay decision
- **Cons**: Default “all rights reserved” confusion; bad for agents/contributors
- **Reversibility**: Easy until first external clone

## Decision

**Option 1.** Owner lock (grill R2): **G12 = A**.

- **First-party code:** **GPL-3.0-or-later** (workspace `license` field when crates return).
- **Qt:** System packages; respect **LGPLv3** obligations at packaging time (dynamic link typical on Arch).
- **qtbridge / other crates:** Honor each crate’s license at pin time; no license-incompatible deps without amendment.
- **Public open-source:** Intentional, **late** — after the app is real enough; until then repo may stay private. License still set in tree so the late publish is not a scramble.
- **Distribution:** AUR/distro packages and/or Flatpak **after** Phase 5-quality; not MVP.

## Consequences

- **Positive**: Clear terms; packaging path known
- **Negative**: Copyleft may limit proprietary forks
- **Neutral**: “Late OSS” is process, not a code architecture change

## Revisit Date

First public release preparation; or if a dependency forces license conflict.

## Dependencies

- **Depends on**: ADR-002 (Qt), ADR-003 (qtbridge)
- **Blocks**: public release checklist

## Amendments

| Date | Amendment | Reason |
|------|-----------|--------|
| 2026-07-15 | Accepted G12=A; OSS late | Interactive grill R2 |
