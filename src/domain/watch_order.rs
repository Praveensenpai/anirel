//! Watch order inside a franchise.
//!
//! The order is by release date, most recent last: it is the only order a
//! viewer can rely on. Entries with an unknown release date go last, and a
//! title's own format and id are the tie breakers.

use super::cluster::RelationGraph;
use super::model::Anime;
use super::relation::RelationKind;

/// Order the titles of a franchise by release date.
///
/// Relations are not used for ordering; they only describe each entry.
pub fn order_members(graph: &RelationGraph, ids: &[i64]) -> Vec<(Anime, RelationKind)> {
    let mut members: Vec<(Anime, RelationKind)> = ids
        .iter()
        .filter_map(|id| {
            graph
                .media(*id)
                .map(|anime| (anime.clone(), RelationKind::Other))
        })
        .collect();

    members.sort_by_key(|(anime, _)| {
        (
            anime.aired_from.unwrap_or((i32::MAX, 12, 31)),
            anime.format_rank(),
            anime.mal_id,
        )
    });

    members.into_iter().collect()
}
