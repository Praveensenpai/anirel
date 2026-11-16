//! Background scanner: resolves the relation graph in batches and streams progress.

use std::collections::HashSet;

use tokio::sync::mpsc::UnboundedSender;

use crate::domain::model::Library;
use crate::engine::Engine;
use crate::infra::anilist::{BATCH_SIZE, ResolvedMedia};

/// Messages produced while scanning.
#[derive(Debug)]
pub enum ScanEvent {
    /// Newly resolved titles.
    Resolved(Vec<ResolvedMedia>),
    /// Scan progress.
    Progress { done: usize, total: usize },
    /// Everything is resolved.
    Finished,
    /// The scan failed.
    Failed(String),
    /// A cover image finished downloading.
    CoverReady(i64, crate::art::CoverArt),
    /// A single-title load finished; its franchise can now be shown.
    LoadComplete(i64),
    /// The user's list was re-downloaded, or the attempt failed.
    LibraryRefreshed(Result<Library, String>),
}

/// How many relation hops beyond the library to follow.
const MAX_HOPS: usize = 3;

/// Resolve every title in the library plus its franchise relatives.
pub async fn run(engine: Engine, library: Library, tx: UnboundedSender<ScanEvent>) {
    let mut resolved_ids: HashSet<i64> = HashSet::new();
    let mut resolved = 0usize;
    let mut frontier: Vec<i64> = library.entries.iter().map(|e| e.anime.mal_id).collect();

    for hop in 0..=MAX_HOPS {
        let pending: Vec<i64> = frontier
            .drain(..)
            .filter(|id| !resolved_ids.contains(id))
            .collect();
        if pending.is_empty() {
            break;
        }
        resolved_ids.extend(pending.iter().copied());

        let mut discovered: Vec<i64> = Vec::new();
        for chunk in pending.chunks(BATCH_SIZE) {
            match engine.resolve_batch(chunk).await {
                Ok(results) => {
                    for result in results.into_iter().flatten() {
                        for (target, kind) in &result.relations {
                            if kind.is_franchise() && !resolved_ids.contains(target) {
                                discovered.push(*target);
                            }
                        }
                        resolved += 1;
                        tx.send(ScanEvent::Resolved(vec![result])).ok();
                    }
                    if hop < MAX_HOPS {
                        resolved_ids.extend(discovered.iter().copied());
                    }
                    tx.send(ScanEvent::Progress {
                        done: resolved,
                        total: resolved + discovered.len(),
                    })
                    .ok();
                    tokio::time::sleep(crate::infra::anilist::REQUEST_PAUSE).await;
                }
                Err(err) => {
                    tx.send(ScanEvent::Failed(err.to_string())).ok();
                    return;
                }
            }
        }
        frontier = discovered;
    }

    tx.send(ScanEvent::Finished).ok();
}
