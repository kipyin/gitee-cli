# Agent notes

Contributor workflow is in [CONTRIBUTING.md](CONTRIBUTING.md). Domain terms:
[CONTEXT.md](CONTEXT.md). Decisions: [docs/adr/](docs/adr/).

Single Rust crate (`gitee-cli-rs`, binary `gitee`).

- Toolchain: Rust **≥ 1.85**. If a build fails with `feature edition2024 is required`, run `rustup update stable && rustup default stable`.
- Build / lint / test: `cargo build --locked`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --locked`. Always pass `--locked`.
- Run: `./target/debug/gitee <cmd>` or `cargo run -- <cmd>`.
- Live API calls need auth (`$GITEE_TOKEN` → OS keyring → `~/.config/gitee/`). Tests use a mockito HTTP server; `auth` / `config` / `alias` work without a token.
