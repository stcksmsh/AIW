use crate::workspace::Workspace;
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::fs;
const BEGIN: &str = "<!-- aiw:begin generated v1 -->";
const END: &str = "<!-- aiw:end -->";
pub fn generate(ws: &Workspace, vendor: &str) -> Result<Vec<String>> {
    let paths = match vendor {
        "codex" => vec!["AGENTS.md"],
        "claude" => vec!["CLAUDE.md"],
        "all" => vec!["AGENTS.md", "CLAUDE.md"],
        _ => anyhow::bail!("adapter must be codex, claude, or all"),
    };
    let body = format!(
        "{BEGIN}\n# AIW bootstrap\n\nRun `aiw load` from this repository before exploring. The `aiw` CLI must be on\n`PATH`; install it from the AIW source repository if it is missing. Use the\n`aiw-workspace` skill for adoption, recovery, verification, and handoff.\nRead `.ai/policy.md` once, then use `aiw show task ID` for the selected task.\nCanonical project state is `.ai/state.json`; decisions are `.ai/decisions/`.\nUse `aiw verify ID --allow-exec` for declared checks after reviewing commands.\nPersist results with `aiw task transition` and next actions with `aiw checkpoint`.\nThis block is generated; vendor-specific guidance belongs outside the markers.\n{END}"
    );
    let mut writes = vec![];
    for path in &paths {
        let p = ws.path(path)?;
        let old = if p.exists() {
            fs::read_to_string(&p)?
        } else {
            String::new()
        };
        ensure!(
            old.matches(BEGIN).count() <= 1 && old.matches(END).count() <= 1,
            "duplicate AIW markers in {path}; resolve manually"
        );
        let existing = match (old.find(BEGIN), old.find(END)) {
            (Some(start), Some(end)) => {
                ensure!(end > start, "malformed AIW markers in {path}");
                format!("{}{}", &old[..start], &old[end + END.len()..])
            }
            (None, None) => old,
            _ => anyhow::bail!("incomplete AIW markers in {path}; resolve manually"),
        };
        // Keep the bootstrap at the beginning. Codex caps the combined AGENTS.md
        // instruction payload, so an appended block can disappear in large files.
        let existing = existing.trim_start_matches(['\r', '\n']);
        let next = if existing.is_empty() {
            format!("{body}\n")
        } else {
            format!("{body}\n\n{existing}")
        };
        writes.push((*path, next));
    }
    for (path, next) in writes {
        ws.write(path, next.as_bytes())?;
    }
    Ok(paths.into_iter().map(str::to_string).collect())
}

const SKILL: &str = include_str!("../integrations/aiw-agent/skills/aiw-workspace/SKILL.md");
const ADOPT: &str =
    include_str!("../integrations/aiw-agent/skills/aiw-workspace/references/adopt.md");
const LIFECYCLE: &str =
    include_str!("../integrations/aiw-agent/skills/aiw-workspace/references/lifecycle.md");

fn bootstrap_health(ws: &Workspace, path: &str) -> Result<Value> {
    let path = ws.path(path)?;
    let status = if !path.exists() {
        "missing"
    } else {
        let content = fs::read_to_string(&path)?;
        match (content.matches(BEGIN).count(), content.matches(END).count()) {
            (1, 1) if content.find(END) > content.find(BEGIN) => "present",
            _ => "malformed",
        }
    };
    Ok(json!({"path": path.strip_prefix(&ws.root).unwrap_or(&path), "status": status}))
}

fn skill_health(ws: &Workspace, path: &str, expected: &str) -> Result<Value> {
    let path = ws.path(path)?;
    let status = if !path.exists() {
        "missing"
    } else if fs::read_to_string(&path)? == expected {
        "present"
    } else {
        "stale"
    };
    Ok(json!({"path": path.strip_prefix(&ws.root).unwrap_or(&path), "status": status}))
}

fn hook_health(ws: &Workspace, path: &str) -> Result<Value> {
    let path = ws.path(path)?;
    let display = path.strip_prefix(&ws.root).unwrap_or(&path);
    if !path.exists() {
        return Ok(json!({
            "path": display,
            "status": "missing",
            "enforcement": null,
            "handlers": {"session_start": "missing", "subagent_start": "missing", "stop": "missing"}
        }));
    }
    let root: Value = match crate::workspace::read_json(&path) {
        Ok(value) => value,
        Err(error) => {
            return Ok(json!({"path": display, "status": "malformed", "error": error.to_string()}));
        }
    };
    let Some(hooks) = root.as_object().and_then(|root| root.get("hooks")) else {
        return Ok(json!({
            "path": display,
            "status": "missing",
            "enforcement": null,
            "handlers": {"session_start": "missing", "subagent_start": "missing", "stop": "missing"}
        }));
    };
    let Some(hooks) = hooks.as_object() else {
        return Ok(
            json!({"path": display, "status": "malformed", "error": "hooks must be a JSON object"}),
        );
    };

    let mut found = serde_json::Map::new();
    let mut malformed = None;
    for (event, command, name) in [
        ("SessionStart", "aiw hook session-start", "session_start"),
        ("SubagentStart", "aiw hook session-start", "subagent_start"),
        ("Stop", "aiw hook stop --strict", "stop"),
    ] {
        let Some(groups) = hooks.get(event) else {
            found.insert(name.into(), json!("missing"));
            continue;
        };
        let Some(groups) = groups.as_array() else {
            malformed = Some(format!("hooks.{event} must be an array"));
            break;
        };
        let mut owned = Vec::new();
        for group in groups {
            let Some(handlers) = group.as_object().and_then(|group| group.get("hooks")) else {
                continue;
            };
            let Some(handlers) = handlers.as_array() else {
                malformed = Some(format!("hooks.{event} group hooks must be an array"));
                break;
            };
            for handler in handlers {
                if is_aiw_hook_handler(handler)
                    && handler.get("command").and_then(Value::as_str) == Some(command)
                {
                    owned.push((group, handler));
                }
            }
            if malformed.is_some() {
                break;
            }
        }
        if malformed.is_some() {
            break;
        }
        let expected = group(event, true);
        let valid = owned.len() == 1
            && *owned[0].1 == expected["hooks"][0]
            && (event != "SessionStart" || owned[0].0["matcher"] == expected["matcher"]);
        found.insert(
            name.into(),
            json!(if owned.is_empty() {
                "missing"
            } else if valid {
                "present"
            } else {
                "stale"
            }),
        );
    }
    if let Some(error) = malformed {
        return Ok(json!({"path": display, "status": "malformed", "error": error}));
    }
    let startup_ok = found["session_start"] == "present" && found["subagent_start"] == "present";
    let stop = found["stop"].as_str().unwrap_or("missing");
    let status = if !startup_ok {
        if found["session_start"] == "stale" || found["subagent_start"] == "stale" {
            "stale"
        } else {
            "missing"
        }
    } else if stop == "stale" {
        "stale"
    } else {
        "valid"
    };
    let enforcement = if startup_ok && stop == "present" {
        "strict"
    } else if startup_ok && stop == "missing" {
        "observe"
    } else {
        "unknown"
    };
    Ok(json!({"path": display, "status": status, "enforcement": enforcement, "handlers": found}))
}

/// Read-only diagnostics for the project-scoped Codex and Claude adapters.
/// Only exact AIW command handlers are examined; unrelated handler behavior is ignored.
pub fn health(ws: &Workspace) -> Result<Value> {
    let codex_skills = json!({
        "skill": skill_health(ws, ".agents/skills/aiw-workspace/SKILL.md", SKILL)?,
        "adopt": skill_health(ws, ".agents/skills/aiw-workspace/references/adopt.md", ADOPT)?,
        "lifecycle": skill_health(ws, ".agents/skills/aiw-workspace/references/lifecycle.md", LIFECYCLE)?,
    });
    let claude_skills = json!({
        "skill": skill_health(ws, ".claude/skills/aiw-workspace/SKILL.md", SKILL)?,
        "adopt": skill_health(ws, ".claude/skills/aiw-workspace/references/adopt.md", ADOPT)?,
        "lifecycle": skill_health(ws, ".claude/skills/aiw-workspace/references/lifecycle.md", LIFECYCLE)?,
    });
    Ok(json!({
        "codex": {
            "bootstrap": bootstrap_health(ws, "AGENTS.md")?,
            "skills": codex_skills,
            "hooks": hook_health(ws, ".codex/hooks.json")?
        },
        "claude": {
            "bootstrap": bootstrap_health(ws, "CLAUDE.md")?,
            "skills": claude_skills,
            "hooks": hook_health(ws, ".claude/settings.json")?
        }
    }))
}

fn is_aiw_hook_handler(value: &Value) -> bool {
    let Some(handler) = value.as_object() else {
        return false;
    };
    handler.get("type").and_then(Value::as_str) == Some("command")
        && matches!(
            handler.get("command").and_then(Value::as_str),
            Some("aiw hook session-start" | "aiw hook stop --strict")
        )
}

fn group(event: &str, strict: bool) -> Value {
    let command = match event {
        "SessionStart" | "SubagentStart" => "aiw hook session-start",
        "Stop" if strict => "aiw hook stop --strict",
        _ => unreachable!(),
    };
    let handler = json!({
        "type": "command",
        "command": command,
        "timeout": 10
    });
    let mut value = json!({"hooks":[handler]});
    if event == "SessionStart" {
        value["matcher"] = json!("startup|resume|clear|compact");
    }
    value
}

fn remove_aiw_handlers(groups: &mut Vec<Value>, event: &str, path: &std::path::Path) -> Result<()> {
    for value in groups.iter() {
        if let Some(handlers) = value.as_object().and_then(|group| group.get("hooks")) {
            ensure!(
                handlers.is_array(),
                "hooks.{event} group hooks must be an array: {}",
                path.display()
            );
        }
    }

    let generated = group(event, true);
    groups.retain_mut(|value| {
        let was_generated = *value == generated;
        let Some(handlers) = value
            .as_object_mut()
            .and_then(|group| group.get_mut("hooks"))
            .and_then(Value::as_array_mut)
        else {
            return true;
        };
        handlers.retain(|handler| !is_aiw_hook_handler(handler));
        !was_generated || !handlers.is_empty()
    });
    Ok(())
}

fn merged_hooks(path: &std::path::Path, strict: bool) -> Result<Value> {
    let mut root = if path.exists() {
        crate::workspace::read_json::<Value>(path)?
    } else {
        json!({})
    };
    let object = root.as_object_mut().ok_or_else(|| {
        anyhow::anyhow!(
            "hook configuration must be a JSON object: {}",
            path.display()
        )
    })?;
    let hooks = object.entry("hooks").or_insert_with(|| json!({}));
    let hooks = hooks
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("hooks must be a JSON object: {}", path.display()))?;
    for event in ["SessionStart", "SubagentStart", "Stop"] {
        let groups = hooks.entry(event).or_insert_with(|| json!([]));
        let groups = groups
            .as_array_mut()
            .ok_or_else(|| anyhow::anyhow!("hooks.{event} must be an array: {}", path.display()))?;
        remove_aiw_handlers(groups, event, path)?;
        if event != "Stop" || strict {
            groups.push(group(event, strict));
        }
    }
    Ok(root)
}

pub fn integrate(ws: &Workspace, vendor: &str, strict: bool) -> Result<Vec<String>> {
    let vendors: &[&str] = match vendor {
        "codex" => &["codex"],
        "claude" => &["claude"],
        "all" => &["codex", "claude"],
        _ => anyhow::bail!("integration must be codex, claude, or all"),
    };

    // Parse every existing config before writing any integration file.
    let mut configs = Vec::new();
    for vendor in vendors {
        let relative = if *vendor == "codex" {
            ".codex/hooks.json"
        } else {
            ".claude/settings.json"
        };
        let path = ws.path(relative)?;
        configs.push((relative, merged_hooks(&path, strict)?));
    }

    let mut generated = generate(ws, vendor)?;
    for vendor in vendors {
        let root = if *vendor == "codex" {
            ".agents/skills/aiw-workspace"
        } else {
            ".claude/skills/aiw-workspace"
        };
        for (suffix, content) in [
            ("SKILL.md", SKILL),
            ("references/adopt.md", ADOPT),
            ("references/lifecycle.md", LIFECYCLE),
        ] {
            let path = format!("{root}/{suffix}");
            ws.write(&path, content.as_bytes())?;
            generated.push(path);
        }
    }
    for (path, config) in configs {
        ws.json(path, &config)?;
        generated.push(path.into());
    }
    Ok(generated)
}
