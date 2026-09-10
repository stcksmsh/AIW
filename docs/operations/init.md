# init

**Purpose.** Scaffold a workspace and deterministic discovery.

**Inputs.** Optional name and working directory.

**Outputs.** Project identity and inspection path.

**State read.** Existing state/policy; Git and repository filenames.

**State written.** Missing canonical scaffold; derived inspection; runtime lock.

**Deterministic work.** Infer root, manifests, extensions, CI and likely tests; validate existing state.

**LLM-required work.** Supply semantic description and invariants after initialization.

**Context boundary.** Filenames and metadata only; no repository-wide source read.

**Transitions.** Absent workspace → version 1; existing decisions are preserved.

**Idempotency / failure.** Repeat preserves canonical content. Invalid state fails; partial scaffold can be retried.
