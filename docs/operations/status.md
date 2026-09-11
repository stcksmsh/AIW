# status

**Purpose.** Report current intent and verification health.

**Inputs.** Working directory.

**Outputs.** JSON counts, selected task, freshness and capped Git status.
`checkpoint_freshness` is `unchanged`, `changed`, `missing` (no checkpoint for the
selected task), `unknown` (legacy checkpoint without a hash), or null (no selected
task). It is independent of `verification` health.

**State read.** Canonical state and current repository fingerprint.

**State written.** None unless events are requested.

**Deterministic work.** Count states, choose task and compare evidence.

**LLM-required work.** None.

**Context boundary.** No logs or completed task bodies.

**Transitions.** None.

**Idempotency / failure.** Stable for identical inputs; malformed canonical state fails.
