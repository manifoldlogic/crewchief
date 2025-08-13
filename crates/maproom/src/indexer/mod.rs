use std::{fs, path::{Path, PathBuf}};

use anyhow::Context;
use humantime::parse_duration;
use ignore::WalkBuilder;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
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
        "md" => Some("md"),
        "mdx" => Some("mdx"),
        "json" => Some("json"),
        "yaml" | "yml" => Some("yaml"),
        "toml" => Some("toml"),
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
    chrono::DateTime::<chrono::Utc>::from_timestamp(dur.as_secs() as i64, dur.subsec_nanos())
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

    // Stats tracking
    let mut files_processed = 0;
    let mut files_skipped = 0;
    let mut total_chunks = 0;
    let mut total_bytes = 0usize;
    let mut language_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    println!("🔍 Scanning worktree: {} @ {}", worktree, &commit[..8.min(commit.len())]);
    println!("   Repository: {}", repo);
    println!("   Path: {}", root_abs.display());

    let mut walk = WalkBuilder::new(&root_abs);
    walk.hidden(false).ignore(true).git_ignore(true).git_exclude(true);
    if let Some(globs) = &exclude {
        let mut ob = ignore::overrides::OverrideBuilder::new(&root_abs);
        for g in globs {
            // Treat excludes as negative overrides
            ob.add(&format!("!{}", g))?;
        }
        walk.overrides(ob.build()?);
    }

    let allow_langs: Option<Vec<String>> = languages.map(|v| v.into_iter().map(|s| s.to_lowercase()).collect());

    for dent in walk.build() {
        let dent = match dent { Ok(d) => d, Err(_) => continue };
        if !dent.file_type().map(|t| t.is_file()).unwrap_or(false) { continue; }
        let path = dent.path();
        let relpath = path.strip_prefix(&root_abs).unwrap_or(path);
        let language = detect_language_from_path(path);
        if language.is_none() { 
            files_skipped += 1;
            continue; 
        }
        if let Some(ref allow) = allow_langs {
            if !allow.iter().any(|l| l == language.unwrap()) { 
                files_skipped += 1;
                continue; 
            }
        }

        let content = match fs::read_to_string(path) { 
            Ok(c) => c, 
            Err(_) => {
                files_skipped += 1;
                continue;
            }
        };
        
        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        let size_bytes = content.len().min(i32::MAX as usize) as i32;
        let last_modified = file_modified_time(path);

        // Update stats
        files_processed += 1;
        total_bytes += content.len();
        *language_counts.entry(language.unwrap().to_string()).or_insert(0) += 1;

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
            total_chunks += 1;
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
            total_chunks += chunks.len();
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

    // Print summary
    println!("\n✅ Scan completed successfully!");
    println!("   Files processed: {}", files_processed);
    if files_skipped > 0 {
        println!("   Files skipped: {}", files_skipped);
    }
    println!("   Total chunks: {}", total_chunks);
    println!("   Total size: {:.2} MB", total_bytes as f64 / 1_048_576.0);
    
    if !language_counts.is_empty() {
        println!("\n   Languages indexed:");
        let mut langs: Vec<_> = language_counts.iter().collect();
        langs.sort_by(|a, b| b.1.cmp(a.1));
        for (lang, count) in langs {
            println!("     {} {}: {}", 
                match lang.as_str() {
                    "ts" | "tsx" => "📘",
                    "js" | "jsx" => "📙",
                    "rs" => "🦀",
                    "md" => "📝",
                    "json" => "📋",
                    "yaml" | "yml" => "📄",
                    "toml" => "⚙️",
                    _ => "📄"
                },
                lang, count);
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
    let root_abs = root.canonicalize().with_context(|| "invalid root path")?;
    let repo_id = crate::db::get_or_create_repo(client, repo, root_abs.to_string_lossy().as_ref()).await?;
    let worktree_id = crate::db::get_or_create_worktree(client, repo_id, worktree, root_abs.to_string_lossy().as_ref()).await?;
    let commit_id = crate::db::get_or_create_commit(client, repo_id, commit, None).await?;

    for path in paths {
        let abs = if path.is_absolute() { path.clone() } else { root_abs.join(path) };
        if !abs.exists() { continue; }
        if abs.is_dir() { continue; }
        let relpath = abs.strip_prefix(&root_abs).unwrap_or(&abs).to_path_buf();
        let language = detect_language_from_path(&abs);
        if language.is_none() { continue; }
        let content = match fs::read_to_string(&abs) { Ok(c) => c, Err(_) => continue };
        let content_hash = blake3::hash(content.as_bytes()).to_hex().to_string();
        let size_bytes = content.len().min(i32::MAX as usize) as i32;
        let last_modified = file_modified_time(&abs);
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
        ).await?;
        let chunks = parser::extract_chunks(&content, language.unwrap());
        if chunks.is_empty() {
            let preview = first_n_lines(&content, 40);
            let ts_doc = build_ts_doc(relpath.to_string_lossy().as_ref(), None, None, None, &preview);
            crate::db::insert_chunk(
                client, file_id, None, "module", None, None, 1, content.lines().count() as i32,
                &preview, &ts_doc, 1.0, 0.0
            ).await?;
        } else {
            for ch in chunks {
                let preview = first_n_lines(&content.split('\n').skip(ch.start_line as usize - 1).take((ch.end_line - ch.start_line + 1) as usize).collect::<Vec<&str>>().join("\n"), 40);
                let ts_doc = build_ts_doc(relpath.to_string_lossy().as_ref(), ch.symbol_name.as_deref(), ch.signature.as_deref(), ch.docstring.as_deref(), &preview);
                crate::db::insert_chunk(
                    client, file_id, ch.symbol_name.as_deref(), &ch.kind, ch.signature.as_deref(), ch.docstring.as_deref(),
                    ch.start_line, ch.end_line, &preview, &ts_doc, 1.0, 0.0
                ).await?;
            }
        }
    }

    info!(?repo, ?worktree, ?commit, updated_files=?paths.len(), "upsert selective complete");
    Ok(())
}

pub async fn watch_worktree(
    client: &Client,
    repo: &str,
    worktree: &str,
    root: &Path,
    throttle: &str,
) -> anyhow::Result<()> {
    let root_abs = root.canonicalize().with_context(|| "invalid root path")?;
    let throttle_dur = parse_duration(throttle)?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();

    let mut watcher: RecommendedWatcher = RecommendedWatcher::new(
        move |res| {
            let _ = tx.send(res);
        },
        Config::default(),
    )?;
    watcher.watch(&root_abs, RecursiveMode::Recursive)?;

    let mut last_run = std::time::Instant::now() - throttle_dur;
    while let Some(event) = rx.recv().await {
        let _ = event; // ignore details for now
        if last_run.elapsed() < throttle_dur { continue; }
        last_run = std::time::Instant::now();
        // For simplicity, re-scan incrementally later; for now run a light scan
        scan_worktree(client, repo, worktree, &root_abs, "WORKTREE", 1, None, None).await.ok();
    }
    Ok(())
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


