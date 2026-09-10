use anyhow::{Result, bail, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct State {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub project: Project,
    pub plans: BTreeMap<String, Plan>,
    pub tasks: BTreeMap<String, Task>,
    pub active_plan: Option<String>,
    pub active_task: Option<String>,
    pub checkpoint: Option<Checkpoint>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub name: String,
    pub description: String,
    pub invariants: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Plan {
    pub title: String,
    pub objective: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub title: String,
    pub plan: String,
    pub status: Status,
    pub dependencies: Vec<String>,
    pub scope: Vec<String>,
    pub acceptance: Vec<String>,
    pub constraints: Vec<String>,
    pub verify: Vec<CommandSpec>,
    pub worker: Option<String>,
    pub blocker: Option<String>,
    pub result: Option<String>,
    pub evidence: Option<Evidence>,
}
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, clap::ValueEnum,
)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}
impl Status {
    pub fn terminal(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Cancelled => "cancelled",
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CommandSpec {
    pub argv: Vec<String>,
    pub timeout_seconds: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub source_hash: String,
    pub contract_hash: String,
    pub success: bool,
    pub runs: Vec<RunSummary>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RunSummary {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u32,
    pub id: String,
    pub argv: Vec<String>,
    pub exit_code: Option<i32>,
    pub success: bool,
    pub timed_out: bool,
    pub duration_ms: u64,
    pub stdout: String,
    pub stderr: String,
    pub diagnostics: Vec<String>,
    pub tail: Vec<String>,
    pub omitted: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub task: Option<String>,
    pub next_action: String,
    pub note: String,
}

pub fn valid_id(id: &str) -> Result<()> {
    ensure!(
        !id.is_empty()
            && id.len() <= 64
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
        "invalid ID {id:?}: use 1–64 ASCII letters, digits, '-' or '_'"
    );
    Ok(())
}
fn bounded(s: &str, name: &str) -> Result<()> {
    ensure!(
        !s.trim().is_empty() && s.len() <= 4096 && !s.contains('\0'),
        "{name} must contain 1–4096 bytes of nonempty text"
    );
    Ok(())
}
fn strings(items: &[String], name: &str) -> Result<()> {
    ensure!(items.len() <= 64, "{name}: maximum 64 entries");
    for item in items {
        bounded(item, name)?;
    }
    Ok(())
}
impl CommandSpec {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.argv.is_empty() && self.argv.len() <= 128,
            "command must have 1–128 argv entries"
        );
        bounded(&self.argv[0], "program")?;
        for arg in &self.argv {
            ensure!(
                arg.len() <= 16384 && !arg.contains('\0'),
                "invalid command argument"
            );
        }
        ensure!(
            (1..=86400).contains(&self.timeout_seconds),
            "timeout must be 1–86400 seconds"
        );
        Ok(())
    }
}
impl State {
    pub fn new(name: String) -> Self {
        Self {
            schema_version: VERSION,
            project: Project {
                name,
                description: "Describe the project's purpose here.".into(),
                invariants: vec![],
            },
            plans: BTreeMap::new(),
            tasks: BTreeMap::new(),
            active_plan: None,
            active_task: None,
            checkpoint: None,
        }
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.schema_version == VERSION,
            "unsupported workspace schema {}; this client supports 1; no files changed",
            self.schema_version
        );
        bounded(&self.project.name, "project name")?;
        bounded(&self.project.description, "description")?;
        strings(&self.project.invariants, "invariants")?;
        for (id, p) in &self.plans {
            valid_id(id)?;
            bounded(&p.title, "plan title")?;
            bounded(&p.objective, "objective")?;
        }
        for (id, t) in &self.tasks {
            valid_id(id)?;
            bounded(&t.title, "task title")?;
            ensure!(
                self.plans.contains_key(&t.plan),
                "task {id}: missing plan {}",
                t.plan
            );
            ensure!(
                !t.acceptance.is_empty(),
                "task {id}: acceptance criteria required"
            );
            strings(&t.acceptance, "acceptance")?;
            strings(&t.scope, "scope")?;
            strings(&t.constraints, "constraints")?;
            for path in &t.scope {
                crate::workspace::relative(path)?;
            }
            ensure!(t.verify.len() <= 32, "maximum 32 verification commands");
            for c in &t.verify {
                c.validate()?;
            }
            for s in [&t.worker, &t.blocker, &t.result].into_iter().flatten() {
                bounded(s, "task metadata")?;
            }
            ensure!(
                t.status != Status::Blocked || t.blocker.is_some(),
                "blocked task {id} needs a blocker"
            );
            ensure!(
                t.status != Status::Done
                    || (t.result.is_some()
                        && t.evidence.as_ref().is_some_and(|e| e.success
                            && !e.runs.is_empty()
                            && e.runs.iter().all(|r| r.success))),
                "done task {id} requires result and successful evidence"
            );
            if let Some(e) = &t.evidence {
                e.validate()?;
            }
            ensure!(t.dependencies.len() <= 64, "maximum 64 dependencies");
            let mut deps = BTreeSet::new();
            for dep in &t.dependencies {
                ensure!(
                    self.tasks.contains_key(dep),
                    "task {id}: missing dependency {dep}"
                );
                ensure!(deps.insert(dep), "task {id}: duplicate dependency {dep}");
                ensure!(dep != id, "task {id}: self dependency");
                if matches!(t.status, Status::InProgress | Status::Done) {
                    ensure!(
                        self.tasks[dep].status == Status::Done,
                        "task {id}: unfinished dependency {dep}"
                    );
                }
            }
        }
        // Kahn's algorithm: no recursion depth proportional to untrusted task data.
        let mut indegree: BTreeMap<&String, usize> = self
            .tasks
            .iter()
            .map(|(id, t)| (id, t.dependencies.len()))
            .collect();
        let mut dependents: BTreeMap<&String, Vec<&String>> = BTreeMap::new();
        for (id, t) in &self.tasks {
            for dep in &t.dependencies {
                dependents.entry(dep).or_default().push(id);
            }
        }
        let mut ready: Vec<_> = indegree
            .iter()
            .filter(|(_, n)| **n == 0)
            .map(|(id, _)| *id)
            .collect();
        let mut visited = 0;
        while let Some(id) = ready.pop() {
            visited += 1;
            if let Some(children) = dependents.get(id) {
                for child in children {
                    let n = indegree.get_mut(child).unwrap();
                    *n -= 1;
                    if *n == 0 {
                        ready.push(child);
                    }
                }
            }
        }
        ensure!(
            visited == self.tasks.len(),
            "task dependency cycle detected"
        );
        if let Some(id) = &self.active_plan {
            ensure!(self.plans.contains_key(id), "missing active plan {id}");
        }
        if let Some(id) = &self.active_task {
            let t = self
                .tasks
                .get(id)
                .ok_or_else(|| anyhow::anyhow!("missing active task {id}"))?;
            ensure!(!t.status.terminal(), "active task is terminal");
            ensure!(
                self.active_plan.as_ref() == Some(&t.plan),
                "active task and plan disagree"
            );
        }
        if let Some(c) = &self.checkpoint {
            bounded(&c.next_action, "next action")?;
            ensure!(c.note.len() <= 4096, "checkpoint note exceeds 4096 bytes");
            if let Some(id) = &c.task {
                ensure!(
                    self.tasks.contains_key(id),
                    "checkpoint references missing task"
                );
            }
        }
        Ok(())
    }
    pub fn ready(&self, id: &str) -> bool {
        self.tasks[id]
            .dependencies
            .iter()
            .all(|d| self.tasks[d].status == Status::Done)
    }
    pub fn transition(
        &mut self,
        id: &str,
        to: Status,
        reason: Option<String>,
        result: Option<String>,
    ) -> Result<()> {
        let t = self
            .tasks
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("unknown task {id}"))?;
        let from = t.status;
        if from == to {
            return Ok(());
        }
        let allowed = matches!(
            (from, to),
            (
                Status::Pending,
                Status::InProgress | Status::Blocked | Status::Cancelled
            ) | (
                Status::InProgress,
                Status::Blocked | Status::Done | Status::Cancelled
            ) | (
                Status::Blocked,
                Status::Pending | Status::InProgress | Status::Cancelled
            ) | (Status::Done | Status::Cancelled, Status::Pending)
        );
        ensure!(
            allowed,
            "invalid transition {} -> {}",
            from.label(),
            to.label()
        );
        if to == Status::InProgress || to == Status::Done {
            ensure!(self.ready(id), "dependencies of {id} are not done");
        }
        if from == Status::Done {
            ensure!(
                !self
                    .tasks
                    .values()
                    .any(|other| other.dependencies.contains(&id.to_string())
                        && matches!(other.status, Status::InProgress | Status::Done)),
                "cannot reopen {id}: active/completed dependent exists"
            );
        }
        if to == Status::Blocked && reason.as_ref().is_none_or(|r| r.trim().is_empty()) {
            bail!("blocked transition needs --reason");
        }
        if to == Status::Done {
            ensure!(
                result.as_ref().is_some_and(|r| !r.trim().is_empty()),
                "done requires --result"
            );
        }
        let t = self.tasks.get_mut(id).unwrap();
        t.status = to;
        t.blocker = if to == Status::Blocked { reason } else { None };
        if to == Status::Done {
            t.result = result;
        }
        if to == Status::Pending {
            t.evidence = None;
            t.result = None;
            t.worker = None;
        }
        if to.terminal() && self.active_task.as_deref() == Some(id) {
            self.active_task = None;
        }
        self.validate()
    }
}

impl Evidence {
    fn validate(&self) -> Result<()> {
        for hash in [&self.source_hash, &self.contract_hash] {
            ensure!(
                hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
                "invalid evidence fingerprint"
            );
        }
        ensure!(
            !self.runs.is_empty() && self.runs.len() <= 32,
            "evidence must have 1–32 run receipts"
        );
        ensure!(
            !self.success || self.runs.iter().all(|r| r.success),
            "inconsistent evidence success"
        );
        for r in &self.runs {
            ensure!(
                r.schema_version == VERSION,
                "unsupported run receipt version"
            );
            valid_id(&r.id)?;
            CommandSpec {
                argv: r.argv.clone(),
                timeout_seconds: 1,
            }
            .validate()?;
            ensure!(
                r.success == (r.exit_code == Some(0) && !r.timed_out),
                "inconsistent run success"
            );
            ensure!(
                r.stdout == format!(".ai/runtime/runs/{}/stdout.log", r.id)
                    && r.stderr == format!(".ai/runtime/runs/{}/stderr.log", r.id),
                "invalid run artifact paths"
            );
            ensure!(
                r.diagnostics.len() <= 26
                    && r.tail.len() <= 8
                    && r.diagnostics.iter().chain(&r.tail).all(|s| s.len() <= 400),
                "run summary exceeds bounds"
            );
        }
        Ok(())
    }
}
