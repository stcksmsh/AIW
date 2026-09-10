# plan

**Purpose.** Record a coherent body of work or select it.

**Inputs.** add ID TITLE --objective TEXT; activate ID.

**Outputs.** Saved acknowledgement.

**State read.** Canonical plans, tasks and intent.

**State written.** Atomic state.json under lock.

**Deterministic work.** Validate IDs/references; selecting a plan clears active task.

**LLM-required work.** Choose objective and task decomposition.

**Context boundary.** Read the plan and affected task fragments.

**Transitions.** Create plan or change active intent.

**Idempotency / failure.** Activating same plan is safe; duplicate add fails without replacement.
