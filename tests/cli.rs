use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::Path,
    process::{Command, Output, Stdio},
};
use tempfile::TempDir;
const BIN: &str = env!("CARGO_BIN_EXE_aiw");
struct Repo(TempDir);
impl Repo {
    fn new(git: bool) -> Self {
        let r = Self(tempfile::tempdir().unwrap());
        if git {
            r.git(&["init", "-q"]);
            r.git(&["config", "user.name", "AIW Test"]);
            r.git(&["config", "user.email", "test@example.invalid"]);
        }
        r.ok(&["init", "--name", "Fixture"]);
        r
    }
    fn path(&self) -> &Path {
        self.0.path()
    }
    fn call(&self, args: &[&str]) -> Output {
        Command::new(BIN)
            .args(args)
            .current_dir(self.path())
            .env_remove("AIW_AGENT")
            .output()
            .unwrap()
    }
    fn call_stdin(&self, args: &[&str], input: &Value) -> Output {
        let mut child = Command::new(BIN)
            .args(args)
            .current_dir(self.path())
            .env_remove("AIW_AGENT")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(serde_json::to_string(input).unwrap().as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    }
    fn ok(&self, args: &[&str]) -> String {
        let out = self.call(args);
        assert!(
            out.status.success(),
            "{args:?}: {}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }
    fn err(&self, args: &[&str], needle: &str) {
        let out = self.call(args);
        assert!(!out.status.success(), "unexpected success: {args:?}");
        assert!(
            String::from_utf8_lossy(&out.stderr).contains(needle),
            "expected {needle}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    fn json(&self, args: &[&str]) -> Value {
        serde_json::from_str(&self.ok(args)).unwrap()
    }
    fn write(&self, path: &str, content: &str) {
        let p = self.path().join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, content).unwrap();
    }
    fn state(&self) -> Value {
        serde_json::from_slice(&fs::read(self.path().join(".ai/state.json")).unwrap()).unwrap()
    }
    fn save(&self, state: &Value) {
        self.write(
            ".ai/state.json",
            &serde_json::to_string_pretty(state).unwrap(),
        );
    }
    fn plan(&self) {
        self.ok(&[
            "plan",
            "add",
            "p",
            "First version",
            "--objective",
            "Prove durable recovery",
        ]);
    }
    fn task(&self, id: &str, deps: &[&str]) {
        let verify = serde_json::to_string(&[BIN, "--version"]).unwrap();
        let mut args = vec![
            "task",
            "add",
            id,
            "Bounded task",
            "--plan",
            "p",
            "--accept",
            "CLI version exits successfully",
            "--scope",
            "src",
            "--constraint",
            "Keep canonical state neutral",
            "--verify",
            &verify,
        ];
        for dep in deps {
            args.extend(["--depends", dep]);
        }
        self.ok(&args);
    }
    fn done(&self, id: &str) {
        self.ok(&["task", "claim", id, "--worker", "fixture"]);
        self.ok(&["verify", id, "--allow-exec"]);
        self.ok(&[
            "task",
            "transition",
            id,
            "done",
            "--result",
            "Acceptance passed",
        ]);
    }
    fn git(&self, args: &[&str]) {
        let o = Command::new("git")
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    }
}

#[test]
fn init_is_idempotent_and_preserves_policy() {
    let r = Repo::new(false);
    r.write(".ai/policy.md", "My policy\n");
    let before = r.state();
    r.ok(&["init", "--name", "Replacement"]);
    assert_eq!(before, r.state());
    assert_eq!(
        fs::read_to_string(r.path().join(".ai/policy.md")).unwrap(),
        "My policy\n"
    );
    assert_eq!(r.json(&["show", "inspection"])["git"], false);
}
#[test]
fn nested_init_uses_git_root_and_worktree_git_file() {
    let r = Repo::new(true);
    r.write("sub/file.rs", "fn main() {}\n");
    let out = Command::new(BIN)
        .arg("init")
        .current_dir(r.path().join("sub"))
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(!r.path().join("sub/.ai").exists());
    r.git(&["add", "."]);
    r.git(&["commit", "-qm", "fixture"]);
    let work = tempfile::tempdir().unwrap();
    let dest = work.path().join("worker");
    r.git(&["worktree", "add", "-qb", "worker", dest.to_str().unwrap()]);
    let out = Command::new(BIN)
        .arg("load")
        .current_dir(&dest)
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(dest.join(".git").is_file());
}
#[test]
fn schema_errors_fail_without_canonical_writes() {
    for change in [
        json!({"schema_version":99}),
        json!({"schema_version":0}),
        json!({"schema_version":"1"}),
        json!({"schema_version":1,"unknown":true}),
    ] {
        let r = Repo::new(false);
        let mut s = r.state();
        for (k, v) in change.as_object().unwrap() {
            s[k] = v.clone();
        }
        r.save(&s);
        let before = fs::read(r.path().join(".ai/state.json")).unwrap();
        assert!(!r.call(&["status"]).status.success());
        assert!(!r.call(&["init"]).status.success());
        assert_eq!(before, fs::read(r.path().join(".ai/state.json")).unwrap());
    }
}
#[test]
fn malformed_json_is_diagnosed() {
    let r = Repo::new(false);
    r.write(".ai/state.json", "{broken");
    r.err(&["load"], "invalid JSON");
    r.err(&["doctor"], "validation failed");
}
#[test]
fn dependency_dag_and_claims_are_enforced() {
    let r = Repo::new(false);
    r.plan();
    r.task("a", &[]);
    r.task("b", &["a"]);
    r.err(&["task", "claim", "b", "--worker", "codex"], "dependencies");
    assert_eq!(r.json(&["list", "tasks", "--ready"])["total"], 1);
    r.done("a");
    r.ok(&["task", "claim", "b", "--worker", "codex"]);
    r.ok(&["task", "claim", "b", "--worker", "codex"]);
    r.err(&["task", "claim", "b", "--worker", "claude"], "claimed");
    r.err(&["task", "transition", "a", "pending"], "dependent");
}
#[test]
fn cycles_missing_references_and_duplicate_dependencies_fail() {
    let r = Repo::new(false);
    r.plan();
    r.task("a", &[]);
    r.task("b", &["a"]);
    let original = r.state();
    for deps in [
        json!(["b"]),
        json!(["missing"]),
        json!(["a"]),
        json!(["b", "b"]),
    ] {
        let mut s = original.clone();
        s["tasks"]["a"]["dependencies"] = deps;
        r.save(&s);
        assert!(!r.call(&["load"]).status.success());
    }
}
#[test]
fn transitions_need_evidence_result_and_blocker() {
    let r = Repo::new(false);
    r.plan();
    r.task("a", &[]);
    r.err(
        &["task", "transition", "a", "done", "--result", "done"],
        "fresh successful verification",
    );
    r.err(&["task", "transition", "a", "blocked"], "reason");
    r.ok(&[
        "task",
        "transition",
        "a",
        "blocked",
        "--reason",
        "Need fixture",
    ]);
    r.ok(&["task", "transition", "a", "pending"]);
    r.ok(&["task", "claim", "a", "--worker", "claude"]);
    r.err(&["verify", "a"], "--allow-exec");
    r.ok(&["verify", "a", "--allow-exec"]);
    r.err(&["task", "transition", "a", "done"], "--result");
    r.ok(&["task", "transition", "a", "done", "--result", "passed"]);
    r.ok(&["task", "transition", "a", "done"]);
    r.ok(&["task", "transition", "a", "pending"]);
    assert!(r.state()["tasks"]["a"]["evidence"].is_null());
}
#[test]
fn source_and_contract_changes_invalidate_evidence() {
    let r = Repo::new(true);
    r.plan();
    r.task("a", &[]);
    r.write("src/lib.rs", "fn original() {}\n");
    r.ok(&["task", "claim", "a", "--worker", "test"]);
    r.ok(&["verify", "a", "--allow-exec"]);
    assert_eq!(r.json(&["status"])["verification"], "passed");
    r.write("src/lib.rs", "fn modified() {}\n");
    assert_eq!(r.json(&["status"])["verification"], "stale");
    r.err(
        &["task", "transition", "a", "done", "--result", "done"],
        "fresh",
    );
    r.ok(&["verify", "a", "--allow-exec"]);
    let mut s = r.state();
    s["tasks"]["a"]["acceptance"] = json!(["Different acceptance"]);
    r.save(&s);
    assert_eq!(r.json(&["status"])["verification"], "stale");
}
#[test]
fn incremental_index_detects_same_size_edits_and_deletions() {
    let r = Repo::new(true);
    r.write("src/a.rs", "fn alpha() {}\n");
    r.write(".gitignore", "ignored/\n");
    r.write("ignored/key.txt", "not searchable");
    r.ok(&["index"]);
    let second = r.json(&["index"]);
    assert_eq!(second["rebuilt"], 0);
    r.write("src/a.rs", "fn bravo() {}\n");
    assert_eq!(r.json(&["index"])["rebuilt"], 1);
    assert_eq!(r.json(&["probe", "bravo"])["hits"][0]["path"], "src/a.rs");
    assert!(
        r.json(&["probe", "not searchable"])["hits"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    fs::remove_file(r.path().join("src/a.rs")).unwrap();
    assert_eq!(r.json(&["index"])["removed"], 1);
}
#[test]
fn corrupt_derived_index_rebuilds_without_affecting_state() {
    let r = Repo::new(false);
    let s = r.state();
    r.write(".ai/derived/index.json", "broken");
    r.ok(&["index"]);
    assert_eq!(s, r.state());
    fs::remove_dir_all(r.path().join(".ai/runtime")).unwrap();
    r.ok(&["load"]);
}
#[test]
fn deterministic_views_and_pagination() {
    let r = Repo::new(false);
    r.plan();
    r.task("a", &[]);
    r.task("b", &[]);
    assert_eq!(r.ok(&["load"]), r.ok(&["load"]));
    assert_eq!(r.ok(&["status"]), r.ok(&["status"]));
    let page = r.json(&["list", "tasks", "--limit", "1"]);
    assert_eq!(page["next_offset"], 1);
    assert_eq!(
        r.json(&["list", "tasks", "--limit", "1", "--offset", "1"])["items"][0]["id"],
        "b"
    );
}
#[test]
fn recovery_remains_bounded_after_large_history() {
    let r = Repo::new(false);
    r.plan();
    r.task("current", &[]);
    r.ok(&["task", "focus", "current"]);
    let mut s = r.state();
    let mut old = s["tasks"]["current"].clone();
    old["status"] = json!("cancelled");
    old["title"] = json!("OBSOLETE-HISTORY".repeat(100));
    for i in 0..1500 {
        s["tasks"][format!("old-{i:04}")] = old.clone();
    }
    s["project"]["description"] = json!("🦀".repeat(1000));
    r.save(&s);
    let output = r.ok(&["load", "--budget", "2048"]);
    assert!(output.len() <= 2048);
    assert!(output.contains("Task current"));
    assert!(!output.contains("OBSOLETE-HISTORY"));
    r.err(&["load", "--budget", "128"], "budget");
}
#[test]
fn checkpoint_freshness_tracks_scoped_source_without_refreshing_evidence() {
    for git in [false, true] {
        let r = Repo::new(git);
        r.plan();
        r.task("active", &[]);
        r.write("src/lib.rs", "original\n");
        r.write(".gitignore", "src/ignored/\n");
        if git {
            r.git(&["add", "."]);
            r.git(&["commit", "-qm", "fixture"]);
        }
        r.ok(&["task", "claim", "active", "--worker", "fixture"]);
        assert_eq!(r.json(&["status"])["checkpoint_freshness"], "missing");
        r.ok(&["verify", "active", "--allow-exec"]);
        let evidence = r.state()["tasks"]["active"]["evidence"].clone();
        let checkpoint = [
            "checkpoint",
            "--next",
            "Review scoped source",
            "--note",
            "Baseline",
        ];
        r.ok(&checkpoint);
        let baseline = r.state()["checkpoint"].clone();
        assert_eq!(baseline["source_hash"].as_str().unwrap().len(), 64);
        r.ok(&checkpoint);
        assert_eq!(r.state()["checkpoint"], baseline);
        assert_eq!(r.json(&["status"])["checkpoint_freshness"], "unchanged");
        assert!(r.ok(&["load"]).contains("Checkpoint source: unchanged"));

        // These writes must not count as scoped source changes.
        for path in [
            "src/ignored/cache",
            "src/target/cache",
            ".ai/runtime/cache",
            ".ai/derived/cache",
            "src-other/lib.rs",
        ] {
            r.write(path, "ignored by checkpoint comparison\n");
        }
        assert_eq!(r.json(&["status"])["checkpoint_freshness"], "unchanged");
        // Verification still covers discovered files outside task scope.
        assert_eq!(r.json(&["status"])["verification"], "stale");

        r.write("src/lib.rs", "changed\n");
        let before_read = r.state();
        assert_eq!(r.json(&["status"])["checkpoint_freshness"], "changed");
        assert!(
            r.ok(&["load", "--budget", "2048"])
                .contains("Checkpoint source: changed")
        );
        let stop = r.call_stdin(&["hook", "stop", "--strict"], &json!({}));
        assert_eq!(stop.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&stop.stderr).contains("persist a checkpoint"));
        assert!(
            r.call_stdin(
                &["hook", "stop", "--strict"],
                &json!({"stop_hook_active":true})
            )
            .status
            .success()
        );
        assert!(r.call_stdin(&["hook", "stop"], &json!({})).status.success());
        assert_eq!(r.state(), before_read);

        r.write("src/lib.rs", "original\n");
        assert_eq!(r.json(&["status"])["checkpoint_freshness"], "unchanged");
        fs::remove_file(r.path().join("src/lib.rs")).unwrap();
        assert_eq!(r.json(&["status"])["checkpoint_freshness"], "changed");
        r.write("src/lib.rs", "original\n");
        r.write("src/new.rs", "added\n");
        assert_eq!(r.json(&["status"])["checkpoint_freshness"], "changed");
        fs::remove_file(r.path().join("src/new.rs")).unwrap();
        assert_eq!(r.json(&["status"])["checkpoint_freshness"], "unchanged");

        r.write("src/lib.rs", "changed again\n");
        r.ok(&checkpoint);
        assert_ne!(
            r.state()["checkpoint"]["source_hash"],
            baseline["source_hash"]
        );
        assert_eq!(r.json(&["status"])["checkpoint_freshness"], "unchanged");
        assert_eq!(r.state()["tasks"]["active"]["evidence"], evidence);
        r.err(
            &[
                "task",
                "transition",
                "active",
                "done",
                "--result",
                "Checkpoint is not evidence",
            ],
            "fresh successful verification",
        );
    }
}
#[test]
fn checkpoint_freshness_handles_legacy_explicit_and_taskless_checkpoints() {
    let r = Repo::new(false);
    assert!(r.json(&["status"])["checkpoint_freshness"].is_null());
    r.ok(&["checkpoint", "--next", "Select a task"]);
    assert!(r.state()["checkpoint"]["source_hash"].is_null());
    r.plan();
    r.task("active", &[]);
    r.task("other", &[]);
    r.ok(&["task", "focus", "active"]);
    r.ok(&["checkpoint", "--task", "other", "--next", "Inspect other"]);
    assert_eq!(r.state()["checkpoint"]["task"], "other");
    assert_eq!(r.json(&["status"])["checkpoint_freshness"], "missing");
    assert!(
        r.ok(&["load", "--task", "other"])
            .contains("Checkpoint source: unchanged")
    );
    let before = r.state();
    r.err(
        &["checkpoint", "--task", "absent", "--next", "Inspect"],
        "missing task",
    );
    r.err(&["checkpoint", "--next", ""], "next action");
    assert_eq!(r.state(), before);

    let mut legacy = r.state();
    legacy["checkpoint"] =
        json!({"task":"active", "next_action":"Inspect source", "note":"Old client"});
    r.save(&legacy);
    assert_eq!(r.json(&["status"])["checkpoint_freshness"], "unknown");
    assert!(r.ok(&["load"]).contains("Checkpoint source: unknown"));
    r.ok(&["task", "claim", "active", "--worker", "fixture"]);
    assert!(r.state()["checkpoint"]["source_hash"].is_null());
    for invalid in [json!("bad hash"), json!("z".repeat(64))] {
        let mut state = legacy.clone();
        state["checkpoint"]["source_hash"] = invalid;
        r.save(&state);
        r.err(&["status"], "invalid checkpoint fingerprint");
    }
    let mut taskless = legacy.clone();
    taskless["checkpoint"]["task"] = Value::Null;
    taskless["checkpoint"]["source_hash"] = json!("a".repeat(64));
    r.save(&taskless);
    r.err(&["status"], "checkpoint fingerprint requires a task");
    r.save(&legacy);
    r.ok(&["checkpoint", "--next", "Capture baseline"]);
    assert_eq!(r.json(&["status"])["checkpoint_freshness"], "unchanged");
}
#[test]
fn checkpoint_freshness_matches_file_boundaries_and_empty_scope() {
    let r = Repo::new(false);
    r.plan();
    r.task("active", &[]);
    r.ok(&["task", "focus", "active"]);
    r.write("src/lib.rs", "original");
    let mut state = r.state();
    state["tasks"]["active"]["scope"] = json!(["src/lib.rs"]);
    r.save(&state);
    r.ok(&["checkpoint", "--next", "Inspect single file"]);
    r.write("src/lib.rs.extra", "outside exact file scope");
    r.write("src/other.rs", "outside exact file scope");
    assert_eq!(r.json(&["status"])["checkpoint_freshness"], "unchanged");
    r.write("src/lib.rs", "modified");
    assert_eq!(r.json(&["status"])["checkpoint_freshness"], "changed");

    let mut state = r.state();
    state["tasks"]["active"]["scope"] = json!([]);
    r.save(&state);
    r.ok(&["checkpoint", "--next", "Inspect workspace source"]);
    r.write("docs/new.md", "included with empty scope");
    assert_eq!(r.json(&["status"])["checkpoint_freshness"], "changed");
}
#[test]
fn provider_handoff_uses_only_canonical_state() {
    let r = Repo::new(true);
    r.plan();
    r.task("bridge", &[]);
    r.ok(&["adapter"]);
    r.ok(&["task", "claim", "bridge", "--worker", "claude"]);
    r.ok(&[
        "checkpoint",
        "--next",
        "Run declared checks and persist a result",
        "--note",
        "Implementation ready; no conversation required",
    ]);
    // New independent processes stand in for clients with no conversation memory.
    let packet = r.ok(&["load"]);
    for expected in [
        "Fixture",
        "Plan p",
        "Task bridge",
        "Keep canonical state neutral",
        "CLI version exits",
        "Verify argv",
        "Run declared checks",
    ] {
        assert!(packet.contains(expected), "missing {expected}");
    }
    r.ok(&[
        "task",
        "transition",
        "bridge",
        "blocked",
        "--reason",
        "Claude session disappeared",
    ]);
    r.ok(&["task", "transition", "bridge", "pending"]);
    r.ok(&["task", "claim", "bridge", "--worker", "codex"]);
    r.ok(&["verify", "bridge", "--allow-exec"]);
    r.ok(&[
        "task",
        "transition",
        "bridge",
        "done",
        "--result",
        "Codex checked acceptance",
    ]);
    fs::remove_dir_all(r.path().join(".ai/runtime")).unwrap();
    assert!(
        r.ok(&["show", "task", "bridge"])
            .contains("Codex checked acceptance")
    );
    assert!(
        r.ok(&["load", "--task", "bridge"])
            .contains("missing/disposable")
    );
}
#[test]
fn adapter_blocks_preserve_user_content_and_are_idempotent() {
    let r = Repo::new(false);
    r.write("AGENTS.md", "User policy\n");
    r.ok(&["adapter"]);
    let before = fs::read(r.path().join("AGENTS.md")).unwrap();
    r.ok(&["adapter"]);
    assert_eq!(before, fs::read(r.path().join("AGENTS.md")).unwrap());
    let content = String::from_utf8(before).unwrap();
    assert!(content.starts_with("<!-- aiw:begin generated v1 -->"));
    assert!(content.ends_with("User policy\n"));
    r.write("CLAUDE.md", "<!-- aiw:begin generated v1 -->\npartial");
    r.err(&["adapter"], "incomplete");
}
#[test]
fn integrate_installs_skills_and_merges_hooks_idempotently() {
    let r = Repo::new(false);
    r.write(
        ".codex/hooks.json",
        r#"{"custom":true,"hooks":{"SessionStart":[{"hooks":[{"type":"command","command":"echo existing"}]}],"Stop":[{"hooks":[{"type":"command","command":"echo stop"}]}]}}"#,
    );
    r.write(
        ".claude/settings.json",
        r#"{"permissions":{"allow":["Read"]}}"#,
    );
    r.ok(&["integrate", "all", "--enforcement", "strict"]);

    for path in [
        ".agents/skills/aiw-workspace/SKILL.md",
        ".agents/skills/aiw-workspace/references/adopt.md",
        ".agents/skills/aiw-workspace/references/lifecycle.md",
        ".claude/skills/aiw-workspace/SKILL.md",
        ".claude/skills/aiw-workspace/references/adopt.md",
        ".claude/skills/aiw-workspace/references/lifecycle.md",
    ] {
        assert!(r.path().join(path).is_file(), "missing {path}");
    }
    let codex: Value =
        serde_json::from_slice(&fs::read(r.path().join(".codex/hooks.json")).unwrap()).unwrap();
    let claude: Value =
        serde_json::from_slice(&fs::read(r.path().join(".claude/settings.json")).unwrap()).unwrap();
    assert_eq!(codex["custom"], true);
    assert_eq!(codex["hooks"]["SessionStart"].as_array().unwrap().len(), 2);
    assert_eq!(codex["hooks"]["Stop"].as_array().unwrap().len(), 2);
    assert_eq!(claude["permissions"]["allow"][0], "Read");
    assert_eq!(claude["hooks"]["Stop"].as_array().unwrap().len(), 1);

    let before = [
        fs::read(r.path().join("AGENTS.md")).unwrap(),
        fs::read(r.path().join(".codex/hooks.json")).unwrap(),
        fs::read(r.path().join(".claude/settings.json")).unwrap(),
    ];
    r.ok(&["integrate", "all", "--enforcement", "strict"]);
    assert_eq!(before[0], fs::read(r.path().join("AGENTS.md")).unwrap());
    assert_eq!(
        before[1],
        fs::read(r.path().join(".codex/hooks.json")).unwrap()
    );
    assert_eq!(
        before[2],
        fs::read(r.path().join(".claude/settings.json")).unwrap()
    );

    r.ok(&["integrate", "all", "--enforcement", "observe"]);
    let codex: Value =
        serde_json::from_slice(&fs::read(r.path().join(".codex/hooks.json")).unwrap()).unwrap();
    assert_eq!(codex["hooks"]["Stop"].as_array().unwrap().len(), 1);
    assert_eq!(
        codex["hooks"]["Stop"][0]["hooks"][0]["command"],
        "echo stop"
    );
}
#[test]
fn integrate_preserves_coinstalled_handlers_for_codex_and_claude() {
    let r = Repo::new(false);
    let initial = json!({
        "custom": {"owner": "third-party"},
        "hooks": {
            "SessionStart": [{
                "matcher": "custom-start",
                "label": "installed-first",
                "hooks": [{"type": "command", "command": "third-party before"}]
            }],
            "Stop": [{
                "label": "installed-first",
                "hooks": [{"type": "command", "command": "third-party stop before"}]
            }]
        }
    });
    for path in [".codex/hooks.json", ".claude/settings.json"] {
        r.write(path, &initial.to_string());
    }
    r.ok(&["integrate", "all", "--enforcement", "strict"]);

    for path in [".codex/hooks.json", ".claude/settings.json"] {
        let mut config: Value =
            serde_json::from_slice(&fs::read(r.path().join(path)).unwrap()).unwrap();
        for event in ["SessionStart", "Stop"] {
            let groups = config["hooks"][event].as_array_mut().unwrap();
            let aiw_group = groups
                .iter_mut()
                .find(|group| {
                    group["hooks"].as_array().is_some_and(|handlers| {
                        handlers.iter().any(|handler| {
                            handler["command"]
                                .as_str()
                                .is_some_and(|command| command.starts_with("aiw hook "))
                        })
                    })
                })
                .unwrap();
            aiw_group["label"] = json!("installed-after");
            aiw_group["note"] = json!("aiw hook text here is not a handler");
            aiw_group["hooks"]
                .as_array_mut()
                .unwrap()
                .push(json!({"type": "command", "command": format!("third-party {event} after")}));
        }
        r.write(path, &serde_json::to_string_pretty(&config).unwrap());
    }

    r.ok(&["integrate", "all", "--enforcement", "strict"]);
    for path in [".codex/hooks.json", ".claude/settings.json"] {
        let config: Value =
            serde_json::from_slice(&fs::read(r.path().join(path)).unwrap()).unwrap();
        assert_eq!(config["custom"]["owner"], "third-party");
        for event in ["SessionStart", "Stop"] {
            let groups = config["hooks"][event].as_array().unwrap();
            let shared = groups
                .iter()
                .find(|group| group["label"] == "installed-after")
                .unwrap();
            assert_eq!(shared["note"], "aiw hook text here is not a handler");
            assert_eq!(shared["hooks"].as_array().unwrap().len(), 1);
            assert_eq!(
                shared["hooks"][0]["command"],
                format!("third-party {event} after")
            );
            assert!(
                groups
                    .iter()
                    .any(|group| group["label"] == "installed-first")
            );
            assert_eq!(
                groups
                    .iter()
                    .flat_map(|group| group["hooks"].as_array().into_iter().flatten())
                    .filter(|handler| handler["command"]
                        .as_str()
                        .is_some_and(|command| command.starts_with("aiw hook ")))
                    .count(),
                1
            );
        }
    }

    r.ok(&["integrate", "all", "--enforcement", "observe"]);
    for path in [".codex/hooks.json", ".claude/settings.json"] {
        let config: Value =
            serde_json::from_slice(&fs::read(r.path().join(path)).unwrap()).unwrap();
        let stop = config["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2);
        assert!(stop.iter().any(|group| group["label"] == "installed-first"));
        let shared = stop
            .iter()
            .find(|group| group["label"] == "installed-after")
            .unwrap();
        assert_eq!(shared["hooks"].as_array().unwrap().len(), 1);
        assert_eq!(shared["hooks"][0]["command"], "third-party Stop after");
    }
}
#[test]
fn integrate_validates_hook_configs_before_writing() {
    let r = Repo::new(false);
    r.write(".codex/hooks.json", "[]");
    r.err(&["integrate", "codex"], "must be a JSON object");
    assert!(!r.path().join("AGENTS.md").exists());
    assert!(!r.path().join(".agents/skills/aiw-workspace").exists());

    let r = Repo::new(false);
    r.write(".codex/hooks.json", r#"{"custom":true}"#);
    r.write(
        ".claude/settings.json",
        r#"{"hooks":{"SessionStart":[{"hooks":"not-an-array"}]}}"#,
    );
    r.err(&["integrate", "all"], "group hooks must be an array");
    assert_eq!(
        fs::read_to_string(r.path().join(".codex/hooks.json")).unwrap(),
        r#"{"custom":true}"#
    );
    assert!(!r.path().join("AGENTS.md").exists());
    assert!(!r.path().join("CLAUDE.md").exists());
    assert!(!r.path().join(".agents/skills/aiw-workspace").exists());
    assert!(!r.path().join(".claude/skills/aiw-workspace").exists());
}
#[test]
fn lifecycle_hooks_load_context_and_guard_unfinished_work_once() {
    let r = Repo::new(false);
    r.plan();
    r.task("active", &[]);
    r.ok(&["task", "focus", "active"]);
    let cwd = r.path().to_str().unwrap();

    let start = r.call_stdin(&["hook", "session-start"], &json!({"cwd":cwd}));
    assert!(start.status.success());
    let packet = String::from_utf8(start.stdout).unwrap();
    assert!(packet.contains("AIW lifecycle integration is active"));
    assert!(packet.contains("Task active"));

    let stop = r.call_stdin(
        &["hook", "stop", "--strict"],
        &json!({"cwd":cwd,"stop_hook_active":false}),
    );
    assert_eq!(stop.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&stop.stderr).contains("active task active is pending"));

    let repeated = r.call_stdin(
        &["hook", "stop", "--strict"],
        &json!({"cwd":cwd,"stop_hook_active":true}),
    );
    assert!(repeated.status.success());

    r.ok(&[
        "task",
        "transition",
        "active",
        "blocked",
        "--reason",
        "Need an unavailable fixture",
    ]);
    let blocked = r.call_stdin(
        &["hook", "stop", "--strict"],
        &json!({"cwd":cwd,"stop_hook_active":false}),
    );
    assert!(blocked.status.success());
}
#[test]
fn commands_preserve_argv_and_capture_failure() {
    let r = Repo::new(false);
    let o = r.call(&[
        "run",
        "--",
        BIN,
        "unknown-command",
        "literal $HOME ; & argument",
    ]);
    assert!(!o.status.success());
    let result: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(result["argv"][2], "literal $HOME ; & argument");
    assert_eq!(result["success"], false);
    assert!(r.path().join(result["stderr"].as_str().unwrap()).is_file());
    assert!(!result["diagnostics"].as_array().unwrap().is_empty());
    let missing = r.call(&["run", "--", "aiw-deliberately-missing-executable"]);
    let v: Value = serde_json::from_slice(&missing.stdout).unwrap();
    assert!(v["exit_code"].is_null());
    assert!(
        v["diagnostics"][0]
            .as_str()
            .unwrap()
            .contains("cannot start")
    );
}
#[test]
fn logs_can_be_shown_and_events_are_optional() {
    let r = Repo::new(false);
    assert!(!r.path().join(".ai/runtime/events.jsonl").exists());
    let result = r.json(&["--events", "run", "--", BIN, "--version"]);
    let id = result["id"].as_str().unwrap();
    assert_eq!(r.json(&["show", "run", id]), result);
    let events = fs::read_to_string(r.path().join(".ai/runtime/events.jsonl")).unwrap();
    let e: Value = serde_json::from_str(events.trim()).unwrap();
    assert_eq!(e["success"], true);
}
#[test]
fn lock_contention_fails_without_corruption() {
    let r = Repo::new(false);
    let f = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(r.path().join(".ai/runtime/write.lock"))
        .unwrap();
    f.try_lock().unwrap();
    r.err(
        &["plan", "add", "p", "Plan", "--objective", "Test"],
        "workspace busy",
    );
    r.ok(&["load"]);
    assert!(r.state()["plans"].as_object().unwrap().is_empty());
}
#[test]
fn invalid_ids_scopes_and_empty_contracts_are_rejected() {
    let r = Repo::new(false);
    r.plan();
    r.err(
        &[
            "task", "add", "../oops", "Bad", "--plan", "p", "--accept", "ok",
        ],
        "invalid ID",
    );
    r.err(
        &[
            "task",
            "add",
            "t",
            "Bad",
            "--plan",
            "p",
            "--accept",
            "ok",
            "--scope",
            "../outside",
        ],
        "relative",
    );
    r.task("valid", &[]);
    let mut s = r.state();
    s["tasks"]["valid"]["acceptance"] = json!([]);
    r.save(&s);
    r.err(&["load"], "acceptance");
}
#[cfg(unix)]
mod unix {
    use super::*;
    #[test]
    fn managed_symlinks_cannot_write_outside_workspace() {
        let r = Repo::new(false);
        let outside = tempfile::tempdir().unwrap();
        fs::remove_dir_all(r.path().join(".ai/runtime")).unwrap();
        std::os::unix::fs::symlink(outside.path(), r.path().join(".ai/runtime")).unwrap();
        r.err(&["index"], "symlink");
        assert!(fs::read_dir(outside.path()).unwrap().next().is_none());
    }
    #[test]
    fn index_does_not_follow_source_symlinks() {
        let r = Repo::new(true);
        let outside = tempfile::NamedTempFile::new().unwrap();
        fs::write(outside.path(), "outside secret marker").unwrap();
        std::os::unix::fs::symlink(outside.path(), r.path().join("linked.rs")).unwrap();
        r.ok(&["index"]);
        assert!(
            r.json(&["probe", "outside secret marker"])["hits"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    #[test]
    fn timeout_and_source_mutating_verification_fail() {
        let r = Repo::new(false);
        let out = r.call(&["run", "--timeout", "1", "--", "sh", "-c", "sleep 10"]);
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["timed_out"], true);
        assert!(!out.status.success());
        r.plan();
        r.task("a", &[]);
        let mut s = r.state();
        s["tasks"]["a"]["verify"] =
            json!([{"argv":["sh","-c","echo changed > source.txt"],"timeout_seconds":10}]);
        r.save(&s);
        r.ok(&["task", "claim", "a", "--worker", "test"]);
        r.err(&["verify", "a", "--allow-exec"], "source changed");
        assert_eq!(r.state()["tasks"]["a"]["evidence"]["success"], false);
    }
    #[test]
    fn structured_diagnostics_are_deduplicated_and_large_lines_bounded() {
        let r = Repo::new(false);
        let message=json!({"reason":"compiler-message","message":{"level":"error","message":"expected type","spans":[{"file_name":"src/lib.rs","line_start":3,"column_start":4,"is_primary":true}]}}).to_string();
        r.write(
            "diagnostics.txt",
            &format!(
                "{message}\n{message}\n{}\ntest broken ... FAILED\n",
                "x".repeat(2_000_000)
            ),
        );
        let out = r.call(&["run", "--", "sh", "-c", "cat diagnostics.txt; exit 7"]);
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["exit_code"], 7);
        assert!(out.stdout.len() < 12000);
        assert_eq!(
            v["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|d| d.as_str().unwrap().contains("expected type"))
                .count(),
            1
        );
        assert!(
            v["diagnostics"][0]
                .as_str()
                .unwrap()
                .contains("src/lib.rs:3:4")
        );
        assert!(
            fs::metadata(r.path().join(v["stdout"].as_str().unwrap()))
                .unwrap()
                .len()
                > 2_000_000
        );
    }
}

#[test]
fn published_schema_and_example_match_the_parser() {
    let schema = Command::new(BIN).arg("schema").output().unwrap();
    assert!(schema.status.success());
    let actual: Value = serde_json::from_slice(&schema.stdout).unwrap();
    let published: Value =
        serde_json::from_str(include_str!("../schemas/workspace-v1.schema.json")).unwrap();
    assert_eq!(
        actual, published,
        "regenerate schema with cargo run -- schema"
    );
    let r = Repo::new(false);
    r.write(
        ".ai/state.json",
        include_str!("../examples/minimal-state.json"),
    );
    assert!(r.ok(&["load"]).contains("Add the library function"));
}
#[test]
fn non_git_index_keeps_legitimate_runtime_directories_and_ignores_build_files() {
    let r = Repo::new(false);
    r.write("src/runtime/engine.rs", "fn scheduler_marker() {}\n");
    r.write(".gitignore", "scratch/\n");
    r.write("scratch/secret.txt", "private_marker\n");
    assert_eq!(
        r.json(&["probe", "scheduler_marker"])["hits"][0]["path"],
        "src/runtime/engine.rs"
    );
    assert!(
        r.json(&["probe", "private_marker"])["hits"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
#[test]
fn edited_plan_and_policy_invalidate_verification() {
    let r = Repo::new(false);
    r.plan();
    r.task("a", &[]);
    r.ok(&["task", "claim", "a", "--worker", "test"]);
    r.ok(&["verify", "a", "--allow-exec"]);
    let mut s = r.state();
    s["plans"]["p"]["objective"] = json!("Changed objective");
    r.save(&s);
    assert_eq!(r.json(&["status"])["verification"], "stale");
    r.ok(&["verify", "a", "--allow-exec"]);
    r.write(".ai/policy.md", "Different policy\n");
    assert_eq!(r.json(&["status"])["verification"], "stale");
}
#[test]
fn malformed_receipts_and_unsupported_nested_versions_are_rejected() {
    let r = Repo::new(false);
    r.plan();
    r.task("a", &[]);
    r.ok(&["task", "claim", "a", "--worker", "test"]);
    r.ok(&["verify", "a", "--allow-exec"]);
    let original = r.state();
    for (field, value) in [
        ("schema_version", json!(2)),
        ("stdout", json!("../../secret")),
        ("success", json!(false)),
    ] {
        let mut s = original.clone();
        s["tasks"]["a"]["evidence"]["runs"][0][field] = value;
        r.save(&s);
        assert!(!r.call(&["load"]).status.success());
    }
}
#[test]
fn empty_verification_is_not_a_vacuous_pass() {
    let r = Repo::new(false);
    r.plan();
    r.task("a", &[]);
    let mut s = r.state();
    s["tasks"]["a"]["verify"] = json!([]);
    r.save(&s);
    r.ok(&["task", "claim", "a", "--worker", "test"]);
    r.err(&["verify", "a", "--allow-exec"], "no verification commands");
}
#[test]
fn task_edit_validates_full_contract_and_does_not_bypass_transitions() {
    let r = Repo::new(false);
    r.plan();
    r.task("a", &[]);
    let mut task = r.state()["tasks"]["a"].clone();
    task["title"] = json!("Edited objective");
    r.write("task.json", &task.to_string());
    r.ok(&["task", "edit", "a", "--file", "task.json"]);
    assert_eq!(r.state()["tasks"]["a"]["title"], "Edited objective");
    task["status"] = json!("in_progress");
    r.write("task.json", &task.to_string());
    r.err(
        &["task", "edit", "a", "--file", "task.json"],
        "cannot change status",
    );
}

#[cfg(unix)]
#[test]
fn tracked_directory_replaced_by_symlink_is_not_followed() {
    let r = Repo::new(true);
    r.write("source/file.rs", "original");
    r.git(&["add", "source"]);
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("file.rs"), "external secret").unwrap();
    fs::remove_dir_all(r.path().join("source")).unwrap();
    std::os::unix::fs::symlink(outside.path(), r.path().join("source")).unwrap();
    r.err(&["index"], "symlink ancestor");
}
#[test]
fn aggregate_verification_and_long_command_display_stay_bounded() {
    let r = Repo::new(false);
    r.plan();
    r.task("a", &[]);
    let mut s = r.state();
    let check = s["tasks"]["a"]["verify"][0].clone();
    s["tasks"]["a"]["verify"] = json!(vec![check; 32]);
    r.save(&s);
    r.ok(&["task", "claim", "a", "--worker", "test"]);
    let output = r.ok(&["verify", "a", "--allow-exec"]);
    assert!(output.len() < 18000);
    let v: Value = serde_json::from_str(&output).unwrap();
    assert_eq!(v["omitted_runs"], 24);
    assert_eq!(v["run_count"], 32);
    let out = r.call(&["run", "--", BIN, &"a".repeat(16000)]);
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert!(out.stdout.len() < 18000);
    assert_eq!(v["argv_truncated"], true);
}
