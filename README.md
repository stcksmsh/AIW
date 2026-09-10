# AIW — AI Workspace

**The project is the long-running process. Agents are disposable compute.**

AIW is a local Rust CLI and an open, vendor-neutral workspace format for coding
agents that must survive context loss, crashes, and provider changes. It stores
intent and task contracts in Git-friendly files, recovers a small context packet,
and runs verification without dumping build logs into the conversation.

No model, account, server, database, or network connection is required at runtime.
This is a working v0.1 reference implementation, not an agent orchestration service.

## Install

Requires Rust 1.93 or newer. Git is strongly recommended; plain directories work.

```sh
cargo install --path . --locked
aiw --version
aiw doctor
```

For development, use `cargo run -- <arguments>` or `target/debug/aiw` after
`cargo build`. The crate is not published. No license has been selected by the
repository owner; see [contributing](CONTRIBUTING.md) before redistributing.

## A complete workflow

Run these commands in the project you want to manage. Replace the verification
argv with that project's actual acceptance command.

```sh
aiw init --name example
aiw adapter all
aiw plan add first 'First release' --objective 'Ship the smallest verified feature'
aiw task add feature 'Implement the feature' --plan first \
  --scope src --scope tests \
  --accept 'The declared tests pass and exercise the feature' \
  --constraint 'Preserve the public API' \
  --verify '["cargo","test","--locked","--message-format=json"]'
aiw list tasks --ready
aiw task claim feature --worker claude
aiw load
# Implement within the declared scope.
aiw verify feature --allow-exec
aiw checkpoint --next 'Review the diff, then persist the result'
aiw task transition feature done --result 'Implemented feature; acceptance tests pass'
```

A fresh Codex, Claude, or other client starts with `aiw load`. The default packet
is **at most 8,192 UTF-8 bytes**, omits completed history, and points to detail:

```sh
aiw status
aiw list tasks --limit 20 --offset 0
aiw show task feature
aiw show project
aiw probe 'authentication'
aiw handoff --task feature --budget 4096
```

`load`/`handoff` are text; other operations emit JSON. Errors go to stderr with a
nonzero exit code. `show` is an explicit, potentially large detail view. Run from
subdirectories or use `aiw -C /path/to/project load`.

## What is durable?

| Location | Role | Normally tracked? |
| --- | --- | --- |
| `.ai/state.json` | Project, plans, task DAG, intent, checkpoint, latest evidence | Yes |
| `.ai/policy.md` | Small neutral operating contract | Yes |
| `.ai/decisions/` | Settled decisions, constraints and ADRs | Yes |
| `.ai/derived/` | Discovery and incremental lexical index | No |
| `.ai/runtime/` | Raw output, run receipts, locks, optional events | No |
| `AGENTS.md`, `CLAUDE.md` | Generated bootstrap blocks with local extensions | Yes; noncanonical |

Edit project metadata and invariants in `.ai/state.json`, then run `aiw doctor`.
Use CLI mutations for tasks; `task edit ID --file task.json` replaces a pending or
blocked task contract after validation. `aiw schema` exports the workspace schema.

## Verification and compact output

```sh
aiw run -- cargo test --locked --message-format=json
aiw run -- cargo clippy --locked --message-format=json -- -D warnings
aiw show run RUN_ID
```

Execution uses literal argv, with a 600-second default timeout. Use `--timeout`
before `--` to change it. stdout/stderr stream directly to complete raw files.
Cargo/rustc JSON diagnostics get locations and deduplication; other tools use a
bounded generic summary. Read the receipt's raw paths explicitly for full output.
AIW does not silently rewrite your commands or add Cargo flags.

A successful task receipt is tied to source content **and** its acceptance
contract. Editing either makes it stale. Commands that change source during
verification cannot produce a passing receipt. `done` requires fresh evidence
and a result. This is practical local evidence, not a hermetic build guarantee:
ignored files, external tools/environment, and submodule working-tree changes
are outside coverage. Review the [evidence model](docs/specification.md).

Repository-defined commands require explicit `--allow-exec`. There are no prompts.
AIW does not sandbox commands or prevent their network/filesystem side effects.
Never execute verification from an untrusted checkout without inspecting it.

## Parallel work and recovery

Dependencies determine which pending tasks can be claimed. Claims are atomic
within one workspace. A worker's packet is `aiw load --task ID`; the full contract
is `aiw show task ID`. Use ordinary Git branches/worktrees for independent edits,
then return the task result, verification, findings and commit. AIW never creates,
merges, resets, pushes or deletes Git state on your behalf.

After a worker disappears, persist the interruption and reassign explicitly:

```sh
aiw task transition feature blocked --reason 'Worker session disappeared'
aiw task transition feature pending
aiw task claim feature --worker codex
aiw load
```

Worktrees have separate state copies and locks; they are not a distributed claim
service. See [worker workflow](docs/operations/task.md).

## Scope and documentation

- [Workspace specification and state machine](docs/specification.md)
- [Architecture decision](docs/adr/0001-local-workspace.md)
- [Official-source research and adapter choices](docs/research.md)
- [Operation contracts](docs/operations/README.md)
- [Dogfood and recovery evidence](docs/dogfood.md)
- [Example workspace](examples/minimal-state.json)
- [Changelog](CHANGELOG.md)

v0.1 deliberately omits semantic symbol resolution, embeddings, MCP/LSP services,
automatic worker launching/merging, releases, and provider billing integrations.
The next milestone is measured structural Rust retrieval and a broader workspace
corpus, driven by recovery quality and bytes saved rather than abstraction count.
