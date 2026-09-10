# load

**Purpose.** Recover enough context to continue; handoff is an alias.

**Inputs.** Optional task ID; 2048–32768 byte budget (8192 default).

**Outputs.** Bounded text with next action, contracts, health, Git and drill-down commands.

**State read.** Validated canonical state and current source/Git facts.

**State written.** None unless optional events are requested.

**Deterministic work.** Select task, compute freshness, omit completed history, truncate visibly.

**LLM-required work.** Reason only about the next task after reading its contract.

**Context boundary.** Overview first; show task/project before acting if fields are omitted.

**Transitions.** None.

**Idempotency / failure.** Stable for identical state/source/Git. Unknown task/version fails; never runs checks.
