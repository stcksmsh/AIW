use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json};
use std::process::Command;

#[derive(Debug, Deserialize)]
struct RawIssue {
    number: u64,
    title: String,
    body: Option<String>,
    labels: Vec<Value>,
}

#[derive(Debug)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub objective: String,
    pub acceptance: Vec<String>,
}

fn valid_repo(repo: &str) -> Result<()> {
    ensure!(
        repo.split('/').count() == 2
            && repo.split('/').all(|part| {
                !part.is_empty()
                    && part.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_')
                    })
            }),
        "GitHub repository must be OWNER/REPO"
    );
    Ok(())
}

fn api(args: &[String]) -> Result<Value> {
    let token = std::env::var("GH_TOKEN").context("GitHub adapter requires GH_TOKEN")?;
    ensure!(!token.trim().is_empty(), "GitHub adapter requires GH_TOKEN");
    let output = Command::new("gh")
        .arg("api")
        .args(args)
        .env("GH_TOKEN", token)
        .output()
        .context("start gh; install GitHub CLI to use the opt-in GitHub adapter")?;
    ensure!(
        output.status.success(),
        "gh api failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    serde_json::from_slice(&output.stdout).context("GitHub API returned invalid JSON")
}

fn label_name(label: &Value) -> Option<&str> {
    label.as_str().or_else(|| label.get("name")?.as_str())
}

fn parse_issue(raw: RawIssue) -> Result<Issue> {
    ensure!(
        raw.labels
            .iter()
            .filter_map(label_name)
            .any(|label| label == "agent"),
        "GitHub issue #{} must have label agent",
        raw.number
    );
    let body = raw.body.unwrap_or_default();
    let (objective, acceptance) = body.split_once("## Acceptance").unwrap_or((&body, ""));
    let objective = objective.trim();
    let acceptance: Vec<String> = acceptance
        .lines()
        .filter_map(|line| {
            line.trim()
                .strip_prefix("- ")
                .or_else(|| line.trim().strip_prefix("* "))
                .map(str::trim)
        })
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    ensure!(
        !objective.is_empty(),
        "GitHub issue body needs plan prose before ## Acceptance"
    );
    ensure!(
        !acceptance.is_empty(),
        "GitHub issue body needs one or more - bullets under ## Acceptance"
    );
    Ok(Issue {
        number: raw.number,
        title: raw.title,
        objective: objective.into(),
        acceptance,
    })
}

pub fn import_issue(repo: &str, number: u64) -> Result<Issue> {
    valid_repo(repo)?;
    parse_issue(serde_json::from_value(api(&[format!(
        "repos/{repo}/issues/{number}"
    )])?)?)
}

pub fn open_pr(repo: &str, title: &str, branch: &str, base: &str, body: &str) -> Result<Value> {
    valid_repo(repo)?;
    ensure!(!title.trim().is_empty(), "PR title must not be empty");
    ensure!(!branch.trim().is_empty(), "PR branch must not be empty");
    ensure!(!base.trim().is_empty(), "PR base must not be empty");
    ensure!(!body.trim().is_empty(), "PR body must not be empty");
    api(&[
        "--method".into(),
        "POST".into(),
        format!("repos/{repo}/pulls"),
        "-f".into(),
        format!("title={title}"),
        "-f".into(),
        format!("head={branch}"),
        "-f".into(),
        format!("base={base}"),
        "-f".into(),
        format!("body={body}"),
    ])
}

pub fn imported_task_id(number: u64) -> String {
    format!("github-issue-{number}")
}

pub fn imported_constraint(repo: &str, number: u64) -> String {
    format!(
        "Imported from GitHub issue {repo}#{number}; add scope and verification before completion"
    )
}

pub fn packet(value: Value, task: &str, branch: &str) -> Value {
    json!({"task":task,"branch":branch,"pull_request":value})
}
