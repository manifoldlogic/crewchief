//! Live file-event handler for the watch loop (R06-R08 / fix spec R-WATCH-2).
//!
//! Before this module existed, every poller event in `maproom watch` was
//! routed to [`super::tree_sha_update::incremental_update`], whose HEAD^{tree}
//! gate is a property of the last *commit* — working-tree edits never change
//! it, so every live event was skipped ("Tree SHA unchanged") and the watch
//! pipeline was dead (E2E regression R06). This handler performs a scoped
//! upsert of exactly the changed file via the battle-tested
//! [`crate::indexer::upsert_files`] path (the same function `maproom upsert`
//! uses), with caller-side pre-validation (R-WATCH-8) so stats are truthful.
//!
//! It deliberately never reads or writes `index_state` — the tree-SHA gate
//! has no business on the live-event path (that gate belongs to the
//! branch-switch/commit-diff path only).

use std::path::Path;

use anyhow::{Context, Result};
use tracing::debug;

use crate::db::index_state::UpdateStats;
use crate::db::Store;
use crate::git::get_head_commit;
use crate::incremental::events::{EventType, IndexingEvent};

/// Handle one live file event from the watch poller with a scoped, verified
/// upsert/removal. Returns truthful [`UpdateStats`] (OD-4 accounting:
/// `files_processed` counts validated submissions and performed removals;
/// `chunks_processed` is not tracked on this path and stays 0).
pub async fn handle_file_event(
    store: &(dyn Store + Send + Sync),
    worktree_id: i64,
    repo: &str,
    worktree: &str,
    watch_root: &Path,
    event: &IndexingEvent,
) -> Result<UpdateStats> {
    // The poller may deliver absolute or repo-relative paths; the store's
    // delete calls need the repo-relative string matching files.relpath,
    // while upsert_files accepts either shape.
    let relpath = event
        .path
        .strip_prefix(watch_root)
        .unwrap_or(&event.path)
        .to_path_buf();
    let relpath_str = relpath.to_string_lossy().to_string();

    let mut stats = UpdateStats::new();

    match event.event_type {
        EventType::Modified => {
            let abs = if relpath.is_absolute() {
                relpath.clone()
            } else {
                watch_root.join(&relpath)
            };
            // R-WATCH-8 pre-validation. A vanished file is a benign race (the
            // poller delivers a Deleted event next); an existing-yet-unreadable
            // file is a hard error — upsert_files would silently skip it and
            // the stats would lie (the R07 phantom).
            if !abs.is_file() {
                debug!(path = %abs.display(), "Modified file no longer exists; awaiting Deleted event");
                return Ok(stats);
            }
            // Review [33]: validate indexability (UTF-8 readable) — a bare
            // open() check let non-UTF8 files report files_processed=1 while
            // upsert_files silently skipped them (the phantom class again).
            match std::fs::read_to_string(&abs) {
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::InvalidData => {
                    debug!(path = %abs.display(), "file is not valid UTF-8; not indexable (scan parity)");
                    return Ok(stats);
                }
                Err(e) => {
                    return Err(anyhow::anyhow!(e)
                        .context(format!("file exists but is unreadable: {}", abs.display())));
                }
            }

            let commit = get_head_commit(watch_root)?;
            crate::indexer::upsert_files(
                store,
                repo,
                worktree,
                watch_root,
                &commit,
                std::slice::from_ref(&relpath),
            )
            .await
            .with_context(|| format!("failed to upsert {}", relpath_str))?;
            stats.files_processed = 1;
        }
        EventType::Renamed => {
            if let Some(old) = &event.old_path {
                let old_rel = old
                    .strip_prefix(watch_root)
                    .unwrap_or(old)
                    .to_string_lossy()
                    .to_string();
                // R-GC-6: full unmap (junction + orphan GC + FTS + files rows).
                store
                    .unmap_superseded_file_chunks(worktree_id, &old_rel, None)
                    .await?;
                stats.files_processed += 1;
            }
            let abs = if relpath.is_absolute() {
                relpath.clone()
            } else {
                watch_root.join(&relpath)
            };
            if abs.is_file() {
                std::fs::File::open(&abs)
                    .with_context(|| format!("file exists but is unreadable: {}", abs.display()))?;
                let commit = get_head_commit(watch_root)?;
                crate::indexer::upsert_files(
                    store,
                    repo,
                    worktree,
                    watch_root,
                    &commit,
                    std::slice::from_ref(&relpath),
                )
                .await
                .with_context(|| format!("failed to upsert renamed {}", relpath_str))?;
                stats.files_processed += 1;
            } else {
                debug!(path = %abs.display(), "Renamed target no longer exists; awaiting Deleted event");
            }
        }
        EventType::Deleted => {
            // Review H2: the git-status poller emits pseudo-Deleted events for
            // any path that merely LEAVES `git status` output — commit, stash,
            // restore, `checkout --`. Unmapping on those destroyed
            // just-indexed data (e.g. `git commit -am` under watch removed the
            // committed file from the index until a manual rescan). Gate the
            // destructive unmap on the file actually being GONE from disk,
            // mirroring the Modified arm's pre-validation.
            let abs = if relpath.is_absolute() {
                relpath.clone()
            } else {
                watch_root.join(&relpath)
            };
            if abs.exists() {
                debug!(
                    path = %abs.display(),
                    "Deleted event but file exists on disk (left git-status via commit/stash/restore); re-upserting instead of unmapping"
                );
                if abs.is_file() {
                    std::fs::File::open(&abs).with_context(|| {
                        format!("file exists but is unreadable: {}", abs.display())
                    })?;
                    let commit = get_head_commit(watch_root)?;
                    crate::indexer::upsert_files(
                        store,
                        repo,
                        worktree,
                        watch_root,
                        &commit,
                        std::slice::from_ref(&relpath),
                    )
                    .await
                    .with_context(|| format!("failed to re-upsert {}", relpath_str))?;
                    stats.files_processed = 1;
                }
                return Ok(stats);
            }

            // R-GC-6: full unmap (junction + orphan GC + FTS + files rows) —
            // remove_worktree_from_chunks lacked files-row GC and FTS cleanup.
            let affected = store
                .unmap_superseded_file_chunks(worktree_id, &relpath_str, None)
                .await?;
            debug!(
                file = %relpath_str,
                junction_rows = affected,
                "Unmapped worktree from chunks for deleted file"
            );
            stats.files_processed = 1;
        }
    }

    Ok(stats)
}
