//! Resolution engine: library loading, batched relation resolution, cache.

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

use crate::domain::cluster::RelationGraph;
use crate::domain::model::{Anime, Library};
use crate::domain::relation::RelationKind;
use crate::error::{AnirelError, Result};
use crate::infra::anilist::{AniListClient, BATCH_SIZE, Media, ResolvedMedia};
use crate::infra::cache::Cache;
use crate::infra::mal_web::MalWebClient;

/// Ties the backends and the cache together.
#[derive(Clone)]
pub struct Engine {
    pub anilist: AniListClient,
    pub cache: Cache,
    pub media_ttl: Duration,
    pub library_ttl: Duration,
}

impl Engine {
    pub fn new(cache_dir: PathBuf, media_ttl: Duration, library_ttl: Duration) -> Self {
        Self {
            anilist: AniListClient::new(),
            cache: Cache::new(cache_dir),
            media_ttl,
            library_ttl,
        }
    }

    /// Load the user's library, preferring a warm cache.
    pub async fn library(&self, user: &str, force_refresh: bool) -> Result<Library> {
        self.library_with_progress(user, force_refresh, |_| {}).await
    }

    /// Load the user's library, preferring a warm cache, and report progress if fetching.
    pub async fn library_with_progress<F>(
        &self,
        user: &str,
        force_refresh: bool,
        on_progress: F,
    ) -> Result<Library>
    where
        F: FnMut(usize),
    {
        if !force_refresh && let Some(library) = self.cache.read_library(user, self.library_ttl) {
            return Ok(library);
        }
        let client = MalWebClient::new();
        let library = client.fetch_library_with_progress(user, on_progress).await?;
        let _ = self.cache.write_library(user, &library);
        Ok(library)
    }

    /// Resolve a batch of MAL ids, using the cache where possible.
    ///
    /// Returns one slot per requested id, in the same order.
    pub async fn resolve_batch(&self, ids: &[i64]) -> Result<Vec<Option<ResolvedMedia>>> {
        let mut out: Vec<Option<ResolvedMedia>> = vec![None; ids.len()];
        let mut pending: Vec<usize> = Vec::new();

        for (index, id) in ids.iter().enumerate() {
            if let Some(media) = self.cache.read_media(*id, self.media_ttl)
                && let Some(item) = materialise(&media)
            {
                out[index] = Some(item);
                continue;
            }
            pending.push(index);
        }

        for chunk in pending.chunks(BATCH_SIZE) {
            let batch: Vec<i64> = chunk.iter().map(|index| ids[*index]).collect();
            let media = self.anilist.resolve(&batch).await?;
            for (slot, raw) in chunk.iter().zip(media) {
                let Some(raw) = raw else {
                    continue;
                };
                if let Some(id) = raw.id_mal {
                    let _ = self.cache.write_media(id, &raw);
                }
                if let Some(item) = materialise(&raw) {
                    out[*slot] = Some(item);
                }
            }
            tokio::time::sleep(crate::infra::anilist::REQUEST_PAUSE).await;
        }
        Ok(out)
    }

    /// Resolve MAL ids into a relation graph (cache first, network for the rest).
    pub async fn resolve_into(
        &self,
        ids: &[i64],
        graph: &mut RelationGraph,
    ) -> Result<Vec<ResolvedMedia>> {
        let mut resolved: Vec<ResolvedMedia> = Vec::new();
        let mut pending: Vec<i64> = Vec::new();

        for id in ids {
            match self.cache.read_media(*id, self.media_ttl) {
                Some(media) => match materialise(&media) {
                    Some(item) => resolved.push(item),
                    None => pending.push(*id),
                },
                None => pending.push(*id),
            }
        }

        for chunk in pending.chunks(BATCH_SIZE) {
            let ids: Vec<i64> = chunk.to_vec();
            let media = self.anilist.resolve(&ids).await?;
            for raw in media.into_iter().flatten() {
                let id = raw.id_mal.unwrap_or_default();
                if id != 0 {
                    let _ = self.cache.write_media(id, &raw);
                }
                if let Some(item) = materialise(&raw) {
                    resolved.push(item);
                }
            }
            tokio::time::sleep(crate::infra::anilist::REQUEST_PAUSE).await;
        }

        for item in &resolved {
            graph.insert_media(item.anime.clone());
            for (target, kind) in &item.relations {
                graph.add_edge(item.anime.mal_id, *target, *kind);
            }
        }
        Ok(resolved)
    }

    /// Follow franchise relations outwards from a set of seeds.
    ///
    /// `hops` counts the layers *beyond* the seeds: `hops = 0` resolves the
    /// seeds only, `hops = 2` also resolves their neighbours and their
    /// neighbours' neighbours.
    pub async fn expand(
        &self,
        graph: &mut RelationGraph,
        seeds: &[i64],
        hops: usize,
    ) -> Result<()> {
        let mut discovered_ids: HashSet<i64> = HashSet::new();
        let mut frontier: Vec<i64> = seeds.to_vec();

        for _ in 0..=hops {
            if frontier.is_empty() {
                break;
            }
            let resolved = self.resolve_into(&frontier, graph).await?;
            let mut next: Vec<i64> = Vec::new();
            for item in resolved {
                for (target, kind) in item.relations {
                    if kind.is_franchise() && !discovered_ids.contains(&target) {
                        discovered_ids.insert(target);
                        next.push(target);
                    }
                }
            }
            frontier = next;
        }
        Ok(())
    }
}

/// Turn cached media into a resolved entry.
pub fn materialise(media: &Media) -> Option<ResolvedMedia> {
    let anime = media.to_anime()?;
    let mut relations = Vec::new();
    if let Some(rels) = &media.relations {
        for edge in &rels.edges {
            let kind = RelationKind::parse(&edge.relation_type);
            if let Some(target) = edge.node.to_anime() {
                relations.push((target.mal_id, kind));
            }
        }
    }
    Some(ResolvedMedia { anime, relations })
}

/// Convenience wrapper used by the CLI and the TUI.
pub async fn build_engine(config: &crate::config::Config) -> Result<Engine> {
    Ok(Engine::new(
        config.cache_dir.clone(),
        config.media_ttl,
        config.library_ttl,
    ))
}

/// Search for a title by name.
pub async fn search_titles(query: &str) -> Result<Vec<Anime>> {
    let client = AniListClient::new();
    crate::infra::anilist::search(&client, query)
        .await
        .map_err(|e| match e {
            AnirelError::Api(detail) => AnirelError::Api(detail),
            other => other,
        })
}


