# verify

**Purpose.** Evaluate a task acceptance command contract.

**Inputs.** Task ID and --allow-exec after command inspection.

**Outputs.** Bounded JSON summary for up to 8 runs and a task drill-down pointer.

**State read.** In-progress task, plan, project invariants and source fingerprint.

**State written.** Latest canonical evidence; runtime logs and receipts.

**Deterministic work.** Run all commands sequentially, compare before/after source and persist result.

**LLM-required work.** Define meaningful acceptance; diagnose failure.

**Context boundary.** At most 3 diagnostics per displayed run; full receipts via show task/run.

**Transitions.** Evidence becomes passed/failed/stale; task stays in_progress.

**Idempotency / failure.** New runs each time. Empty contract, unmet dependencies or missing authorization fail before execution.
