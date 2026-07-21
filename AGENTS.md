# gitee-cli

## Agent skills

### Issue tracker

Issues and specs live as local markdown files under `.scratch/<feature>/` (one spec + one file per ticket). See `docs/agents/issue-tracker.md`.

### Triage labels

Default five-role vocabulary (`needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`), recorded as a `Status:` line in each issue file. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at the repo root, created lazily by the skills when terms/decisions get resolved. See `docs/agents/domain.md`.

## Cursor Cloud specific instructions

Single Rust CLI crate (`gitee-cli-rs`, binary `gitee`). Standard commands are in `README.md` and `.github/workflows/ci.yml`.

- Toolchain gotcha: the committed `Cargo.lock` pins deps (e.g. `idna_adapter`) that require Rust edition2024, so a toolchain **≥ 1.85** is mandatory. The base VM's default `stable` may be older (1.83) and will fail with `feature edition2024 is required`. The update script runs `rustup update stable` + `rustup default stable`; if a build hits that error, run those manually.
- Build/lint/test (dev): `cargo build`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`. Always pass `--locked` (CI does).
- Run: `./target/debug/gitee <cmd>` (or `cargo run -- <cmd>`).
- API commands need auth: set `GITEE_TOKEN` env or run `gitee auth login`. Token lookup order is `$GITEE_TOKEN` → OS keyring → `~/.config/gitee/`. No network/token is needed to exercise `auth`/`config`/`alias` locally, and the integration tests use a `mockito` HTTP server (no live Gitee access required).
