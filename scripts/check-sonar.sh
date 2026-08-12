#!/usr/bin/env bash
# PhotoTux SonarQube gate: Clippy JSON (same flags as check-rust.sh) + scanner + quality gate.
# Token: SONAR_TOKEN, else gitignored .sonar/scanner-token (never commit).
# Host:  SONAR_HOST_URL (default http://localhost:9000)
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

if [[ -t 1 ]]; then
  C_RED=$'\033[0;31m'
  C_GRN=$'\033[0;32m'
  C_YLW=$'\033[0;33m'
  C_RST=$'\033[0m'
else
  C_RED="" C_GRN="" C_YLW="" C_RST=""
fi

die() { echo "${C_RED}error:${C_RST} $*" >&2; exit 1; }
info() { echo "${C_YLW}==>${C_RST} $*"; }
ok() { echo "${C_GRN}ok${C_RST} $*"; }

[[ -f "$ROOT/Cargo.toml" ]] || die "No Cargo.toml — cannot run SonarQube analysis."
[[ -f "$ROOT/sonar-project.properties" ]] || die "missing sonar-project.properties"

export PATH="${HOME}/.cargo/bin:/usr/lib/qt6/bin:${HOME}/.local/bin:${HOME}/.local/share/sonarqube-cli/bin:${PATH:-}"

if [[ -x /usr/lib/qt6/bin/qmake ]]; then
  export PATH="/usr/lib/qt6/bin:$PATH"
  export QMAKE="${QMAKE:-/usr/lib/qt6/bin/qmake}"
fi

command -v cargo >/dev/null || die "cargo not found"
command -v cargo-clippy >/dev/null 2>&1 || cargo clippy -V >/dev/null 2>&1 \
  || die "clippy not found (rustup component add clippy)"
command -v sonar-scanner >/dev/null || die "sonar-scanner not found (install SonarScanner CLI)"

SONAR_HOST_URL="${SONAR_HOST_URL:-http://localhost:9000}"
SONAR_TOKEN="${SONAR_TOKEN:-}"
if [[ -z "$SONAR_TOKEN" && -f "$ROOT/.sonar/scanner-token" ]]; then
  SONAR_TOKEN="$(tr -d '[:space:]' <"$ROOT/.sonar/scanner-token")"
fi
[[ -n "$SONAR_TOKEN" ]] || die "No SonarQube token. Set SONAR_TOKEN or create .sonar/scanner-token (gitignored)."

info "Clippy JSON report (workspace, -D warnings)"
mkdir -p "$ROOT/target/sonar"
set +e
cargo clippy --workspace --all-targets --all-features --message-format=json -- -D warnings \
  >"$ROOT/target/sonar/clippy-report.json"
clippy_ec=$?
set -e
[[ "$clippy_ec" -eq 0 ]] || die "clippy JSON report failed (exit $clippy_ec). Fix clippy first: ./scripts/check-rust.sh"
ok "clippy report → target/sonar/clippy-report.json"

info "sonar-scanner (quality gate wait) → $SONAR_HOST_URL"
sonar-scanner \
  "-Dsonar.host.url=${SONAR_HOST_URL}" \
  "-Dsonar.token=${SONAR_TOKEN}" \
  "-Dsonar.qualitygate.wait=true"

ok "SonarQube analysis + quality gate"
echo "    dashboard: ${SONAR_HOST_URL}/dashboard?id=phototux"
exit 0
