# AIW agent integration guide

Give this document to any coding agent that must install, enable, or use AIW.

## What AIW adds

AIW separates durable project state from an agent conversation. The repository is
the long-running process; Claude, Codex, and other agents are replaceable workers.

The integration has three layers:

1. The `aiw` CLI owns the deterministic state machine, compact recovery views,
   task claims, checkpoints, command capture, and verification evidence.
2. The `aiw-workspace` skill tells an agent when and how to use those operations.
3. Lifecycle hooks load AIW context at startup and can prevent one premature stop
   while the active task is still pending or in progress.

`.ai/state.json`, `.ai/policy.md`, and `.ai/decisions/` are canonical. Skills,
hooks, `AGENTS.md`, `CLAUDE.md`, conversation history, and derived indexes are
adapters or caches. They never override canonical state, Git, or source code.

## Install for the current user

From an AIW source checkout:

```sh
./scripts/install-user.sh
aiw --version
```

The installer places the binary in Cargo's user binary directory, normally
`~/.cargo/bin/aiw`. It installs the same portable skill in both locations:

```text
~/.agents/skills/aiw-workspace/   # Codex
~/.claude/skills/aiw-workspace/   # Claude Code
```

The binary directory must be on `PATH` in the agent process. A CLI-only install is:

```sh
cargo install --path /path/to/AIW --locked
```

## Enable a repository

Run this from the repository root:

```sh
aiw init --name PROJECT_NAME
aiw integrate all --enforcement strict
aiw doctor
```

Use `claude` or `codex` instead of `all` to generate only one vendor integration.
The command preserves unrelated instructions, settings, and hook groups. Repeated
runs are idempotent.

Generated project files are:

```text
.ai/                                      canonical and runtime AIW workspace
.agents/skills/aiw-workspace/             Codex project skill
.claude/skills/aiw-workspace/             Claude project skill
.codex/hooks.json                         Codex lifecycle hooks
.claude/settings.json                     Claude lifecycle hooks
AGENTS.md                                 small generated AIW bootstrap at start
CLAUDE.md                                 small generated AIW bootstrap at start
```

Restart the agent after enabling or changing hooks. Codex project hooks also need
repository trust and review through `/hooks`. Claude noninteractive mode loads
project settings from a trusted working directory.

## Enforcement modes

`--enforcement observe` installs startup hooks only. `SessionStart` and
`SubagentStart` call:

```sh
aiw hook session-start
```

The command detects the workspace from the hook's `cwd` and emits a bounded
`aiw load` packet. Outside an AIW repository it exits quietly.

`--enforcement strict` also installs a `Stop` hook that calls:

```sh
aiw hook stop --strict
```

If the active task is pending or in progress, the hook exits with code 2 and asks
the agent to continue once. It allows the repeated stop attempt so it cannot form
an infinite loop. It also allows stopping when the task is blocked, cancelled,
done, or absent.

Strict mode forces the agent to reconcile its exit with persisted task state. It
cannot prove semantic correctness or force good judgment. AIW therefore permits
`done` only after fresh declared verification and a concrete result.

## Required workflow for an agent

When `.ai/state.json` exists, do this before broad exploration:

```sh
aiw load
aiw show task TASK_ID
```

If the selected task is pending, claim it before editing:

```sh
aiw task claim TASK_ID --worker STABLE_WORKER_NAME
```

Honor the task's acceptance criteria, scope, constraints, dependencies, and saved
verification commands. Use focused retrieval before large reads:

```sh
aiw status
aiw probe 'specific term'
aiw show project
```

After meaningful progress, persist the next concrete action:

```sh
aiw checkpoint \
  --next 'Exact action the next agent should perform' \
  --note 'Short confirmed fact that is expensive to rediscover'
```

Inspect the saved command arrays before authorizing them, then verify:

```sh
aiw verify TASK_ID --allow-exec
```

On success:

```sh
aiw task transition TASK_ID done \
  --result 'Concrete implementation and verification result'
```

When work cannot continue:

```sh
aiw checkpoint --next 'Exact unblock action' --note 'Confirmed blocker context'
aiw task transition TASK_ID blocked --reason 'Specific external or technical blocker'
```

Never weaken acceptance, invent evidence, or mark a task blocked only to bypass the
strict hook.

## Adopt interrupted work

Do not copy a conversation transcript into AIW. Inspect current Git state, source,
the latest user request, existing project instructions, and real verification
commands. Record one coherent active plan and bounded executable tasks. Each task
needs acceptance criteria, relative scope, material constraints, literal command
arrays, and dependencies that reflect actual blocking relationships.

If a worker disappeared while a task was in progress:

1. Inspect its checkout and AIW state.
2. Checkpoint confirmed edits, failures, and the exact next action.
3. Transition the task to `blocked` if ownership or state is uncertain.
4. Move it back to `pending` after the checkout is safe to resume.
5. Let the replacement agent claim it under a new worker name.
6. Re-run verification before completion.

## Minimal handoff prompt

Give the replacement agent the repository path and task ID, then use this prompt:

```text
Use the aiw-workspace skill. Run `aiw load` and `aiw show task TASK_ID` before
broad exploration. Claim the task under your worker name, continue through its
acceptance criteria, and persist a checkpoint. Transition it to done only after
fresh declared verification; otherwise record the exact blocker and next action.
```

The replacement agent should not need the previous conversation.
