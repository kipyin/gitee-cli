# SDD Progress — gh-parity tickets 15–22

BASE (before Batch 1): 9a71135 docs: refresh README for tickets 01-14 + platform limits

## Batching
1. 18/19/20/21 parallel (isolated worktrees) — FAILED (dispose + relative worktree.base); implemented in-controller instead
2. 22 repo star/watch
3. 16 → 17 sequential
4. 15 browse/--web

## Seams (user-approved)
- API ops: tests/ops_*.rs (mockito)
- CLI parse: cli.rs parse_tests
- Pure unit: URL/config/alias/credential where applicable

## Ledger
- Task 18-21: complete (in-controller; full cargo test green; pending commit)
