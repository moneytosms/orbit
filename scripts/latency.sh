#!/usr/bin/env bash
set -euo pipefail

if [[ "${1:-}" == "--check" ]]; then
  shift
  exec python3 "$(dirname "$0")/latency.py" --check "$@"
fi

build_lock="${ORBIT_BUILD_LOCK:-${TMPDIR:-/tmp}/orbit-cargo-build.lock}"
if command -v rtk >/dev/null 2>&1; then
  flock "$build_lock" rtk cargo build --release -p orbit-server -p orbit-cli
else
  flock "$build_lock" cargo build --release -p orbit-server -p orbit-cli
fi
exec python3 "$(dirname "$0")/latency.py" "$@"
