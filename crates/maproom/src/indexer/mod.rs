use std::{fs, path::{Path, PathBuf}};

use anyhow::Context;
use ignore::WalkBuilder;
use tokio_postgres::Client;
use tracing::info;

mod parser;

fn detect_language_from_path(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "ts" => Some("ts"),
        "tsx" => Some("tsx"),
        "js" => Some("js"),
        "jsx" => Some("jsx"),
        "rs" => Some("rs"),
        _ => None,
    }
}

fn build_ts_doc(relpath: &str, symbol_name: Option<&str>, signature: Option<&str>, docstring: Option<&str>, preview: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    parts.push(relpath.to_owned());
    if let Some(s) = symbol_name { parts.push(s.to_owned()); }
    if let Some(s) = signature { parts.push(s.to_owned()); }
    if let Some(s) = docstring { parts.push(s.to_owned()); }
    parts.push(preview.to_owned());
    parts.join(" \n ")
}

fn first_n_lines(s: &str, n: usize) -> String {
    s.lines().take(n).collect::<Vec<_>>().join("\n")
}

fn file_modified_time(path: &Path) -> Option<chrono::DateTime<chrono::Utc>> {
    use std::time::UNIX_EPOCH;
    let t = fs::metadata(path).and_then(|m| m.modified()).ok()?;
    let dur = t.duration_since(UNIX_EPOCH).ok()?;
    let naive = chrono::NaiveDateTime::from_timestamp_opt(dur.as_secs() as i64, dur.subsec_nanos())?;
    Some(chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(naive, chrono::Utc))
}

pub async fn scan_worktree(
    client: &Client,
    repo: &str,
    worktree: &str,
    root: &Path,
    commit: &str,
    _concurrency: usize,
    languages: Option<Vec<String>>,
    exclude: Option<Vec<String>>,
) -> anyhow::Result<()> {
    let root_abs = root.canonicalize().with_context(|| "invalid root path")?;
    let repo_id = crate::db::get_or_create_repo(client, repo, root_abs.to_string_lossy().as_ref()).await?;
    let worktree_id = crate::db::get_or_create_worktree(client, repo_id, worktree, root_abs.to_string_lossy().as_ref()).await?;
    let commit_id = crate::db::get_or_create_commit(client, repo_id, commit, None).await?;

    let mut walk = WalkBuilder::new(&root_abs);
    walk.hidden(false).ignore(true).git_ignore(true).git_exclude(true);
    // TODO: implement exclude globs via ignore::overrides::OverrideBuilder

    let allow_langs: Option<Vec<String>> = languages.map(|v| v.into_iter().map(|s| s.to_lowercase()).collect());

    for dent in walk.build() {
        let dent = match dent { Ok(d) => d, Err(_) => continue };
        if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) { continue; }
        let path = dent.path();
        let relpath = path.strip_prefix(&root_abs).unwrap_or(path);
        let language = detect_language_from_path(path);
        if language.is_none() { continue; }
        if let Some(ref allow) = allow_langs {
            if !allow.iter().any(|l| l == language.unwrap()) { continue; }
        }

        let content = match fs::read_to_string(path) { Ok(c) => c, Err(_) => continue };
        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        let size_bytes = content.len().min(i32::MAX as usize) as i32;
        let last_modified = file_modified_time(path);

        let file_id = crate::db::upsert_file(
            client,
            repo_id,
            worktree_id,
            commit_id,
            relpath.to_string_lossy().as_ref(),
            language,
            &content_hash,
            size_bytes,
            last_modified,
        )
        .await?;

        let chunks = parser::extract_chunks(&content, language.unwrap());
        if chunks.is_empty() {
            // Fallback: single module chunk
            let preview = first_n_lines(&content, 40);
            let ts_doc = build_ts_doc(relpath.to_string_lossy().as_ref(), None, None, None, &preview);
            crate::db::insert_chunk(
                client,
                file_id,
                None,
                "module",
                None,
                None,
                1,
                content.lines().count() as i32,
                &preview,
                &ts_doc,
                1.0,
                0.0,
            ).await?;
        } else {
            for ch in chunks {
                let preview = first_n_lines(&content.split('\n').skip(ch.start_line as usize - 1).take((ch.end_line - ch.start_line + 1) as usize).collect::<Vec<&str>>().join("\n"), 40);
                let ts_doc = build_ts_doc(relpath.to_string_lossy().as_ref(), ch.symbol_name.as_deref(), ch.signature.as_deref(), ch.docstring.as_deref(), &preview);
                crate::db::insert_chunk(
                    client,
                    file_id,
                    ch.symbol_name.as_deref(),
                    &ch.kind,
                    ch.signature.as_deref(),
                    ch.docstring.as_deref(),
                    ch.start_line,
                    ch.end_line,
                    &preview,
                    &ts_doc,
                    1.0,
                    0.0,
                ).await?;
            }
        }
    }

    info!(?repo, ?worktree, ?commit, "scan complete");
    Ok(())
}

pub async fn upsert_files(
    client: &Client,
    repo: &str,
    worktree: &str,
    root: &Path,
    commit: &str,
    paths: &[PathBuf],
) -> anyhow::Result<()> {
    let _ = paths; // TODO: selective
    scan_worktree(client, repo, worktree, root, commit, 1, None, None).await
}

#[derive(Debug, Clone)]
pub struct SymbolChunk {
    pub symbol_name: Option<String>,
    pub kind: String,
    pub signature: Option<String>,
    pub docstring: Option<String>,
    pub start_line: i32,
    pub end_line: i32,
}


