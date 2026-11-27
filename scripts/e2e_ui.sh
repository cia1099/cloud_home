#!/usr/bin/env bash
# Browser end-to-end tests of the web UI (R-032): real cloud_home binary + real Chromium (playwright-core).
# Usage: scripts/e2e_ui.sh [--media]   (--media also writes screenshots/recordings to plan/media/R-032/)
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

( cd "$ROOT" && cargo build --quiet )
( cd "$ROOT/ui" && bun install --frozen-lockfile >/dev/null && bun run build >/dev/null )
cd "$ROOT/e2e/ui"
bun install --frozen-lockfile >/dev/null 2>&1 || bun install >/dev/null

if [ "${1:-}" = "--media" ]; then
  export E2E_MEDIA_DIR="$ROOT/plan/media/R-032"
  mkdir -p "$E2E_MEDIA_DIR"
fi
bun test --timeout 120000
