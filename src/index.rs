use crate::{
    repo,
    workspace::{Workspace, read_json},
};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Read};

const MAX_TEXT: u64 = 1024 * 1024;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub hash: String,
    pub bytes: u64,
    pub lines: usize,
    pub searchable: bool,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Index {
    pub schema_version: u32,
    pub provenance: String,
    pub files: BTreeMap<String, Entry>,
}
#[derive(Serialize)]
pub struct Update {
    pub files: usize,
    pub rebuilt: usize,
    pub reused: usize,
    pub removed: usize,
}
pub fn text_file(ws: &Workspace, path: &str) -> Result<Option<String>> {
    let path = ws.path(path)?;
    let meta = fs::symlink_metadata(&path)?;
    if !meta.is_file() || meta.len() > MAX_TEXT {
        return Ok(None);
    }
    let mut bytes = vec![];
    fs::File::open(path)?
        .take(MAX_TEXT + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_TEXT as usize || bytes.contains(&0) {
        return Ok(None);
    }
    Ok(String::from_utf8(bytes).ok())
}
pub fn update(ws: &Workspace) -> Result<(Index, Update)> {
    let path = ws.path(".ai/derived/index.json")?;
    let old: Option<Index> = read_json(&path)
        .ok()
        .filter(|i: &Index| i.schema_version == 1 && i.provenance == "aiw-lexical-v1");
    let hashes = repo::snapshot(&ws.root)?;
    let mut index = Index {
        schema_version: 1,
        provenance: "aiw-lexical-v1".into(),
        files: BTreeMap::new(),
    };
    let mut counts = Update {
        files: hashes.len(),
        rebuilt: 0,
        reused: 0,
        removed: 0,
    };
    for (path, hash) in hashes {
        if let Some(entry) = old
            .as_ref()
            .and_then(|i| i.files.get(&path))
            .filter(|e| e.hash == hash)
        {
            index.files.insert(path, entry.clone());
            counts.reused += 1;
            continue;
        }
        let meta = fs::symlink_metadata(ws.root.join(&path))?;
        let text = if meta.is_file() {
            text_file(ws, &path)?
        } else {
            None
        };
        let entry = Entry {
            hash,
            bytes: meta.len(),
            lines: text.as_ref().map_or(0, |s| s.lines().count()),
            searchable: text.is_some(),
        };
        index.files.insert(path, entry);
        counts.rebuilt += 1;
    }
    if let Some(old) = old {
        counts.removed = old
            .files
            .keys()
            .filter(|p| !index.files.contains_key(*p))
            .count();
    }
    ws.json(".ai/derived/index.json", &index)?;
    Ok((index, counts))
}
#[derive(Serialize)]
pub struct Hit {
    pub path: String,
    pub line: Option<usize>,
    pub text: String,
}
#[derive(Serialize)]
pub struct Probe {
    pub schema_version: u32,
    pub query: String,
    pub hits: Vec<Hit>,
    pub more: bool,
}
pub fn probe(ws: &Workspace, query: &str, limit: usize) -> Result<Probe> {
    ensure!(
        !query.trim().is_empty() && query.len() <= 512,
        "query must contain 1–512 bytes"
    );
    ensure!((1..=100).contains(&limit), "limit must be 1–100");
    let (index, _) = update(ws)?;
    let needle = query.to_lowercase();
    let mut out = Probe {
        schema_version: 1,
        query: query.into(),
        hits: vec![],
        more: false,
    };
    for (path, entry) in index.files {
        let mut matches = vec![];
        if path.to_lowercase().contains(&needle) {
            matches.push(Hit {
                path: path.clone(),
                line: None,
                text: "path match".into(),
            });
        }
        if entry.searchable
            && let Some(text) = text_file(ws, &path)?
        {
            for (i, line) in text.lines().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    matches.push(Hit {
                        path: path.clone(),
                        line: Some(i + 1),
                        text: crate::view::clip(line, 240),
                    });
                    if matches.len() > limit {
                        break;
                    }
                }
            }
        }
        for hit in matches {
            if out.hits.len() == limit {
                out.more = true;
                return Ok(out);
            }
            out.hits.push(hit);
        }
    }
    Ok(out)
}
