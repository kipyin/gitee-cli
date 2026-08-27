# Contributing

Thanks for helping. This repo is a single Rust crate (`gitee-cli-rs`); the
installed binary is `gitee`.

## Build

Rust **1.85+** is required (`Cargo.lock` pins crates that need edition2024).

```bash
git clone https://github.com/kipyin/gitee-cli.git
cd gitee-cli
cargo build --locked
```

Run a local binary with `./target/debug/gitee <cmd>` or `cargo run -- <cmd>`.

Live API commands need a Gitee token (`GITEE_TOKEN` or `gitee auth login`).
`auth` / `config` / `alias` work without one. Tests mock HTTP with mockito
and do not call Gitee.

## Test

CI runs the same commands. Please run them before opening a PR:

```bash
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
```

Always pass `--locked` so `Cargo.lock` stays the source of truth.

## Pull requests

- Target `main`. Keep the change focused.
- Do not bump the crate version unless you are cutting a release.
- Gitee API paths, query keys, and body quirks belong in `src/api/`, not in
  `src/cmd/` — see [ADR-0002](docs/adr/0002-operations-modules-own-api-shape.md).
- Gitee issue identifiers are strings (e.g. `I88`), not integers — see
  [ADR-0001](docs/adr/0001-issue-numbers-are-strings.md).
- Domain terms live in [CONTEXT.md](CONTEXT.md).

## Issues

Use [GitHub Issues](https://github.com/kipyin/gitee-cli/issues). Labels currently
in use:

| Label | Meaning |
|-------|---------|
| `bug` | Something is broken |
| `enhancement` | New feature or request |
| `documentation` | Docs-only work |
| `good first issue` | Small, well-scoped starter |
| `help wanted` | Extra attention needed |
| `question` | Clarification |
| `wontfix` | Will not be actioned |

Include `gitee --version`, OS, and the command you ran. For API surprises,
add `--debug` output and redact tokens.

## License

Contributions are accepted under the MIT License. See [LICENSE](LICENSE).
