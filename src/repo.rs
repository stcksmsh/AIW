use anyhow::{Context, Result, ensure};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

pub fn git_root(dir: &Path) -> Option<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(dir)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| PathBuf::from(String::from_utf8_lossy(&out.stdout).trim_end()))
}
pub fn git(dir: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .current_dir(dir)
        .output()
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim_end().into())
}
pub fn excluded(path: &str) -> bool {
    path == ".ai/state.json"
        || path.starts_with(".ai/derived/")
        || path.starts_with(".ai/runtime/")
        || path.split('/').any(|p| {
            matches!(
                p,
                ".git" | "target" | "node_modules" | ".venv" | "__pycache__"
            )
        })
}
pub fn files(root: &Path) -> Result<Vec<String>> {
    let mut paths = BTreeSet::new();
    if git_root(root).is_some() {
        let out = Command::new("git")
            .args([
                "ls-files",
                "-z",
                "--cached",
                "--others",
                "--exclude-standard",
            ])
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(root)
            .output()?;
        ensure!(out.status.success(), "git ls-files failed");
        for raw in out.stdout.split(|b| *b == 0).filter(|s| !s.is_empty()) {
            let p = std::str::from_utf8(raw)
                .context("non-UTF-8 Git path unsupported in workspace v1")?
                .replace('\\', "/");
            if !excluded(&p) {
                paths.insert(p);
            }
        }
    } else {
        let walker_root = root.to_path_buf();
        let walker = ignore::WalkBuilder::new(root)
            .hidden(false)
            .follow_links(false)
            .require_git(false)
            .filter_entry(move |e| {
                let relative = e
                    .path()
                    .strip_prefix(&walker_root)
                    .unwrap_or(e.path())
                    .to_string_lossy()
                    .replace('\\', "/");
                e.depth() == 0
                    || (!excluded(&relative)
                        && relative != ".ai/runtime"
                        && relative != ".ai/derived"
                        && relative != ".git")
            })
            .build();
        for entry in walker {
            let entry = entry?;
            if entry
                .file_type()
                .is_some_and(|t| t.is_file() || t.is_symlink())
            {
                let path = entry
                    .path()
                    .strip_prefix(root)?
                    .to_str()
                    .context("non-UTF-8 path unsupported")?
                    .replace('\\', "/");
                if !excluded(&path) {
                    paths.insert(path);
                }
            }
        }
    }
    Ok(paths.into_iter().collect())
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn file_hash(path: &Path) -> Result<String> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        return Ok(hash(
            format!("symlink:{}", fs::read_link(path)?.display()).as_bytes(),
        ));
    }
    if meta.is_dir() {
        return Ok(hash(
            git(path, &["rev-parse", "HEAD"])
                .unwrap_or_default()
                .as_bytes(),
        ));
    }
    ensure!(
        meta.is_file(),
        "refusing to hash special file {}",
        path.display()
    );
    let mut h = Sha256::new();
    let mut f = fs::File::open(path)?;
    let mut chunk = [0u8; 65536];
    loop {
        let n = f.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        h.update(&chunk[..n]);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        h.update((meta.permissions().mode() & 0o111).to_le_bytes());
    }
    Ok(format!("{:x}", h.finalize()))
}
pub fn snapshot(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for path in files(root)? {
        // A tracked directory can be replaced by a link after Git enumerates it.
        let relative = Path::new(&path);
        let mut checked = root.to_path_buf();
        for component in relative.parent().unwrap_or(Path::new("")).components() {
            checked.push(component);
            if let Ok(meta) = fs::symlink_metadata(&checked) {
                ensure!(
                    !meta.file_type().is_symlink(),
                    "refusing symlink ancestor in source path {path}"
                );
            }
        }
        match file_hash(&root.join(&path)) {
            Ok(hash) => {
                result.insert(path, hash);
            }
            Err(e)
                if e.downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) => {}
            Err(e) => return Err(e).with_context(|| format!("hash {path}")),
        }
    }
    Ok(result)
}
pub fn fingerprint(root: &Path) -> Result<String> {
    Ok(hash(&serde_json::to_vec(&snapshot(root)?)?))
}
#[derive(Serialize)]
pub struct Inspection {
    pub schema_version: u32,
    pub git: bool,
    pub languages: BTreeMap<String, usize>,
    pub manifests: Vec<String>,
    pub ci: Vec<String>,
    pub tests: Vec<String>,
    pub docs: Vec<String>,
    pub agent_configs: Vec<String>,
    pub suggested_verify: Vec<Vec<String>>,
}
pub fn inspect(root: &Path) -> Result<Inspection> {
    let mut i = Inspection {
        schema_version: 1,
        git: git_root(root).is_some(),
        languages: BTreeMap::new(),
        manifests: vec![],
        ci: vec![],
        tests: vec![],
        docs: vec![],
        agent_configs: vec![],
        suggested_verify: vec![],
    };
    for p in files(root)? {
        let name = p.rsplit('/').next().unwrap_or(&p);
        let lang = match Path::new(&p).extension().and_then(|s| s.to_str()) {
            Some("rs") => Some("Rust"),
            Some("py") => Some("Python"),
            Some("js" | "jsx" | "ts" | "tsx") => Some("JavaScript/TypeScript"),
            Some("java" | "kt") => Some("JVM"),
            Some("c" | "cpp" | "h") => Some("C/C++"),
            Some("go") => Some("Go"),
            _ => None,
        };
        if let Some(lang) = lang {
            *i.languages.entry(lang.into()).or_default() += 1;
        }
        if matches!(
            name,
            "Cargo.toml"
                | "package.json"
                | "pyproject.toml"
                | "CMakeLists.txt"
                | "Makefile"
                | "build.gradle"
                | "build.gradle.kts"
                | "go.mod"
                | "Dockerfile"
                | "pnpm-lock.yaml"
        ) {
            i.manifests.push(p.clone());
        }
        if p.starts_with(".github/workflows/") || name == ".gitlab-ci.yml" {
            i.ci.push(p.clone());
        }
        if p.starts_with("tests/") || name.starts_with("test_") || name.ends_with("_test.go") {
            i.tests.push(p.clone());
        }
        if name.ends_with(".md") {
            i.docs.push(p.clone());
        }
        if matches!(
            name,
            "AGENTS.md" | "AGENTS.override.md" | "CLAUDE.md" | "GEMINI.md" | ".clinerules"
        ) || p.starts_with(".cursor/")
            || p.starts_with(".claude/")
            || p.starts_with(".codex/")
            || p.starts_with(".roo/")
            || p.starts_with(".opencode/")
        {
            i.agent_configs.push(p);
        }
    }
    if root.join("Cargo.toml").is_file() {
        i.suggested_verify = vec![
            vec!["cargo".into(), "fmt".into(), "--check".into()],
            vec![
                "cargo".into(),
                "test".into(),
                "--locked".into(),
                "--message-format=json".into(),
            ],
            vec![
                "cargo".into(),
                "clippy".into(),
                "--locked".into(),
                "--all-targets".into(),
                "--message-format=json".into(),
                "--".into(),
                "-D".into(),
                "warnings".into(),
            ],
        ];
    }
    Ok(i)
}
