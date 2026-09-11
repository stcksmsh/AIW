use crate::{
    model::{State, Task},
    repo,
};
use anyhow::{Context, Result, bail, ensure};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

pub const POLICY: &str = "# AIW operating contract\n\nThe project persists; agents are replaceable.\n\n- Favor correctness and finished, verified software.\n- Continue inspect → implement → build → test → diagnose → fix → verify → checkpoint.\n- Start with `aiw load`; retrieve only the task, decision, or source needed next.\n- Prefer deterministic tools and authoritative state before inference.\n- Diagnose evidence before broadening changes; honor task scope and invariants.\n- Completion requires declared verification and a result; persist real blockers.\n- Keep decisions and next actions in AIW, not only in conversation.\n- Checkpoint so a fresh agent can continue without your history.\n- Delegate only independent bounded work; return compact results and evidence.\n- Consult local state/source/docs before targeted authoritative external docs.\n";

pub fn relative(path: &str) -> Result<()> {
    ensure!(
        !path.is_empty()
            && !path.contains('\\')
            && !path.contains(':')
            && !path.chars().any(char::is_control),
        "invalid portable relative path {path:?}"
    );
    ensure!(
        Path::new(path)
            .components()
            .all(|c| matches!(c, Component::Normal(_))),
        "path must be relative without '.' or '..': {path}"
    );
    Ok(())
}
#[derive(Clone)]
pub struct Workspace {
    pub root: PathBuf,
}
impl Workspace {
    pub fn checkpoint_freshness(&self, state: &State, id: &str) -> Result<&'static str> {
        let Some(checkpoint) = state
            .checkpoint
            .as_ref()
            .filter(|c| c.task.as_deref() == Some(id))
        else {
            return Ok("missing");
        };
        let Some(source_hash) = &checkpoint.source_hash else {
            return Ok("unknown");
        };
        Ok(
            if *source_hash == self.checkpoint_source(&state.tasks[id])? {
                "unchanged"
            } else {
                "changed"
            },
        )
    }
    pub fn checkpoint_source(&self, task: &Task) -> Result<String> {
        let mut source = repo::snapshot(&self.root)?;
        if !task.scope.is_empty() {
            source.retain(|path, _| {
                task.scope
                    .iter()
                    .any(|scope| Path::new(path).starts_with(Path::new(scope)))
            });
        }
        Ok(repo::hash(&serde_json::to_vec(&source)?))
    }
    pub fn discover(start: &Path) -> Result<Self> {
        let start = start.canonicalize().context("resolve working directory")?;
        for dir in start.ancestors() {
            if dir.join(".ai").exists() {
                let ws = Self { root: dir.into() };
                ws.path(".ai/state.json")?;
                return Ok(ws);
            }
            if dir.join(".git").exists() {
                break;
            }
        }
        bail!("no AIW workspace; run aiw init")
    }
    pub fn for_init(start: &Path) -> Result<Self> {
        let start = start.canonicalize()?;
        if let Ok(ws) = Self::discover(&start) {
            return Ok(ws);
        }
        let root = crate::repo::git_root(&start).unwrap_or(start);
        Ok(Self { root })
    }
    // Reject existing symlink components for every AIW-owned read/write path.
    pub fn path(&self, relative_path: &str) -> Result<PathBuf> {
        relative(relative_path)?;
        let mut path = self.root.clone();
        for c in Path::new(relative_path).components() {
            path.push(c);
            match fs::symlink_metadata(&path) {
                Ok(m) => ensure!(
                    !m.file_type().is_symlink(),
                    "refusing symlink in managed path {}",
                    path.display()
                ),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(e.into()),
            }
        }
        Ok(path)
    }
    pub fn lock(&self) -> Result<File> {
        let path = self.path(".ai/runtime/write.lock")?;
        fs::create_dir_all(path.parent().unwrap())?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)?;
        file.try_lock()
            .context("workspace busy: another AIW mutation is running; retry when it finishes")?;
        Ok(file)
    }
    pub fn read(&self) -> Result<State> {
        let path = self.path(".ai/state.json")?;
        let value: serde_json::Value = read_json(&path)?;
        ensure!(
            value.get("schema_version").and_then(|v| v.as_u64()) == Some(1),
            "unsupported or missing workspace schema_version; this client supports 1; no files changed"
        );
        let state: State = serde_json::from_value(value).context("malformed state.json")?;
        state.validate()?;
        Ok(state)
    }
    pub fn save(&self, state: &State) -> Result<()> {
        state.validate()?;
        ensure!(
            serde_json::to_vec_pretty(state)?.len() < 16 * 1024 * 1024,
            "canonical state exceeds 16 MiB; split the workspace before adding more history"
        );
        self.json(".ai/state.json", state)
    }
    pub fn json(&self, path: &str, value: &impl Serialize) -> Result<()> {
        let mut bytes = serde_json::to_vec_pretty(value)?;
        bytes.push(b'\n');
        self.write(path, &bytes)
    }
    pub fn write(&self, path: &str, bytes: &[u8]) -> Result<()> {
        atomic_write(&self.path(path)?, bytes)
    }
    pub fn init(&self, name: Option<String>) -> Result<State> {
        self.path(".ai/state.json")?;
        fs::create_dir_all(self.path(".ai")?)?;
        let _lock = self.lock()?;
        let state = if self.path(".ai/state.json")?.exists() {
            self.read()?
        } else {
            let name = name.unwrap_or_else(|| {
                self.root
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into()
            });
            let state = State::new(name);
            self.save(&state)?;
            state
        };
        for (path, content) in [
            (".ai/policy.md", POLICY),
            (".ai/.gitignore", "/derived/\n/runtime/\n"),
            (
                ".ai/decisions/README.md",
                "# Decisions\n\nRecord settled architectural decisions here. Source code remains authoritative for implementation facts.\n",
            ),
        ] {
            if !self.path(path)?.exists() {
                self.write(path, content.as_bytes())?;
            }
        }
        Ok(state)
    }
}
pub fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let mut bytes = Vec::new();
    File::open(path)
        .with_context(|| format!("read {}", path.display()))?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() <= 16 * 1024 * 1024,
        "JSON exceeds 16 MiB: {}",
        path.display()
    );
    serde_json::from_slice(&bytes).with_context(|| format!("invalid JSON in {}", path.display()))
}
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if fs::read(path).ok().as_deref() == Some(bytes) {
        return Ok(());
    }
    let parent = path.parent().context("file has no parent")?;
    fs::create_dir_all(parent)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .with_context(|| format!("replace {}", path.display()))?;
    #[cfg(unix)]
    File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_a_complete_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.json");
        fs::write(&path, b"{\"old\":true}\n").unwrap();

        atomic_write(&path, b"{\"schema_version\":1}\n").unwrap();

        let bytes = fs::read(&path).unwrap();
        assert_eq!(bytes, b"{\"schema_version\":1}\n");
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(value["schema_version"], 1);
    }
}
