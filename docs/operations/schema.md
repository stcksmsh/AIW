# schema

**Purpose.** Export the version-1 language-independent JSON Schema.

**Inputs.** No workspace required.

**Outputs.** JSON Schema document.

**State read.** Compiled model metadata only.

**State written.** None; redirect explicitly to update published artifact.

**Deterministic work.** Generate schema with field types and unknown-field policy.

**LLM-required work.** Review compatibility implications of model changes.

**Context boundary.** Explicit schema detail; not part of default recovery.

**Transitions.** None.

**Idempotency / failure.** Deterministic for the same binary. Semantic validation remains required in addition to shape validation.
