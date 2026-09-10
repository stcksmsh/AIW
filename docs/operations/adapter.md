# adapter

**Purpose.** Generate Codex and Claude bootstrap compatibility files.

**Inputs.** codex, claude, or all (default).

**Outputs.** Generated path list.

**State read.** Existing AGENTS.md/CLAUDE.md and validated canonical workspace.

**State written.** Marked bootstrap blocks in root adapter files.

**Deterministic work.** Validate markers; preserve surrounding content; generate
identical neutral pointers at the beginning so bounded instruction loading sees
the bootstrap.

**LLM-required work.** Optional vendor enhancements outside markers only.

**Context boundary.** Bootstrap is small; policy and operation details remain separate.

**Transitions.** No canonical state changes.

**Idempotency / failure.** Repeated generation is identical. Malformed/duplicate markers or symlinks fail before planned writes.
