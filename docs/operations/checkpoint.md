# checkpoint

**Purpose.** Persist only facts needed at the next restart.

**Inputs.** --next TEXT, optional --note TEXT and --task ID (default active task).

**Outputs.** Saved acknowledgement and recovery command.

**State read.** Current intent and task IDs.

**State written.** Latest checkpoint in canonical state.

**Deterministic work.** Validate bounded text/references and replace atomically.

**LLM-required work.** Select a concrete next action and relevant settled observation.

**Context boundary.** No repeated diff, source dump, log or conversation summary.

**Transitions.** Only checkpoint changes.

**Idempotency / failure.** Same fields produce same state. Invalid task/text fails without replacement.
