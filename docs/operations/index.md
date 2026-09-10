# index

**Purpose.** Refresh a disposable lexical repository index.

**Inputs.** Working directory.

**Outputs.** Counts of rebuilt/reused/removed entries.

**State read.** Git/ignore-aware membership, hashes, UTF-8 text ≤1 MiB, optional old index.

**State written.** Derived index with schema/provenance; runtime lock.

**Deterministic work.** Hash every input, reuse unchanged extraction, remove deleted entries.

**LLM-required work.** None.

**Context boundary.** Print counts; source text stays outside context.

**Transitions.** Derived generation only.

**Idempotency / failure.** Unchanged index content is stable. Corrupt old index rebuilds; unreadable source fails.
