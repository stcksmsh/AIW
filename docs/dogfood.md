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
keeps these in the tail only. The v0.1 suite contained 3 diagnostic unit tests and
30 CLI integration tests, including output storms and a 2 MB single-line raw log.

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

## v0.2 installed-agent recovery trial

The v0.2 integration was tested against an isolated snapshot of an interrupted
Hysteresis checkout. Its root `AGENTS.md` was 64,404 bytes, and Codex reported
truncating it at the 32 KiB instruction budget. The generated AIW bootstrap was
still present because v0.2 places it at the beginning. AIW reconstructed the
latest Julia-render feedback as a bounded task contract with acceptance, scope,
constraints, verification commands, and the exact known failing frame.

A fresh ephemeral Codex process with no prior conversation loaded the project
skill, ran `aiw load`, showed and claimed the task, and reproduced the boundary
regression at frame 399 (`zoom=0.37996876`, escape spread 0). It was intentionally
stopped when the user selected Claude for the continuing trial; it made no scoped
source edits. A fresh authenticated Claude Code 2.1.268 process then:

1. ran the generated `SessionStart` hook successfully;
2. received the bounded recovery packet as hook context;
3. discovered and explicitly launched the personal `aiw-workspace` skill;
4. ran `aiw load`, read the task, and claimed it as `claude-aiw-trial`;
5. reproduced the same existing failing `hyst-render` boundary test.

This is a successful live provider round trip for startup injection, skill
discovery, canonical recovery, claim transfer, and baseline reproduction. The
visual fix and full 90-second rendered acceptance remain work in the isolated
Hysteresis task, not evidence for AIW's own release. The original Hysteresis
checkout was not modified.

## v1 local consumer recovery and lifecycle trials

Recorded 2026-09-11 on Linux with the release build
`/secondary/Programming/Github/AIW/target/release/aiw` (`aiw 0.2.0`). Two new,
independent Git repositories were created with `mktemp -d`, at
`/tmp/aiw-release-codex.Uhz85k` and `/tmp/aiw-release-claude.W8Pa95`. Neither
repository was copied from AIW and neither consumer state was written back to
this repository. Each contained its own `.ai/state.json`, plan `v1`, and task
`resume`.

The Codex consumer used these exact operations (with the same `aiw` executable
above):

```sh
git init -q
aiw init --name 'Codex local lifecycle trial'
aiw plan add v1 'Local recovery trial' \
  --objective 'Prove recovery and handoff from canonical state'
aiw task add resume 'Recover an interrupted task' --plan v1 \
  --accept 'Fresh process recovers the task contract' \
  --accept 'Lifecycle handoff persists canonical state' --scope src \
  --constraint 'Do not use provider memory as state' \
  --verify '["/secondary/Programming/Github/AIW/target/release/aiw","--version"]'
aiw integrate codex --enforcement strict
aiw hook session-start
aiw task claim resume --worker codex-local-trial
aiw checkpoint --next 'A fresh process must load the interrupted task and transfer its claim'
aiw load --task resume
aiw hook stop --strict
aiw task transition resume blocked --reason 'Codex worker intentionally interrupted after persisted checkpoint; handoff required'
aiw task transition resume pending
aiw task claim resume --worker codex-handoff-trial
aiw verify resume --allow-exec
aiw task transition resume done --result 'Fresh local process recovered, transferred, verified, and completed the canonical task.'
```

`doctor` reported the Codex bootstrap, skill, and all three strict handlers as
present and valid. The fresh `load` recovered the claim, checkpoint fact, next
action, acceptance, constraint, and declared argv. `hook stop --strict` exited
2 while the task was active and unverified. After the recorded interruption,
the handoff claim verified successfully: `run-1miL1F` ran the declared version
argv with exit code 0, and the final fresh load reported `resume [done]`.

The Claude consumer repeated the same state-machine trial with
`aiw init --name 'Claude local lifecycle trial'`, `aiw integrate claude
--enforcement strict`, workers `claude-local-trial` and
`claude-handoff-trial`, and the corresponding Claude interruption reason.
Its `doctor` result reported the Claude bootstrap, skill, and strict handlers
valid; the fresh load and strict-stop guard had the same results. Its declared
verification was `run-lyuVNp`, which ran the same version argv with exit code
0, and its final fresh load reported `resume [done]`.

Both provider executables were locally installed (`codex-cli 0.153.4` and
Claude Code 2.1.268), but an authenticated provider session was not available
to these trials: invoking either agent to do work would require a model/service
call, which the release contract forbids. The trials therefore executed the
generated provider-specific lifecycle commands locally and validated their
canonical recovery and handoff behavior; they do not claim an interactive
provider-session result. The CLI, adapters, and evidence remain local and
deterministic, and no provider memory, network service, or consumer repository
state was treated as canonical for AIW.

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
v0.2 adds 4 CLI integration tests (34 total, plus 3 unit tests) covering exact
handler-level hook merging for Codex and Claude, install idempotency, startup
context, and strict-stop behavior. The merge regression exercises installers in
both sequential orders, including a third-party handler and metadata added to an
AIW group, and the strict-to-observe transition.
The generated schema is checked against the committed schema and the example is
parsed by the real CLI. Python jsonschema is not installed locally; no independent
JSON Schema validator result is claimed. CI repeats Rust checks on Linux, macOS
and Windows; provider calls are excluded from CI.

See canonical task evidence for the latest exact command receipts and outcomes.
The repository history and final task results are the durable checkpoint.
