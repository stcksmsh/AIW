use crate::{model::Status, repo, runner, view, workspace::Workspace};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{io::Read, path::PathBuf};

pub struct Outcome {
    pub context: Option<String>,
    pub block: Option<String>,
}

fn input() -> Result<Value> {
    let mut bytes = Vec::new();
    std::io::stdin()
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 1024 * 1024, "hook input exceeds 1 MiB");
    if bytes.is_empty() {
        return Ok(Value::Object(Default::default()));
    }
    serde_json::from_slice(&bytes).context("invalid hook JSON input")
}

fn workspace(value: &Value) -> Option<Workspace> {
    let cwd = value
        .get("cwd")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())?;
    Workspace::discover(&cwd).ok()
}

pub fn session_start() -> Result<Outcome> {
    let value = input()?;
    let Some(ws) = workspace(&value) else {
        return Ok(Outcome {
            context: None,
            block: None,
        });
    };
    let state = ws.read()?;
    let packet = view::load(&ws, &state, None, 8192)?;
    Ok(Outcome {
        context: Some(format!(
            "AIW lifecycle integration is active. Use the aiw-workspace skill. \
             Treat this packet as recovery context, then inspect only the selected task and relevant source.\n\n{packet}"
        )),
        block: None,
    })
}

pub fn stop(strict: bool) -> Result<Outcome> {
    let value = input()?;
    if value
        .get("stop_hook_active")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(Outcome {
            context: None,
            block: None,
        });
    }
    let Some(ws) = workspace(&value) else {
        return Ok(Outcome {
            context: None,
            block: None,
        });
    };
    let state = ws.read()?;
    let Some(id) = state.active_task.as_deref() else {
        return Ok(Outcome {
            context: None,
            block: None,
        });
    };
    let task = &state.tasks[id];
    if !strict || task.status.terminal() || task.status == Status::Blocked {
        return Ok(Outcome {
            context: None,
            block: None,
        });
    }
    let health = if task.status == Status::InProgress {
        runner::health(&state, task, &repo::fingerprint(&ws.root)?)?
    } else {
        "unverified"
    };
    Ok(Outcome {
        context: None,
        block: Some(format!(
            "AIW strict guard: active task {id} is {} with verification {health}. \
             Continue the declared task, or persist an honest blocker and checkpoint before stopping. \
             If acceptance is satisfied, run `aiw verify {id} --allow-exec` and transition it to done with a concrete result.",
            task.status.label()
        )),
    })
}
