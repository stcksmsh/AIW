# list

**Purpose.** Page through task or plan summaries.

**Inputs.** tasks/plans; limit 1–100, offset; optional task status/ready filter.

**Outputs.** JSON page, total and next offset.

**State read.** Canonical maps and dependencies.

**State written.** None unless events are requested.

**Deterministic work.** Stable ID order, DAG readiness, pagination.

**LLM-required work.** Choose work using objective and scope.

**Context boundary.** At most one page; retrieve task detail separately.

**Transitions.** None.

**Idempotency / failure.** Repeat is stable; invalid page bounds fail.
