//! Watch order inside a franchise.
//!
//! The order is a topological sort over the relation edges: a prequel comes
//! before its sequel, a side story after the season it belongs to. Release
//! year, format and title break ties.

use std::collections::{BTreeSet, HashMap, HashSet};

use super::cluster::RelationGraph;
use super::model::Anime;
use super::relation::RelationKind;

/// Order the titles of a franchise for watching.
pub fn order_members(graph: &RelationGraph, ids: &[i64]) -> Vec<(Anime, RelationKind)> {
    let set: HashSet<i64> = ids.iter().copied().collect();
    let mut before: HashMap<i64, HashSet<i64>> = HashMap::new();
    let mut kind_between: HashMap<(i64, i64), RelationKind> = HashMap::new();

    for from in &set {
        for edge in graph.edges_of(*from) {
            if !set.contains(&edge.to) || !edge.kind.is_franchise() {
                continue;
            }
            // "X has a prequel Y" => Y comes first.
            let (earlier, later, kind) = match edge.kind {
                RelationKind::Prequel | RelationKind::Parent => (edge.to, *from, edge.kind),
                _ => (*from, edge.to, edge.kind),
            };
            before.entry(later).or_default().insert(earlier);
            kind_between
                .entry((earlier, later))
                .and_modify(|existing| {
                    if kind_priority(kind) > kind_priority(*existing) {
                        *existing = kind;
                    }
                })
                .or_insert(kind);
        }
    }

    let mut remaining: BTreeSet<(i32, u8, i64)> = ids
        .iter()
        .filter_map(|id| {
            graph
                .media(*id)
                .map(|a| (a.year.unwrap_or(i32::MAX), a.format_rank(), a.mal_id))
        })
        .collect();

    let mut ordered: Vec<i64> = Vec::with_capacity(ids.len());
    let mut done: HashSet<i64> = HashSet::new();

    while !remaining.is_empty() {
        let next = remaining.iter().find_map(|key| {
            let id = key.2;
            let ready = before
                .get(&id)
                .map(|deps| deps.iter().all(|d| done.contains(d)))
                .unwrap_or(true);
            ready.then_some(id)
        });

        let id = match next {
            Some(id) => id,
            None => {
                // Cycle (summary relations often loop): take the earliest and continue.
                let key = *remaining.iter().next().expect("non-empty");
                key.2
            }
        };
        remaining.remove(&(key_of(graph, id)));
        done.insert(id);
        ordered.push(id);
    }

    let mut result: Vec<(Anime, RelationKind)> = Vec::with_capacity(ordered.len());
    let mut previous: Option<i64> = None;
    for id in ordered {
        let anime = match graph.media(id) {
            Some(a) => a.clone(),
            None => continue,
        };
        let relation = previous
            .and_then(|p| kind_between.get(&(p, id)).copied())
            .unwrap_or(RelationKind::Other);
        result.push((anime, relation));
        previous = Some(id);
    }
    result
}

fn kind_priority(kind: RelationKind) -> u8 {
    match kind {
        RelationKind::SideStory => 7,
        RelationKind::SpinOff => 6,
        RelationKind::Alternative => 5,
        RelationKind::Summary => 4,
        RelationKind::Parent => 3,
        RelationKind::Prequel | RelationKind::Sequel => 2,
        RelationKind::Character => 1,
        RelationKind::Adaptation | RelationKind::Other => 0,
    }
}

fn key_of(graph: &RelationGraph, id: i64) -> (i32, u8, i64) {
    graph
        .media(id)
        .map(|a| (a.year.unwrap_or(i32::MAX), a.format_rank(), a.mal_id))
        .unwrap_or((i32::MAX, 9, id))
}
