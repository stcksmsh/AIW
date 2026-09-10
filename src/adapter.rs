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

fn contains_aiw_hook(value: &Value) -> bool {
    match value {
        Value::String(value) => value.starts_with("aiw hook "),
        Value::Array(values) => values.iter().any(contains_aiw_hook),
        Value::Object(values) => values.values().any(contains_aiw_hook),
        _ => false,
    }
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
        groups.retain(|value| !contains_aiw_hook(value));
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
