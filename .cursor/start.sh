#!/usr/bin/env bash
# Per-boot startup: re-apply house git identity. Idempotent.
# --watch covers Cursor planting agent-hooks after Start returns.
set -euo pipefail

CURSOR_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$CURSOR_DIR/.."

# House git: author/signing from env, strip Co-authored-by.
bash "$CURSOR_DIR/hooks/house-git.sh" apply --watch
