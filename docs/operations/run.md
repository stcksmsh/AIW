# run

**Purpose.** Run explicit argv and compress output.

**Inputs.** --timeout 1–86400 (default 600); -- PROGRAM ARGS.

**Outputs.** Compact JSON receipt and nonzero CLI exit on failure.

**State read.** Validated workspace; command runs from root with inherited environment.

**State written.** Runtime raw stdout/stderr and result JSON; optional events.

**Deterministic work.** Spawn, capture, timeout, parse Cargo/rustc JSON, deduplicate and bound summaries.

**LLM-required work.** Diagnose only relevant failures; retrieve raw logs when summary is insufficient.

**Context boundary.** At most 24 diagnostics of 400 bytes plus 8 tail lines; argv display is capped.

**Transitions.** No task transition or acceptance claim.

**Idempotency / failure.** Each execution has a new artifact directory. Launch failure is captured; interruption leaves no passing task evidence.
