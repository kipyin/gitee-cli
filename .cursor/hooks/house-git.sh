#!/usr/bin/env bash
# House git identity for Cloud Agent sessions.
#
# Interface: house-git.sh apply [--watch]
#   Author: GIT_AUTHOR_NAME + GIT_AUTHOR_EMAIL when both are set; otherwise
#   leave user.name / user.email alone.
#   Signing: GIT_SIGNING_KEY (PEM or base64) -> ~/.ssh/cloud-signing; if unset,
#   gpgsign false (never Cursor unknown_key).
#   commit-msg strips Co-authored-by / Made-with trailers last.
#   --watch re-installs into hook dirs planted after the first apply.
set -euo pipefail

HOOKS_SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DISPATCHER="$HOOKS_SRC/commit-msg"
DEST_ROOT="${HOME}/.cursor/kstack-hooks"
SIGNING_KEY_FILE="$HOME/.ssh/cloud-signing"

usage() {
  echo "usage: $0 apply [--watch]" >&2
  exit 2
}

[[ "${1:-}" == "apply" ]] || usage
shift
WATCH=0
if [[ "${1:-}" == "--watch" ]]; then
  WATCH=1
  shift
fi
[[ $# -eq 0 ]] || usage

apply_identity() {
  if [[ -n "${GIT_AUTHOR_NAME:-}" && -n "${GIT_AUTHOR_EMAIL:-}" ]]; then
    git config --global user.name "$GIT_AUTHOR_NAME"
    git config --global user.email "$GIT_AUTHOR_EMAIL"
  fi
  # Never use ~/.cursor/bin/cursor-git-ssh-keygen (GitHub unknown_key / Unverified).
  git config --global --unset-all gpg.ssh.program || true
  if [[ -n "${GIT_SIGNING_KEY:-}" ]]; then
    mkdir -p "$HOME/.ssh"
    if [[ "$GIT_SIGNING_KEY" == *"BEGIN "* && "$GIT_SIGNING_KEY" == *"PRIVATE KEY"* ]]; then
      printf '%s\n' "$GIT_SIGNING_KEY" >"$SIGNING_KEY_FILE"
    else
      printf '%s' "$GIT_SIGNING_KEY" | base64 -d >"$SIGNING_KEY_FILE"
    fi
    chmod 600 "$SIGNING_KEY_FILE"
    ssh-keygen -y -f "$SIGNING_KEY_FILE" >"$SIGNING_KEY_FILE.pub"
    git config --global gpg.format ssh
    git config --global user.signingkey "$SIGNING_KEY_FILE.pub"
    git config --global commit.gpgsign true
    git config --global tag.gpgsign true
  else
    git config --global commit.gpgsign false
    git config --global tag.gpgsign false
    git config --global --unset-all user.signingkey || true
    git config --global --unset-all gpg.format || true
  fi
}

install_into() {
  local dest="$1"
  [[ -d "$dest" ]] || return 0
  [[ -f "$DISPATCHER" ]] || return 0
  if [[ -f "$dest/commit-msg" && ! -L "$dest/commit-msg" ]] && cmp -s "$DISPATCHER" "$dest/commit-msg"; then
    return 0
  fi
  local tmp
  tmp="$(mktemp "$dest/commit-msg.house.XXXXXX")"
  cp "$DISPATCHER" "$tmp"
  chmod +x "$tmp"
  mv -f "$tmp" "$dest/commit-msg"
  rm -f "$dest/commit-msg.cursor.strip-attribution"
}

install_all() {
  local hp
  if hp="$(git config --get core.hooksPath || true)" && [[ -n "$hp" ]]; then
    install_into "$hp"
  fi
  shopt -s nullglob
  local d
  for d in "${HOME}/.cursor/agent-hooks/"*/; do
    install_into "$d"
  done
  shopt -u nullglob
}

# Keep a durable copy for sessions that still look under kstack-hooks.
mkdir -p "$DEST_ROOT"
cp "$DISPATCHER" "$DEST_ROOT/commit-msg"
cp "$HOOKS_SRC/house-git.sh" "$DEST_ROOT/house-git.sh"
chmod +x "$DEST_ROOT/commit-msg" "$DEST_ROOT/house-git.sh"
rm -f "$DEST_ROOT/commit-msg.cursor.strip-attribution" \
  "$DEST_ROOT/apply-identity.sh" \
  "$DEST_ROOT/install-house-commit-msg.sh"

apply_identity
install_all

if [[ "$WATCH" -eq 1 ]]; then
  (
    while true; do
      sleep 5
      install_all || true
    done
  ) >>"$HOME/.cursor/house-git-watch.log" 2>&1 &
  disown || true
fi
