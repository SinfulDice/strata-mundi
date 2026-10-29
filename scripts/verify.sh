#!/usr/bin/env bash
# Milestone checks (fmt, clippy, tests) printing only a short summary and the errors,
# so that build logs don't fill the Claude Code context. Full logs stay in a temp dir.
# Usage: scripts/verify.sh [--quick]   (--quick skips the release clippy)
set -uo pipefail
cd "$(dirname "$0")/.."

log_dir=$(mktemp -d -t strata-verify-XXXXXX)
status=0

summarize_failure() {
  grep -E -A2 '^(error|warning)(\[[A-Z0-9]+\])?:|^test .* FAILED|^---- .* stdout ----|panicked at' "$1" | head -n 80
}

summarize_tests() {
  awk '/^test result:/ { passed += $4; failed += $6; ignored += $8 }
       END { printf "      %d passed, %d failed, %d ignored\n", passed, failed, ignored }' "$1"
}

step() {
  local name=$1
  shift
  local log="$log_dir/$name.log"
  if "$@" >"$log" 2>&1; then
    echo "OK    $name"
    [ "$name" = test ] && summarize_tests "$log"
  else
    echo "FAIL  $name (full log: $log)"
    summarize_failure "$log"
    status=1
  fi
}

step fmt cargo fmt --all
step clippy cargo clippy --all-targets -- -D warnings
if [ "${1:-}" != "--quick" ]; then
  step clippy-release cargo clippy --release --all-targets -- -D warnings
fi
step test cargo test --workspace
exit $status
