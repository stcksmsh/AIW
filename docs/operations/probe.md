# probe

**Purpose.** Locate relevant source with lexical retrieval.

**Inputs.** Literal query ≤512 bytes; result limit 1–100.

**Outputs.** JSON path/line/text hits and more flag.

**State read.** Fresh index membership and selected current source text.

**State written.** Refreshed derived index.

**Deterministic work.** Case-insensitive substring matches in paths and lines, stable file/line order.

**LLM-required work.** Select a few relevant hits; make semantic inferences only after source inspection.

**Context boundary.** ≤100 hits; snippets ≤240 bytes. No symbol-resolution claim.

**Transitions.** Derived refresh only.

**Idempotency / failure.** Same content produces same hits. Empty query fails; binaries/large files are not searched.
