# github

**Purpose.** Explicit opt-in import of one GitHub Issue and opening one pull
request for a completed task branch.

**Inputs.** `GH_TOKEN`, GitHub CLI `gh`, one `OWNER/REPO`, and either
`github import-issue --repo OWNER/REPO --issue NUMBER` or
`github open-pr TASK --repo OWNER/REPO --base BRANCH --body TEXT`.

**Issue format.** Issue requires label `agent`. Prose before `## Acceptance`
becomes a new plan objective. Each `- ` or `* ` line under that heading becomes
a task acceptance criterion. Issue title becomes task title. Import creates
`github-issue-NUMBER` with empty scope and verification; add both before claiming
completion.

**Outputs.** Import prints created task/plan ID. PR open prints task branch and
GitHub API response.

**State read/written.** Import writes one plan and one pending task. PR open reads
a done task and local branch. Neither changes evidence, checkpoints, hooks or
runtime locks.

**Deterministic work.** Validate local IDs/state and issue format. GitHub calls are
only made after explicit adapter invocation and `GH_TOKEN` validation.

**LLM-required work.** Write issue plan prose, acceptance bullets, scope,
verification and PR body.

**Context boundary.** No polling, scheduler, dashboard, multi-repo sync, push,
merge, or provider dependency. Core lifecycle never invokes `gh` or requires
network/authentication.

**Transitions.** Imported task starts pending. PR open requires task `done` and
existing branch `aiw/task/TASK`; push branch before opening PR.
