# checkpoint

**Purpose.** Persist only facts needed at the next restart.

**Inputs.** --next TEXT, optional --note TEXT and --task ID (default active task).

**Outputs.** Saved acknowledgement and recovery command.

**State read.** Current intent, task scope and discovered local source.

**State written.** Latest checkpoint in canonical state, including `source_hash`
for the selected task. Taskless checkpoints omit the hash.

**Deterministic work.** Validate bounded text/references, hash the sorted source
map filtered to exact scope paths and their descendants, and replace atomically.
Empty scope covers all discovered source. Discovery, ignore and file hashing
rules match verification. No semantic progress is inferred.

**LLM-required work.** Select a concrete next action and relevant settled observation.

**Context boundary.** No repeated diff, source dump, log or conversation summary.

**Transitions.** Only checkpoint changes. Verification evidence is preserved.

**Idempotency / failure.** Same fields and scoped source produce same state.
An explicit checkpoint captures current source even when the text is unchanged.
Invalid task/text or a source-read failure fails without replacement. Legacy
checkpoints without a fingerprint remain readable with unknown source freshness.
