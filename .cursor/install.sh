#!/usr/bin/env bash
# Idempotent Cloud Agent install: kstack skills + house git identity.
# Per-boot identity re-apply lives in start.sh.
set -euo pipefail

CURSOR_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$CURSOR_DIR/.."

# Official Matt 25 + global extras from public kipyin/kstack (install.sh global).
# Do not vendor those files in this repo.
rm -rf /tmp/kstack
GIT_TERMINAL_PROMPT=0 git clone --depth 1 https://github.com/kipyin/kstack.git /tmp/kstack
bash /tmp/kstack/install.sh global

# House git identity (author, signing, commit-msg strip). Durable under
# ~/.cursor/kstack-hooks; start.sh re-applies with --watch each session.
bash "$CURSOR_DIR/hooks/house-git.sh" apply || true
