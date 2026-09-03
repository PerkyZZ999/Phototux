#!/usr/bin/env bash
# Build both static sites, so a broken one cannot ship unnoticed.
#
# The landing page shipped for a day with two malformed <img> tags — a stray
# `/` in the middle of the attribute list — and neither `rust-tc doctor` nor
# `check-docs-links.py` had anything to say about it, because nothing in the
# Rust gate compiles Astro and the link checker reads markdown sources rather
# than building the site. The only signal was `pnpm build`, which nobody ran.
#
# Usage:
#   ./scripts/check-web.sh          # build both sites
#   ./scripts/check-web.sh --quiet  # exit code only
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root/web"

quiet=0
[[ "${1:-}" == "--quiet" ]] && quiet=1

if ! command -v pnpm >/dev/null 2>&1; then
  echo "pnpm not found — see web/README.md" >&2
  exit 127
fi

# A cold checkout has no node_modules, and `astro build` fails confusingly
# rather than saying so.
if [[ ! -d node_modules ]]; then
  [[ $quiet -eq 1 ]] || echo "installing web dependencies…"
  pnpm install --frozen-lockfile >/dev/null
fi

if [[ $quiet -eq 1 ]]; then
  pnpm build >/dev/null 2>&1
else
  pnpm build
  echo "ok both sites build"
fi
