# gitee-cli — Domain Glossary

Single-context repo. System-wide decisions: `docs/adr/`.

## Terms

**Gitee** — The hosted service and its REST API v5 (`https://{host}/api/v5`). This CLI targets Gitee, not GitHub.

**Host** — Gitee hostname (CLI `--host`, default `gitee.com`). Selects API base URL and which stored token to use.

**Token** — Personal access token (PAT) for API auth. Resolved by `Config::token(host)` with precedence: `$GITEE_TOKEN` env → OS keyring (service `gitee-cli`, account = host) → plaintext `{host}.token` file. Sent as `Authorization: token …` header.

**Repo** — Repository identity `owner/name` (`src/repo.rs::Repo`). Resolved lazily in `Ctx::repo()`: explicit `--repo` wins; else `git remote get-url` for `--remote` (default `origin`).

**Pull Request** — Merge request on a repo. Model: `PullRequest` in `src/models.rs`; `number` is `i64`. Operations: `src/api/pulls.rs`.

**Issue** — Work item on a repo. Model: `Issue` in `src/models.rs`; `number` is `String` (alphanumeric, e.g. `I6D3AV`) — see ADR-0001. Operations: `src/api/issues.rs`.

**Release** — Tag-based release with optional notes and **assets** (`ReleaseAsset`). Identified by `tag_name`; operations include create, list, get-by-tag, upload asset. Model: `Release` in `src/models.rs`; operations: `src/api/releases.rs`.

**State** — Lifecycle vocabulary in `src/models.rs`: `PrState` (`open`, `closed`, `merged`) and `IssueState` (`open`, `progressing`, `closed`, `rejected`). Unknown API values deserialize to `Unknown` for forward compatibility; known values serialize as API strings.

**Operations module** — Typed API seam in `src/api/{pulls,issues,releases,repos}.rs`. Owns every path template, query key, form field name, JSON-vs-form encoding choice, and Gitee quirk. See ADR-0002.

**Transport** — Thin HTTP wrapper in `src/api/client.rs` (`Client`). Verb-level only: `get`, `get_paged`, `post`, `patch`, `patch_json`, `post_multipart`. No domain path knowledge beyond `{base}` + caller-supplied path.

**Output seam** — Human and JSON rendering in `src/out.rs`. Printers take `&mut impl Write`; `Output::render` picks JSON (`--json`) or a human callback. Production locks stdout; tests use buffers.

**Ctx** — Command runtime bundle in `src/cmd/mod.rs`: `Client` + `Output` + lazy `Repo` resolution (`OnceCell`). Handlers parse args, call operations modules, render via `out`.
