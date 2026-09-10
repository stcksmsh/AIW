# integrate

**Purpose.** Install a portable agent workflow skill and lifecycle hooks around
the canonical AIW CLI.

**Inputs.** `codex`, `claude`, or `all` (default), plus `--enforcement observe` or
`--enforcement strict`.

**Outputs.** Generated path list, selected enforcement, and the restart/trust next
step.

**State read.** Existing vendor instruction and JSON configuration files plus the
validated AIW workspace.

**State written.** Marked root bootstrap blocks; project skill files under
`.agents/skills` and `.claude/skills`; merged hook groups in `.codex/hooks.json`
and `.claude/settings.json`. Existing unrelated settings and hooks are preserved.

**Deterministic work.** SessionStart and SubagentStart run `aiw hook session-start`,
which emits a bounded recovery packet when the hook working directory belongs to
an AIW workspace and otherwise emits nothing. Strict Stop runs
`aiw hook stop --strict`. It exits 2 once while the active task is pending or in
progress. The repeated-hook flag, blocked tasks, terminal tasks, and repositories
without AIW exit successfully.

**LLM-required work.** Reconcile a changed user request with the task contract,
implement acceptance, diagnose verification, and persist an honest result or
blocker.

**Context boundary.** Startup output is capped at 8,192 bytes. The skill links to
adoption and handoff references that load only when relevant.

**Transitions.** No canonical state changes. Hooks cannot mark work done or invent
a task.

**Idempotency / failure.** Repeated integration replaces only AIW hook groups and
generates identical files. Existing hook JSON is parsed before any integration
file is written. Malformed JSON, markers, or managed symlinks fail closed.
