# task and bounded worker workflow

**Purpose.** Store executable units, schedule by dependencies, claim work and
persist verified results. **Inputs.** add/edit/focus/claim/transition and a task ID;
see `aiw task --help`. **Outputs.** Saved acknowledgement; inspect with show task.
**State read/written.** Task DAG, plans, intent and evidence in one atomic state
transaction. **Deterministic work.** Validate shape/references/cycles, transitions,
readiness, worker identity, and fresh evidence at completion. **LLM-required work.**
Choose bounded objective, acceptance, scope, constraints and meaningful result.
**Context boundary.** Load task packet, then its full contract and targeted source.
**Transitions.** See the specification's state machine. **Idempotency/failure.**
Same-state transition and same-worker claim are idempotent. Duplicate add, invalid
DAG, stale evidence, missing blocker/result, and conflicting ownership fail without
replacing canonical state. Editing requires pending/blocked and clears evidence.

## Independent workers

1. Coordinator runs `aiw list tasks --ready`, reads scopes, and assigns disjoint
   tasks. Claim in the coordinator checkout before creating worker branches.
2. Use `git worktree add -b TASK_BRANCH PATH` according to repository policy.
   Commit canonical assignments first if they must appear in the new worktree.
3. Give a worker the task ID, branch and `aiw load --task ID`. The packet includes
   the plan fragment, constraints, acceptance and likely relevant paths. Scope is
   declarative: the coordinator reviews the actual Git diff.
4. Worker implements, diagnoses and verifies. Return a compact object such as:
   `{"task":"ID","status":"done","changed_files":["src/x.rs"],"verification":"passed","findings":[],"blockers":[],"commit":"<actual SHA>"}`.
   AIW stores the canonical task result/evidence; Git stores actual changed files
   and commits. Do not invent a commit or copy giant logs into this object.
5. Coordinator reviews/cherry-picks as appropriate, resolves state.json conflicts
   deliberately, runs `aiw doctor`, and re-verifies in the combined checkout.

There is no provider launcher, global claim service, scope sandbox, automatic
merge, or worktree cleanup in v0.1. Separate checkouts have separate local locks.
