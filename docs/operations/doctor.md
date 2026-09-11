# doctor

**Purpose.** Explain workspace validity, optional local capabilities, and the
read-only health of project-scoped Codex and Claude integrations.

**Inputs.** Working directory; workspace optional.

**Outputs.** JSON validation result, capabilities, integration health, and
limitations. For each adapter, `integrations` reports the bootstrap block, all
three skill files, and AIW-owned lifecycle handlers. Missing files report
`missing`; invalid JSON or hook containers report `malformed`; changed managed
skills or malformed/duplicate AIW handlers report `stale`. A valid startup-only
configuration is `observe`; a valid Stop handler makes it `strict`.

**State read.** Canonical state, root override presence, executable --version
results, and project integration files. It does not require a provider binary,
account, network connection, or another tool.

**State written.** None.

**Deterministic work.** Validate schema/DAG, probe Git, rg, Rust and vendor
binaries, and inspect only exact AIW lifecycle command handlers. It does not
inspect or infer unrelated handler behavior.

**LLM-required work.** Resolve missing tools only when needed for the chosen task.

**Context boundary.** Summaries only; no environment or credential dump.

**Transitions.** None.

**Idempotency / failure.** Read-only. Invalid existing workspace yields nonzero; absent workspace reports how to initialize.
