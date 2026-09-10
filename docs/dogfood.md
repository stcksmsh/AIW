# Dogfood and recovery evidence

Recorded 2026-09-10 on Linux, Rust/Cargo 1.93.0. These are reproducible local
observations; CI execution on other operating systems is not claimed here.

## AIW managed its own implementation

After the initial 22 CLI integration tests passed, this repository was initialized
with the compiled `aiw init --name AIW`. Plan `v01` and tasks `harden`, `docs`, and
`release` recorded the remaining development. `release` depends on the first two.
The hardening task was claimed, checkpointed and recovered through `aiw load`.
That packet exposed an unfilled project description; the description and four
architectural invariants were added to canonical state.

Further development used task contracts, incremental index/probe, generated Codex
and Claude adapters, captured commands, and declared verification. Evidence and
final results live in `.ai/state.json`; raw output is disposable in `.ai/runtime/`.
The architecture record created before implementation now lives in `.ai/decisions/`.

Review fixed plan-objective freshness, unbounded aggregate verification output,
receipt validation, overbroad non-Git directory exclusion, and symlink ancestors
in tracked paths. Dogfooding verification also caught successful test names and
`0 failed` summaries being misclassified as diagnostics; a regression test now
keeps these in the tail only. The suite now contains 3 diagnostic unit tests and 30 CLI
integration tests, including output storms and a 2 MB single-line raw log.

## Provider-neutral disappearance scenario

`provider_handoff_uses_only_canonical_state` uses a temporary Git repository and
fresh, independent CLI processes:

1. Initialize state and generate both adapters.
2. Claim `bridge` as Claude and persist a next action.
3. Start a fresh process with no conversation and run `load`.
4. Assert recovery of project, plan, task, constraints, acceptance and commands.
5. Persist the disappeared-worker blocker, reset to pending and claim as Codex.
6. Verify the task and persist Codex's result.
7. Delete runtime artifacts and start another fresh process.
8. Read the result and load the completed task, reporting missing disposable logs.

All steps pass. No format conversion or vendor memory is involved. The Git
worktree test separately confirms that `.git` files and clean cloned state work.

Live installed-client smoke tests were also attempted with no conversation resume:
Claude Code (`--no-session-persistence`, read tools only) timed out after 40 seconds
without output; Codex (`exec --ephemeral --sandbox read-only`) could not initialize
because its own local state database was read-only in this execution environment.
These are **not** successful live provider round trips. The deterministic protocol
scenario is covered; end-to-end authenticated adapter behavior remains a manual
acceptance check in an environment where both clients can initialize normally.
AIW's timeout/failure capture retained compact receipts for both attempts.

## Recovery budget measurement

Run `python3 scripts/measure_recovery.py` after `cargo build`:

| Synthetic fixture | Recovery bytes |
| --- | ---: |
| Active task, no history | 759 |
| Same task plus 1,500 terminal historical tasks | 759 |
| Same history, explicit 2,048-byte budget | 759 |

The outputs before/after history were byte-identical. One observed debug-build
recovery took about 49 ms; latency is machine-dependent, not a performance promise.
An additional integration test combines long Unicode fields with 1,500 historical
tasks and asserts output ≤2,048 bytes with no history text. The real AIW hardening
packet measured 1,756 bytes during development; Git changes affect its exact size.
No provider-specific tokenizer is assumed or token count invented.

## Checks

`cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`,
`cargo test --locked`, and a release build are the final declared release contract.
The generated schema is checked against the committed schema and the example is
parsed by the real CLI. Python jsonschema is not installed locally; no independent
JSON Schema validator result is claimed. CI repeats Rust checks on Linux, macOS
and Windows; provider calls are excluded from CI.

See canonical task evidence for the latest exact command receipts and outcomes.
The repository history and final task results are the durable checkpoint.
