# worktree

**Purpose.** Create and clean Git worktrees bound to an AIW task branch.

**Inputs.** `worktree create TASK --path PATH` or `worktree cleanup TASK --path PATH`.

**Outputs.** JSON task, branch and path metadata. Create uses branch
`aiw/task/TASK`. Cleanup retains that branch.

**State read.** Main workspace task state; cleanup also reads task state from the
specified worktree.

**State written.** No canonical AIW state. Git writes the requested worktree.

**Deterministic work.** Create rejects terminal tasks and existing/in-tree paths.
Cleanup requires a registered worktree, matching task branch, clean Git status,
and task `done` or `cancelled` in that worktree.

**LLM-required work.** Choose task, path, source edits and commit boundary.

**Context boundary.** No provider, network, remote, merge, push, branch deletion,
or forced removal occurs.

**Transitions.** None. Claim, verify and transition occur inside the worktree.

**Idempotency / failure.** Existing branches/paths, dirty worktrees and active
tasks fail without removal. Commit or otherwise resolve worktree changes before
cleanup.
