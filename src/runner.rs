use crate::{
    model::{CommandSpec, Evidence, RunSummary, State, Task},
    repo,
    workspace::Workspace,
};
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    collections::{BTreeSet, VecDeque},
    fs::{self, File},
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

pub fn contract(state: &State, task: &Task) -> Result<String> {
    Ok(repo::hash(&serde_json::to_vec(&(
        &task.title,
        &task.plan,
        &state.plans[&task.plan],
        &task.dependencies,
        &task.scope,
        &task.acceptance,
        &task.constraints,
        &task.verify,
        &state.project.invariants,
    ))?))
}
pub fn health(state: &State, task: &Task, source_hash: &str) -> Result<&'static str> {
    Ok(match &task.evidence {
        None => "unverified",
        Some(e) if e.source_hash != source_hash || e.contract_hash != contract(state, task)? => {
            "stale"
        }
        Some(e) if e.success => "passed",
        Some(_) => "failed",
    })
}
pub fn run(ws: &Workspace, spec: &CommandSpec) -> Result<RunSummary> {
    spec.validate()?;
    let runtime = ws.path(".ai/runtime/runs")?;
    fs::create_dir_all(&runtime)?;
    let dir = tempfile::Builder::new()
        .prefix("run-")
        .tempdir_in(runtime)?
        .keep();
    let id = dir.file_name().unwrap().to_string_lossy().into_owned();
    let stdout = format!(".ai/runtime/runs/{id}/stdout.log");
    let stderr = format!(".ai/runtime/runs/{id}/stderr.log");
    let out = File::create(ws.path(&stdout)?)?;
    let err = File::create(ws.path(&stderr)?)?;
    let started = Instant::now();
    let mut command = Command::new(&spec.argv[0]);
    command
        .args(&spec.argv[1..])
        .current_dir(&ws.root)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .env("NO_COLOR", "1")
        .env("CARGO_TERM_COLOR", "never");
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut timed_out = false;
    let mut spawn_error = None;
    let status = match command.spawn() {
        Ok(mut child) => {
            let status = loop {
                if let Some(status) = child.try_wait()? {
                    break status;
                }
                if started.elapsed() >= Duration::from_secs(spec.timeout_seconds) {
                    timed_out = true;
                    #[cfg(unix)]
                    // The child starts its own process group. Kill descendants too.
                    unsafe {
                        libc::kill(-(child.id() as i32), libc::SIGKILL);
                    }
                    let _ = child.kill();
                    break child.wait()?;
                }
                thread::sleep(Duration::from_millis(20));
            };
            Some(status)
        }
        Err(error) => {
            spawn_error = Some(format!("cannot start {}: {error}", spec.argv[0]));
            None
        }
    };
    let (mut diagnostics, tail, omitted) = summarize(&[ws.path(&stdout)?, ws.path(&stderr)?])?;
    if let Some(error) = spawn_error {
        diagnostics.insert(0, crate::view::clip(&error, 400));
    }
    if timed_out {
        diagnostics.insert(
            0,
            format!("timed out after {} seconds", spec.timeout_seconds),
        );
    }
    let summary = RunSummary {
        schema_version: 1,
        id: id.clone(),
        argv: spec.argv.clone(),
        exit_code: status.and_then(|s| s.code()),
        success: status.is_some_and(|s| s.success()) && !timed_out,
        timed_out,
        duration_ms: started.elapsed().as_millis() as u64,
        stdout,
        stderr,
        diagnostics,
        tail,
        omitted,
    };
    ws.json(&format!(".ai/runtime/runs/{id}/result.json"), &summary)?;
    Ok(summary)
}
// Bounded chunks, not read_line: adversarial one-line logs cannot exhaust memory.
fn summarize(paths: &[std::path::PathBuf]) -> Result<(Vec<String>, Vec<String>, u64)> {
    let mut diagnostics = vec![];
    let mut seen = BTreeSet::new();
    let mut tail = VecDeque::new();
    let mut omitted = 0;
    for path in paths {
        let mut reader = BufReader::new(File::open(path)?);
        let mut line = Vec::new();
        let mut overflow = false;
        loop {
            let buf = reader.fill_buf()?;
            if buf.is_empty() {
                if !line.is_empty() {
                    collect(
                        &line,
                        overflow,
                        &mut diagnostics,
                        &mut seen,
                        &mut tail,
                        &mut omitted,
                    );
                }
                break;
            }
            let end = buf.iter().position(|b| *b == b'\n');
            let n = end.map_or(buf.len(), |i| i + 1);
            let remaining = 65536usize.saturating_sub(line.len());
            line.extend_from_slice(&buf[..n.min(remaining)]);
            if n > remaining {
                overflow = true;
            }
            reader.consume(n);
            if end.is_some() {
                collect(
                    &line,
                    overflow,
                    &mut diagnostics,
                    &mut seen,
                    &mut tail,
                    &mut omitted,
                );
                line.clear();
                overflow = false;
            }
        }
    }
    Ok((diagnostics, tail.into_iter().collect(), omitted))
}
fn collect(
    raw: &[u8],
    overflow: bool,
    diagnostics: &mut Vec<String>,
    seen: &mut BTreeSet<String>,
    tail: &mut VecDeque<String>,
    omitted: &mut u64,
) {
    let text = String::from_utf8_lossy(raw);
    let text = text.trim();
    if text.is_empty() {
        return;
    }
    if overflow {
        *omitted += 1;
    }
    let diagnostic = if let Ok(json) = serde_json::from_str::<Value>(text) {
        let message = if json["reason"] == "compiler-message" {
            &json["message"]
        } else {
            &json
        };
        if let (Some(level), Some(msg)) = (message["level"].as_str(), message["message"].as_str()) {
            let location = message["spans"]
                .as_array()
                .and_then(|s| s.iter().find(|s| s["is_primary"] == true))
                .map(|s| {
                    format!(
                        "{}:{}:{}: ",
                        s["file_name"].as_str().unwrap_or("?"),
                        s["line_start"],
                        s["column_start"]
                    )
                })
                .unwrap_or_default();
            Some(format!("{location}{level}: {msg}"))
        } else if json.get("reason").is_some() {
            return;
        } else {
            None
        }
    } else {
        None
    };
    let clean = crate::view::clip(text, 400);
    if !clean.is_empty() {
        if tail.len() == 8 {
            tail.pop_front();
        }
        tail.push_back(clean.clone());
    }
    let lower = text.to_ascii_lowercase();
    let routine_test_success = lower.starts_with("test ")
        && (lower.ends_with(" ... ok") || lower.starts_with("test result: ok."));
    let diagnostic = diagnostic.or_else(|| {
        (!routine_test_success
            && (lower.contains("error")
                || lower.contains("warning")
                || lower.contains("failed")
                || lower.contains("panicked")))
        .then_some(clean)
    });
    if let Some(d) = diagnostic {
        let d = crate::view::clip(&d, 400);
        if seen.contains(&d) {
            *omitted += 1;
        } else if diagnostics.len() < 24 {
            seen.insert(d.clone());
            diagnostics.push(d);
        } else {
            *omitted += 1;
        }
    }
}
pub fn verify(ws: &Workspace, state: &mut State, id: &str) -> Result<Evidence> {
    let task = state.tasks.get(id).context("unknown task")?;
    ensure!(
        task.status == crate::model::Status::InProgress,
        "verification requires an in-progress task"
    );
    ensure!(state.ready(id), "task dependencies are not done");
    ensure!(!task.verify.is_empty(), "task has no verification commands");
    let source_hash = repo::fingerprint(&ws.root)?;
    let contract_hash = contract(state, task)?;
    let mut runs = vec![];
    for spec in &task.verify {
        runs.push(run(ws, spec)?);
    }
    let after = repo::fingerprint(&ws.root)?;
    let success = runs.iter().all(|r| r.success) && after == source_hash;
    let evidence = Evidence {
        source_hash,
        contract_hash,
        success,
        runs,
    };
    // Verification commands must not mutate canonical state through another path.
    ensure!(
        serde_json::to_vec(&ws.read()?)? == serde_json::to_vec(state)?,
        "canonical state changed during verification; evidence not saved"
    );
    state.tasks.get_mut(id).unwrap().evidence = Some(evidence.clone());
    ws.save(state)?;
    if after != evidence.source_hash {
        eprintln!("source changed during verification; evidence is stale, rerun");
    }
    Ok(evidence)
}
pub fn event(ws: &Workspace, command: &str, success: bool, elapsed: u128) -> Result<()> {
    let path = ws.path(".ai/runtime/events.jsonl")?;
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let value = serde_json::json!({"schema_version":1,"command":command,"success":success,"duration_ms":elapsed,"agent":std::env::var("AIW_AGENT").ok(),"provider":std::env::var("AIW_PROVIDER").ok(),"model":std::env::var("AIW_MODEL").ok()});
    writeln!(f, "{value}")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn successful_test_names_and_zero_failures_are_not_diagnostics() {
        let mut diagnostics = vec![];
        let mut seen = BTreeSet::new();
        let mut tail = VecDeque::new();
        let mut omitted = 0;
        for line in [
            "test schema_errors_fail_without_writes ... ok",
            "test result: ok. 30 passed; 0 failed; 0 ignored",
        ] {
            collect(
                line.as_bytes(),
                false,
                &mut diagnostics,
                &mut seen,
                &mut tail,
                &mut omitted,
            );
        }
        assert!(diagnostics.is_empty());
        assert_eq!(tail.len(), 2);
    }
    #[test]
    fn direct_rustc_json_keeps_primary_location_and_drops_cargo_artifact_chatter() {
        let mut diagnostics = vec![];
        let mut seen = BTreeSet::new();
        let mut tail = VecDeque::new();
        let mut omitted = 0;
        let message = serde_json::json!({"$message_type":"diagnostic","level":"warning","message":"unused variable","spans":[{"file_name":"src/lib.rs","line_start":8,"column_start":2,"is_primary":false},{"file_name":"src/lib.rs","line_start":9,"column_start":5,"is_primary":true}]});
        collect(
            message.to_string().as_bytes(),
            false,
            &mut diagnostics,
            &mut seen,
            &mut tail,
            &mut omitted,
        );
        collect(
            br#"{"reason":"compiler-artifact","filenames":["noise"]}"#,
            false,
            &mut diagnostics,
            &mut seen,
            &mut tail,
            &mut omitted,
        );
        assert_eq!(
            diagnostics,
            vec!["src/lib.rs:9:5: warning: unused variable"]
        );
        assert_eq!(tail.len(), 1);
    }
    #[test]
    fn diagnostic_storm_has_bounded_unique_storage_and_visible_omission_count() {
        let mut diagnostics = vec![];
        let mut seen = BTreeSet::new();
        let mut tail = VecDeque::new();
        let mut omitted = 0;
        for n in 0..10000 {
            collect(
                format!("error: failure {n}").as_bytes(),
                false,
                &mut diagnostics,
                &mut seen,
                &mut tail,
                &mut omitted,
            );
        }
        assert_eq!(diagnostics.len(), 24);
        assert_eq!(seen.len(), 24);
        assert_eq!(tail.len(), 8);
        assert_eq!(omitted, 9976);
    }
}
