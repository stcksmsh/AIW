# ADR 0001: a portable workspace, a deterministic CLI

Status: accepted for v0.1 (2026-09-10).

The project is the long-running process; agents are disposable compute.

## Decisions

- Rust reference CLI; version 1 JSON workspace contract is language independent.
- `.ai/state.json` is the atomic canonical transaction boundary: project, plans,
  tasks, intent, and the latest compact checkpoint. Markdown holds policy and ADRs.
  A single file trades merge granularity for reliable DAG and intent updates.
- `.ai/derived/` is rebuildable; `.ai/runtime/` contains disposable logs and locks.
  Neither is authoritative. Verification receipts describe evidence at a content
  fingerprint; missing runtime artifacts and stale evidence are visible.
- OS file locking serializes local mutations; atomic replacement prevents partial
  state. Readers validate versions, references and cycles. Unsupported versions
  fail closed without writes. No pretend migration from a nonexistent predecessor.
- Git supplies repository membership and history. A filesystem walker supports
  non-Git directories. SHA-256 supplies invalidation; lexical retrieval comes first.
- Verification runs explicit argv, never an implicit shell. Repository-defined
  commands require `--allow-exec`; command output goes to files, not agent context.
- `load` has a hard UTF-8 byte budget and omits completed task history. `show` and
  paginated `list` provide deeper inspection. No model calls in the CLI.
- AGENTS.md and CLAUDE.md contain generated bootstrap blocks pointing to the same
  canonical policy and CLI. Existing surrounding instructions survive generation.
- Dependency readiness and worker claims are primitives, not an agent scheduler.
  Git worktrees are managed by Git; isolated worker changes return through commits.

## Scope

Retain init, load, status, list, show, plan, task, run, verify, checkpoint,
handoff (load alias), index, probe, adapter and doctor. Combine execute with run;
review is a human/agent operation; delegation uses task claims and packets.
Archive/ship/gc are deferred until there is an actual lifecycle to automate.

No database, service, embeddings, provider SDK, MCP server, or persistent LSP
process. Structured Cargo/rustc JSON plus bounded generic text handles output.
Semantic symbol resolution, other structured diagnostic adapters, automatic
worktree merges, cost collection, and remote side effects are later extensions.

## Research basis

Official sources inspected on 2026-09-10; capabilities inform adapters, never the
canonical format. Detailed links and adoption decisions: [research](../../docs/research.md).

## Consequences

Small projects pay for content hashing on verification/recovery; this buys honest
freshness without trusting timestamps. Ignored files and external environments
are outside evidence coverage. Receipt freshness is not a hermetic build claim.
Concurrent branches can conflict in state.json; resolve as ordinary Git data,
then run doctor. Commands are trusted programs, not sandboxed by AIW.
