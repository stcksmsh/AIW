# show

**Purpose.** Retrieve one explicit detail entity.

**Inputs.** task/plan/run plus ID, or project/checkpoint/inspection.

**Outputs.** Full entity JSON; inspection is computed fresh.

**State read.** Only requested canonical entity, runtime receipt, or discovery.

**State written.** None unless events are requested.

**Deterministic work.** Lookup and validation; deterministic inspection heuristics.

**LLM-required work.** Interpret selected contract or evidence.

**Context boundary.** Explicit detail may be large; never silently include raw logs.

**Transitions.** None.

**Idempotency / failure.** Repeat stable absent changes; missing entity/artifact fails explicitly.
