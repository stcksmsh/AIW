mod adapter;
mod hook;
mod index;
mod model;
mod repo;
mod runner;
mod view;
mod workspace;

use anyhow::{Context, Result, ensure};
use clap::{Parser, Subcommand};
use model::{CommandSpec, Plan, Status, Task};
use serde_json::json;
use std::{path::PathBuf, time::Instant};
use workspace::Workspace;

#[derive(Parser)]
#[command(
    version,
    about = "Persistent project state and deterministic tools for disposable agents"
)]
struct Cli {
    #[arg(short = 'C', long, global = true, default_value = ".")]
    directory: PathBuf,
    /// Append local operation timing/provider metadata to disposable JSONL.
    #[arg(long, global = true)]
    events: bool,
    #[command(subcommand)]
    command: Op,
}
#[derive(Subcommand)]
enum Op {
    /// Scaffold a versioned workspace without replacing existing decisions.
    Init {
        #[arg(long)]
        name: Option<String>,
    },
    /// Recover current work within a hard UTF-8 byte budget.
    #[command(alias = "handoff")]
    Load {
        #[arg(long)]
        task: Option<String>,
        #[arg(long, default_value_t = 8192)]
        budget: usize,
    },
    Status,
    List {
        #[arg(value_parser = ["tasks", "plans"])]
        entity: String,
        #[arg(long, value_enum)]
        status: Option<Status>,
        #[arg(long)]
        ready: bool,
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long, default_value_t = 0)]
        offset: usize,
    },
    Show {
        #[arg(value_parser = ["task", "plan", "project", "checkpoint", "inspection", "run"])]
        entity: String,
        id: Option<String>,
    },
    Plan {
        #[command(subcommand)]
        action: PlanOp,
    },
    Task {
        #[command(subcommand)]
        action: TaskOp,
    },
    /// Execute explicit argv; full output is saved, a compact receipt is printed.
    Run {
        #[arg(long, default_value_t = 600)]
        timeout: u64,
        #[arg(required = true, last = true)]
        argv: Vec<String>,
    },
    /// Execute a task's repository-defined acceptance commands.
    Verify {
        id: String,
        #[arg(long)]
        allow_exec: bool,
    },
    Checkpoint {
        #[arg(long)]
        next: String,
        #[arg(long, default_value = "")]
        note: String,
        #[arg(long)]
        task: Option<String>,
    },
    Index,
    Probe {
        query: String,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    Adapter {
        #[arg(value_parser = ["all", "codex", "claude"], default_value = "all")]
        vendor: String,
    },
    /// Install project skills, bootstrap files, and optional lifecycle enforcement.
    Integrate {
        #[arg(value_parser = ["all", "codex", "claude"], default_value = "all")]
        vendor: String,
        #[arg(long, value_enum, default_value_t = Enforcement::Observe)]
        enforcement: Enforcement,
    },
    /// Agent lifecycle entrypoints. Intended for generated hook configuration.
    #[command(hide = true)]
    Hook {
        #[command(subcommand)]
        action: HookOp,
    },
    Doctor,
    /// Export the language-independent workspace JSON Schema.
    Schema,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum Enforcement {
    /// Inject recovery context at session and subagent start.
    Observe,
    /// Also continue a turn once while its active task is unfinished.
    Strict,
}

#[derive(Subcommand)]
enum HookOp {
    SessionStart,
    Stop {
        #[arg(long)]
        strict: bool,
    },
}
#[derive(Subcommand)]
enum PlanOp {
    Add {
        id: String,
        title: String,
        #[arg(long)]
        objective: String,
    },
    Activate {
        id: String,
    },
}
#[derive(Subcommand)]
enum TaskOp {
    Add {
        id: String,
        title: String,
        #[arg(long)]
        plan: String,
        #[arg(long = "accept", required = true)]
        acceptance: Vec<String>,
        #[arg(long)]
        scope: Vec<String>,
        #[arg(long = "constraint")]
        constraints: Vec<String>,
        #[arg(long = "depends", value_delimiter = ',')]
        dependencies: Vec<String>,
        /// JSON argv array; repeat for each command. No implicit shell.
        #[arg(long = "verify")]
        verify: Vec<String>,
        #[arg(long, default_value_t = 600)]
        timeout: u64,
    },
    /// Replace a pending/blocked task contract with a validated JSON Task object.
    Edit {
        id: String,
        #[arg(long)]
        file: PathBuf,
    },
    Focus {
        id: String,
    },
    /// Atomically claim dependency-ready work and record worker identity.
    Claim {
        id: String,
        #[arg(long)]
        worker: String,
    },
    Transition {
        id: String,
        #[arg(value_enum)]
        to: Status,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        result: Option<String>,
    },
}
fn print(value: &impl serde::Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}
fn main() {
    if let Err(error) = dispatch(Cli::parse()) {
        eprintln!("aiw: {error:#}");
        std::process::exit(1);
    }
}
fn dispatch(cli: Cli) -> Result<()> {
    let start = Instant::now();
    if let Op::Hook { action } = &cli.command {
        let outcome = match action {
            HookOp::SessionStart => hook::session_start()?,
            HookOp::Stop { strict } => hook::stop(*strict)?,
        };
        if let Some(context) = outcome.context {
            println!("{context}");
        }
        if let Some(reason) = outcome.block {
            eprintln!("{reason}");
            std::process::exit(2);
        }
        return Ok(());
    }
    if matches!(cli.command, Op::Schema) {
        return print(&schemars::schema_for!(model::State));
    }
    if let Op::Init { name } = cli.command {
        let ws = Workspace::for_init(&cli.directory)?;
        let state = ws.init(name)?;
        let _lock = ws.lock()?;
        let inspection = repo::inspect(&ws.root)?;
        ws.json(".ai/derived/inspection.json", &inspection)?;
        print(
            &json!({"schema_version":1,"project":state.project.name,"workspace":".ai/state.json","inspection":".ai/derived/inspection.json","next":"aiw load"}),
        )?;
        return Ok(());
    }
    if matches!(cli.command, Op::Doctor) {
        return doctor(&cli.directory);
    }
    let ws = Workspace::discover(&cli.directory)?;
    let mutation = !matches!(
        cli.command,
        Op::Load { .. } | Op::Status | Op::List { .. } | Op::Show { .. }
    );
    let _lock = if mutation || cli.events {
        Some(ws.lock()?)
    } else {
        None
    };
    let mut state = ws.read()?;
    let operation = match &cli.command {
        Op::Load { .. } => "load",
        Op::Status => "status",
        Op::List { .. } => "list",
        Op::Show { .. } => "show",
        Op::Plan { .. } => "plan",
        Op::Task { .. } => "task",
        Op::Run { .. } => "run",
        Op::Verify { .. } => "verify",
        Op::Checkpoint { .. } => "checkpoint",
        Op::Index => "index",
        Op::Probe { .. } => "probe",
        Op::Adapter { .. } => "adapter",
        Op::Integrate { .. } => "integrate",
        _ => "other",
    };
    let result = (|| -> Result<()> {
        match cli.command {
            Op::Load { task, budget } => {
                print!("{}", view::load(&ws, &state, task.as_deref(), budget)?);
            }
            Op::Status => print(&view::status(&ws, &state)?)?,
            Op::List {
                entity,
                status,
                ready,
                limit,
                offset,
            } => {
                ensure!((1..=100).contains(&limit), "limit must be 1–100");
                let items: Vec<_> = if entity == "tasks" {
                    state.tasks.iter().filter(|(id,t)| status.is_none_or(|s| s == t.status) && (!ready || (t.status == Status::Pending && state.ready(id)))).map(|(id,t)| json!({"id":id,"title":view::clip(&t.title,160),"plan":t.plan,"status":t.status,"ready":t.status == Status::Pending && state.ready(id)})).collect()
                } else {
                    state
                        .plans
                        .iter()
                        .map(|(id, p)| json!({"id":id,"title":view::clip(&p.title,160)}))
                        .collect()
                };
                print(
                    &json!({"schema_version":1,"total":items.len(),"offset":offset,"items":items.iter().skip(offset).take(limit).collect::<Vec<_>>(),"next_offset":if offset.saturating_add(limit) < items.len() {Some(offset+limit)} else {None}}),
                )?;
            }
            Op::Show { entity, id } => match entity.as_str() {
                "project" => print(&state.project)?,
                "checkpoint" => print(&state.checkpoint)?,
                "task" => {
                    let id = id.context("show task requires an ID")?;
                    print(state.tasks.get(&id).context("unknown task")?)?;
                }
                "plan" => {
                    let id = id.context("show plan requires an ID")?;
                    print(state.plans.get(&id).context("unknown plan")?)?;
                }
                "inspection" => print(&repo::inspect(&ws.root)?)?,
                "run" => {
                    let id = id.context("show run requires an ID")?;
                    model::valid_id(&id)?;
                    let r: model::RunSummary = workspace::read_json(
                        &ws.path(&format!(".ai/runtime/runs/{id}/result.json"))?,
                    )?;
                    print(&r)?;
                }
                _ => unreachable!(),
            },
            Op::Plan { action } => {
                match action {
                    PlanOp::Add {
                        id,
                        title,
                        objective,
                    } => {
                        model::valid_id(&id)?;
                        ensure!(!state.plans.contains_key(&id), "plan {id} already exists");
                        state.plans.insert(id.clone(), Plan { title, objective });
                        if state.active_plan.is_none() {
                            state.active_plan = Some(id);
                        }
                    }
                    PlanOp::Activate { id } => {
                        ensure!(state.plans.contains_key(&id), "unknown plan");
                        state.active_plan = Some(id);
                        state.active_task = None;
                    }
                }
                ws.save(&state)?;
                print(&json!({"saved":true}))?;
            }
            Op::Task { action } => {
                match action {
                    TaskOp::Add {
                        id,
                        title,
                        plan,
                        acceptance,
                        scope,
                        constraints,
                        dependencies,
                        verify,
                        timeout,
                    } => {
                        model::valid_id(&id)?;
                        ensure!(!state.tasks.contains_key(&id), "task {id} already exists");
                        let commands = verify
                            .iter()
                            .map(|s| {
                                Ok(CommandSpec {
                                    argv: serde_json::from_str(s)
                                        .context("--verify must be a JSON argv array")?,
                                    timeout_seconds: timeout,
                                })
                            })
                            .collect::<Result<Vec<_>>>()?;
                        state.tasks.insert(
                            id,
                            Task {
                                title,
                                plan,
                                status: Status::Pending,
                                dependencies,
                                scope,
                                acceptance,
                                constraints,
                                verify: commands,
                                worker: None,
                                blocker: None,
                                result: None,
                                evidence: None,
                            },
                        );
                    }
                    TaskOp::Edit { id, file } => {
                        let old = state.tasks.get(&id).context("unknown task")?;
                        ensure!(
                            matches!(old.status, Status::Pending | Status::Blocked),
                            "only pending/blocked task contracts can be edited"
                        );
                        let mut new: Task = workspace::read_json(&file)?;
                        ensure!(
                            new.status == old.status && new.worker == old.worker,
                            "edit cannot change status or worker; use task transition/claim"
                        );
                        new.evidence = None;
                        new.result = None;
                        state.tasks.insert(id, new);
                    }
                    TaskOp::Focus { id } => {
                        let t = state.tasks.get(&id).context("unknown task")?;
                        ensure!(!t.status.terminal(), "cannot focus terminal task");
                        state.active_plan = Some(t.plan.clone());
                        state.active_task = Some(id);
                    }
                    TaskOp::Claim { id, worker } => {
                        let t = state.tasks.get(&id).context("unknown task")?;
                        ensure!(
                            t.worker.as_ref().is_none_or(|w| w == &worker),
                            "task is claimed by {}; persist a blocker and return it to pending before reassignment",
                            t.worker.as_deref().unwrap_or("")
                        );
                        ensure!(
                            matches!(t.status, Status::Pending | Status::InProgress),
                            "claim requires pending/in-progress task"
                        );
                        state.transition(&id, Status::InProgress, None, None)?;
                        let t = state.tasks.get_mut(&id).unwrap();
                        t.worker = Some(worker);
                        state.active_plan = Some(t.plan.clone());
                        state.active_task = Some(id);
                    }
                    TaskOp::Transition {
                        id,
                        to,
                        reason,
                        result,
                    } => {
                        let task = state.tasks.get(&id).context("unknown task")?;
                        if to == Status::Done && task.status != Status::Done {
                            ensure!(
                                runner::health(&state, task, &repo::fingerprint(&ws.root)?)?
                                    == "passed",
                                "completion requires fresh successful verification; run aiw verify {id} --allow-exec"
                            );
                        }
                        state.transition(&id, to, reason, result)?;
                    }
                }
                ws.save(&state)?;
                print(&json!({"saved":true}))?;
            }
            Op::Run { timeout, argv } => {
                let result = runner::run(
                    &ws,
                    &CommandSpec {
                        argv,
                        timeout_seconds: timeout,
                    },
                )?;
                print(&view::run_packet(&result))?;
                ensure!(
                    result.success,
                    "command failed; raw logs: {}, {}",
                    result.stdout,
                    result.stderr
                );
            }
            Op::Verify { id, allow_exec } => {
                ensure!(
                    allow_exec,
                    "repository-defined commands require --allow-exec; inspect aiw show task {id} first"
                );
                let evidence = runner::verify(&ws, &mut state, &id)?;
                print(&view::verification_packet(&evidence, &id))?;
                ensure!(
                    evidence.success,
                    "verification failed or source changed; inspect compact diagnostics and raw artifacts"
                );
            }
            Op::Checkpoint { next, note, task } => {
                let task = task.or_else(|| state.active_task.clone());
                let source_hash = task
                    .as_ref()
                    .map(|id| {
                        let selected = state
                            .tasks
                            .get(id)
                            .context("checkpoint references missing task")?;
                        ws.checkpoint_source(selected)
                    })
                    .transpose()?;
                state.checkpoint = Some(model::Checkpoint {
                    task,
                    next_action: next,
                    note,
                    source_hash,
                });
                ws.save(&state)?;
                print(&json!({"saved":true,"recovery":"aiw load"}))?;
            }
            Op::Index => {
                let (_, counts) = index::update(&ws)?;
                print(&counts)?;
            }
            Op::Probe { query, limit } => print(&index::probe(&ws, &query, limit)?)?,
            Op::Adapter { vendor } => print(&json!({"generated":adapter::generate(&ws,&vendor)?}))?,
            Op::Integrate {
                vendor,
                enforcement,
            } => print(&json!({
                "generated": adapter::integrate(&ws, &vendor, matches!(enforcement, Enforcement::Strict))?,
                "enforcement": format!("{enforcement:?}").to_lowercase(),
                "next": "restart the agent; Codex users must review/trust project hooks with /hooks"
            }))?,
            Op::Init { .. } | Op::Doctor | Op::Schema | Op::Hook { .. } => unreachable!(),
        }
        Ok(())
    })();
    if cli.events
        && let Err(error) =
            runner::event(&ws, operation, result.is_ok(), start.elapsed().as_millis())
    {
        eprintln!("event recording failed: {error}");
    }
    result
}
fn doctor(directory: &std::path::Path) -> Result<()> {
    let mut capabilities = serde_json::Map::new();
    for tool in [
        "git",
        "rg",
        "cargo",
        "rustc",
        "rust-analyzer",
        "claude",
        "codex",
    ] {
        let available = std::process::Command::new(tool)
            .arg("--version")
            .current_dir(directory)
            .output()
            .is_ok_and(|o| o.status.success());
        capabilities.insert(tool.into(), json!(available));
    }
    let workspace = Workspace::discover(directory);
    let (valid, error) = match &workspace {
        Ok(ws) => match ws.read() {
            Ok(_) => (true, None),
            Err(e) => (false, Some(format!("{e:#}"))),
        },
        Err(e) => (false, Some(format!("{e:#}"))),
    };
    let overrides = workspace
        .as_ref()
        .ok()
        .is_some_and(|ws| ws.root.join("AGENTS.override.md").exists());
    let integrations = workspace
        .as_ref()
        .ok()
        .filter(|_| valid)
        .map(adapter::health)
        .transpose()?;
    print(
        &json!({"schema_version":1,"workspace_valid":valid,"error":error,"capabilities":capabilities,"codex_override_present":overrides,"integrations":integrations,"notes":["Integration diagnostics are read-only and do not require a provider binary, account, network connection, or another tool.","Only exact AIW lifecycle command handlers are inspected; unrelated handlers remain outside the diagnostic boundary.","Core has no provider dependency. Missing Git uses filesystem discovery.","Lexical index is built in; semantic LSP integration is deferred.","Ignored files, external dependencies and environment changes are outside verification fingerprints.","Commands execute with your privileges; AIW is not a sandbox.","Windows descendant-process timeout handling is best effort."]}),
    )?;
    if workspace.is_ok() {
        ensure!(valid, "workspace validation failed");
    }
    Ok(())
}
