use crate::{
    model::{State, Status},
    repo, runner,
    workspace::Workspace,
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};

pub fn clip(text: &str, budget: usize) -> String {
    let clean: String = text
        .chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .collect();
    if clean.len() <= budget {
        return clean;
    }
    let mut n = budget.saturating_sub(3).min(clean.len());
    while !clean.is_char_boundary(n) {
        n -= 1;
    }
    format!("{}...", &clean[..n])
}
pub fn selected<'a>(state: &'a State, requested: Option<&'a str>) -> Option<&'a str> {
    requested
        .or(state.active_task.as_deref())
        .or_else(|| {
            state
                .tasks
                .iter()
                .find(|(_, t)| {
                    t.status == Status::InProgress
                        && state.active_plan.as_ref().is_none_or(|p| *p == t.plan)
                })
                .map(|(id, _)| id.as_str())
        })
        .or_else(|| {
            state
                .tasks
                .iter()
                .find(|(id, t)| {
                    t.status == Status::Pending
                        && state.ready(id)
                        && state.active_plan.as_ref().is_none_or(|p| *p == t.plan)
                })
                .map(|(id, _)| id.as_str())
        })
}
pub fn status(ws: &Workspace, state: &State) -> Result<Value> {
    let mut counts = std::collections::BTreeMap::new();
    for task in state.tasks.values() {
        *counts.entry(task.status.label()).or_insert(0) += 1;
    }
    let source = repo::fingerprint(&ws.root)?;
    let selected = selected(state, None);
    let verification = selected
        .map(|id| runner::health(state, &state.tasks[id], &source))
        .transpose()?;
    Ok(
        json!({"schema_version":1,"project":clip(&state.project.name,160),"active_plan":state.active_plan,"active_task":state.active_task,"selected_task":selected,"tasks":counts,"verification":verification,"git_head":repo::git(&ws.root,&["rev-parse","--short","HEAD"]),"git_status":clip(&repo::git(&ws.root,&["status","--short","--untracked-files=normal"]).unwrap_or_default(),1500)}),
    )
}
pub fn load(ws: &Workspace, state: &State, task_id: Option<&str>, budget: usize) -> Result<String> {
    ensure!(
        (2048..=32768).contains(&budget),
        "load budget must be 2048–32768 bytes"
    );
    if let Some(id) = task_id {
        ensure!(state.tasks.contains_key(id), "unknown task {id}");
    }
    let mut lines = vec![format!("AIW v1 | {}", clip(&state.project.name,160)), format!("Purpose: {}", clip(&state.project.description,300)), "Authority: .ai/state.json + .ai/policy.md + .ai/decisions/. Read .ai/policy.md once; Git/source outrank derived views.".into()];
    let selected = selected(state, task_id);
    if let Some(id) = selected {
        let t = &state.tasks[id];
        let p = &state.plans[&t.plan];
        lines.push(format!(
            "Plan {}: {} | {}",
            t.plan,
            clip(&p.title, 120),
            clip(&p.objective, 240)
        ));
        lines.push(format!(
            "Task {id} [{}]: {}",
            t.status.label(),
            clip(&t.title, 240)
        ));
        let source = repo::fingerprint(&ws.root)?;
        let health = runner::health(state, t, &source)?;
        lines.push(format!(
            "Verification: {health}; {} declared command(s). Details: aiw show task {id}",
            t.verify.len()
        ));
        let next = state
            .checkpoint
            .as_ref()
            .filter(|c| c.task.as_deref() == Some(id))
            .map(|c| c.next_action.as_str());
        lines.push(format!(
            "Next: {}",
            clip(
                next.unwrap_or(match t.status {
                    Status::Pending =>
                        "Claim this task, inspect its scope, then implement acceptance criteria.",
                    Status::InProgress =>
                        "Inspect task and source; implement, verify, then persist result.",
                    Status::Blocked => "Resolve the persisted blocker before resuming.",
                    _ => "Task is complete; use aiw list tasks for subsequent work.",
                }),
                360
            )
        ));
        if let Some(blocker) = &t.blocker {
            lines.push(format!("Blocker: {}", clip(blocker, 300)));
        }
        if let Some(worker) = &t.worker {
            lines.push(format!("Worker: {}", clip(worker, 100)));
        }
        lines.push(format!(
            "Dependencies: {}",
            if t.dependencies.is_empty() {
                "none".into()
            } else {
                t.dependencies
                    .iter()
                    .map(|d| format!("{d}={}", state.tasks[d].status.label()))
                    .collect::<Vec<_>>()
                    .join(", ")
            }
        ));
        for (label, values) in [
            ("Acceptance", &t.acceptance),
            ("Scope", &t.scope),
            ("Constraint", &t.constraints),
            ("Invariant", &state.project.invariants),
        ] {
            for v in values.iter().take(4) {
                lines.push(format!("{label}: {}", clip(v, 240)));
            }
            if values.len() > 4 {
                lines.push(format!(
                    "{label}: {} more; aiw show task {id} / aiw show project",
                    values.len() - 4
                ));
            }
        }
        for command in t.verify.iter().take(3) {
            lines.push(format!(
                "Verify argv: {}",
                clip(&serde_json::to_string(&command.argv)?, 300)
            ));
        }
        if t.verify.len() > 3 {
            lines.push(format!(
                "Verify: {} more commands; aiw show task {id}",
                t.verify.len() - 3
            ));
        }
        if let Some(e) = &t.evidence {
            let logs = e.runs.iter().all(|r| {
                ws.path(&r.stdout).is_ok_and(|p| p.is_file())
                    && ws.path(&r.stderr).is_ok_and(|p| p.is_file())
            });
            lines.push(format!(
                "Raw verification artifacts: {} (receipts remain in task)",
                if logs {
                    "available"
                } else {
                    "missing/disposable"
                }
            ));
        }
        if let Some(c) = state
            .checkpoint
            .as_ref()
            .filter(|c| c.task.as_deref() == Some(id))
            && !c.note.is_empty()
        {
            lines.push(format!("Checkpoint fact: {}", clip(&c.note, 300)));
        }
    } else {
        if let Some(id) = &state.active_plan {
            lines.push(format!(
                "Plan {id}: {}",
                clip(&state.plans[id].objective, 300)
            ));
        }
        lines.push(
            "No runnable task selected. Inspect aiw list tasks; create work or resolve blockers."
                .into(),
        );
        if let Some(c) = state.checkpoint.as_ref().filter(|c| c.task.is_none()) {
            lines.push(format!("Next: {}", clip(&c.next_action, 360)));
        }
        for i in state.project.invariants.iter().take(4) {
            lines.push(format!("Invariant: {}", clip(i, 240)));
        }
    }
    let blockers: Vec<_> = state
        .tasks
        .iter()
        .filter(|(_, t)| t.status == Status::Blocked)
        .collect();
    for (id, t) in blockers.iter().take(3) {
        lines.push(format!(
            "Blocked {id}: {}",
            clip(t.blocker.as_deref().unwrap_or(""), 200)
        ));
    }
    if blockers.len() > 3 {
        lines.push(format!(
            "{} additional blockers; aiw list tasks --status blocked",
            blockers.len() - 3
        ));
    }
    if let Some(changes) = repo::git(&ws.root, &["status", "--short", "--untracked-files=normal"]) {
        lines.push(format!(
            "Git changes:\n{}",
            if changes.is_empty() {
                "clean".into()
            } else {
                clip(&changes, 600)
            }
        ));
    }
    if let Some(history) = repo::git(&ws.root, &["log", "-3", "--format=%h %s"]) {
        lines.push(format!("Recent Git:\n{}", clip(&history, 360)));
    }
    lines.push("Drill down: aiw status | aiw list tasks | aiw show task ID | aiw show project | aiw probe QUERY".into());
    let full = format!("{}\n", lines.join("\n"));
    if full.len() <= budget {
        return Ok(full);
    }
    let marker = "\n[Budget reached; fields omitted. Read aiw show task ID and aiw show project before acting.]\n";
    Ok(format!("{}{}", clip(&full, budget - marker.len()), marker))
}

pub fn run_packet(run: &crate::model::RunSummary) -> Value {
    let mut value = serde_json::to_value(run).expect("receipt is serializable");
    value["argv"] = json!(
        run.argv
            .iter()
            .take(24)
            .map(|a| clip(a, 256))
            .collect::<Vec<_>>()
    );
    if run.argv.len() > 24 || run.argv.iter().any(|a| a.len() > 256) {
        value["argv_truncated"] = json!(true);
        value["details"] = json!(format!("aiw show run {}", run.id));
    }
    value
}
pub fn verification_packet(evidence: &crate::model::Evidence, id: &str) -> Value {
    json!({"schema_version":1,"success":evidence.success,"source_hash":evidence.source_hash,"contract_hash":evidence.contract_hash,"run_count":evidence.runs.len(),"runs":evidence.runs.iter().take(8).map(|r| json!({"id":r.id,"argv":clip(&serde_json::to_string(&r.argv).unwrap_or_default(),300),"success":r.success,"exit_code":r.exit_code,"timed_out":r.timed_out,"duration_ms":r.duration_ms,"diagnostics":r.diagnostics.iter().take(3).collect::<Vec<_>>(),"stdout":r.stdout,"stderr":r.stderr})).collect::<Vec<_>>(),"details":format!("aiw show task {id}"),"omitted_runs":evidence.runs.len().saturating_sub(8)})
}
