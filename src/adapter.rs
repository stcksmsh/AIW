use crate::workspace::Workspace;
use anyhow::{Result, ensure};
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
        "{BEGIN}\n# AIW bootstrap\n\nRun `aiw load` from this repository before exploring. If AIW is not installed,\ninstall the Rust reference CLI with `cargo install --path . --locked` when this\nis the AIW repository, or follow the AIW installation instructions.\nRead `.ai/policy.md` once, then use `aiw show task ID` for the selected task.\nCanonical project state is `.ai/state.json`; decisions are `.ai/decisions/`.\nUse `aiw verify ID --allow-exec` for declared checks after reviewing commands.\nPersist results with `aiw task transition` and next actions with `aiw checkpoint`.\nThis block is generated; vendor-specific guidance belongs outside the markers.\n{END}"
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
        let next = match (old.find(BEGIN), old.find(END)) {
            (Some(start), Some(end)) => {
                ensure!(end > start, "malformed AIW markers in {path}");
                format!("{}{}{}", &old[..start], body, &old[end + END.len()..])
            }
            (None, None) => format!(
                "{}{}{}\n",
                old,
                if old.is_empty() {
                    ""
                } else if old.ends_with('\n') {
                    "\n"
                } else {
                    "\n\n"
                },
                body
            ),
            _ => anyhow::bail!("incomplete AIW markers in {path}; resolve manually"),
        };
        writes.push((*path, next));
    }
    for (path, next) in writes {
        ws.write(path, next.as_bytes())?;
    }
    Ok(paths.into_iter().map(str::to_string).collect())
}
