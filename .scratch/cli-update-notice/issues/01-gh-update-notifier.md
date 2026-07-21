# How does gh's update notifier work?

Type: research
Status: resolved

## Question

How does GitHub CLI (`gh`) implement its update notifier end-to-end: where it learns the latest version, cache/TTL and file location, opt-out env/config knobs, how (if at all) it detects Homebrew, the exact user-visible message shape, and when relative to command execution it prints?

## Answer

Homebrew-only (`updateable` build tag): background `GET /repos/cli/cli/releases/latest`, 24h YAML cache at `$XDG_STATE_HOME/gh/state.yml` (else `~/.local/state/gh/state.yml`), opt-out via `GH_NO_UPDATE_NOTIFIER` (no config key; also skip CI/Codespaces/non-TTY). Runtime Homebrew path check adds `brew upgrade gh` and suppresses notices for releases &lt;24h old. On successful commands only, prints yellow/cyan stderr tip `A new release of gh is available: X → Y` plus release URL after the command finishes.

## Comments

- Research write-up: [../research/gh-update-notifier.md](../research/gh-update-notifier.md)
