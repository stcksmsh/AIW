# doctor

**Purpose.** Explain workspace validity and optional local capabilities.

**Inputs.** Working directory; workspace optional.

**Outputs.** JSON validation result, capabilities and limitations.

**State read.** Canonical state, root override presence, executable --version results.

**State written.** None.

**Deterministic work.** Validate schema/DAG and probe Git, rg, Rust and vendor binaries.

**LLM-required work.** Resolve missing tools only when needed for the chosen task.

**Context boundary.** Summaries only; no environment or credential dump.

**Transitions.** None.

**Idempotency / failure.** Read-only. Invalid existing workspace yields nonzero; absent workspace reports how to initialize.
