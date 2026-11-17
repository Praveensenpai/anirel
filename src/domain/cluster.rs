//! Relation graph, franchise clustering and watch-order computation.

use std::collections::{HashMap, HashSet};

use super::model::{Anime, Library, LibraryEntry};
use super::relation::RelationKind;
use super::watch_order::order_members;

/// A directed edge in the relation graph.
#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub to: i64,
    pub kind: RelationKind,
}

/// The relation graph for everything we know about.
#[derive(Debug, Default)]
pub struct RelationGraph {
    media: HashMap<i64, Anime>,
    edges: HashMap<i64, Vec<Edge>>,
}

impl RelationGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_media(&mut self, anime: Anime) {
        self.media.insert(anime.mal_id, anime);
    }

    pub fn add_edge(&mut self, from: i64, to: i64, kind: RelationKind) {
        if from == to {
            return;
        }
        let list = self.edges.entry(from).or_default();
        if !list.iter().any(|e| e.to == to && e.kind == kind) {
            list.push(Edge { to, kind });
        }
    }

    pub fn media(&self, mal_id: i64) -> Option<&Anime> {
        self.media.get(&mal_id)
    }

    /// Every edge in the graph.
    pub fn edges(&self) -> impl Iterator<Item = (i64, &[Edge])> {
        self.edges
            .iter()
            .map(|(from, edges)| (*from, edges.as_slice()))
    }

    pub fn edges_of(&self, mal_id: i64) -> &[Edge] {
        self.edges.get(&mal_id).map_or(&[], Vec::as_slice)
    }

    pub fn contains(&self, mal_id: i64) -> bool {
        self.media.contains_key(&mal_id)
    }

    pub fn media_count(&self) -> usize {
        self.media.len()
    }

    /// Every title known to the graph.
    pub fn all_media(&self) -> Vec<Anime> {
        self.media.values().cloned().collect()
    }

    /// All ids that should be expanded further (relations of relations).
    pub fn neighbours(&self, mal_id: i64) -> Vec<i64> {
        self.edges_of(mal_id).iter().map(|e| e.to).collect()
    }
}

/// One title inside a franchise.
#[derive(Debug, Clone)]
pub struct ClusterMember {
    pub anime: Anime,
    /// Relation kind to the previous title in watch order.
    pub relation: RelationKind,
    pub entry: Option<LibraryEntry>,
}

impl ClusterMember {
    /// Not on the user's list at all.
    pub fn is_missing(&self) -> bool {
        self.entry.is_none()
    }

    /// On the list but not finished yet.
    pub fn is_unwatched(&self) -> bool {
        self.entry
            .as_ref()
            .is_some_and(|e| !e.status.is_completed())
    }

    /// Needs attention: either absent from the list or still unwatched.
    pub fn needs_action(&self) -> bool {
        self.is_missing() || self.is_unwatched()
    }

    pub fn total_duration_minutes(&self) -> Option<i64> {
        self.anime.total_duration_minutes()
    }

    pub fn watched_duration_minutes(&self) -> i64 {
        let ep_dur = self.anime.episode_duration_minutes();
        if let Some(entry) = &self.entry {
            if entry.status.is_completed() {
                self.anime
                    .episodes
                    .unwrap_or(entry.episodes_watched)
                    .max(entry.episodes_watched)
                    * ep_dur
            } else {
                entry.episodes_watched * ep_dur
            }
        } else {
            0
        }
    }

    pub fn left_duration_minutes(&self) -> i64 {
        let total = self.total_duration_minutes().unwrap_or(0);
        let watched = self.watched_duration_minutes();
        total.saturating_sub(watched)
    }
}

/// A franchise: the root title plus every related title we found.
#[derive(Debug, Clone)]
pub struct Cluster {
    pub root_id: i64,
    pub name: String,
    pub members: Vec<ClusterMember>,
    /// The highest score the user gave to any title in this franchise.
    pub best_score: Option<i64>,
}

impl Cluster {
    pub fn owned_count(&self) -> usize {
        self.members.iter().filter(|m| !m.is_missing()).count()
    }

    pub fn missing_count(&self) -> usize {
        self.members.iter().filter(|m| m.is_missing()).count()
    }

    /// Titles that are missing *or* still unwatched.
    pub fn action_count(&self) -> usize {
        self.members.iter().filter(|m| m.needs_action()).count()
    }

    pub fn total_count(&self) -> usize {
        self.members.len()
    }

    pub fn missing(&self) -> impl Iterator<Item = &ClusterMember> {
        self.members.iter().filter(|m| m.is_missing())
    }

    pub fn member_ids(&self) -> Vec<i64> {
        self.members.iter().map(|m| m.anime.mal_id).collect()
    }

    pub fn total_duration_minutes(&self) -> i64 {
        self.members
            .iter()
            .filter_map(|m| m.total_duration_minutes())
            .sum()
    }

    pub fn watched_duration_minutes(&self) -> i64 {
        self.members
            .iter()
            .map(|m| m.watched_duration_minutes())
            .sum()
    }

    pub fn left_duration_minutes(&self) -> i64 {
        self.members
            .iter()
            .map(|m| m.left_duration_minutes())
            .sum()
    }
}

/// Disjoint-set over anime ids.
#[derive(Debug)]
struct UnionFind {
    parent: HashMap<i64, i64>,
}

impl UnionFind {
    fn new() -> Self {
        Self {
            parent: HashMap::new(),
        }
    }

    fn find(&mut self, x: i64) -> i64 {
        let p = *self.parent.entry(x).or_insert(x);
        if p == x {
            return x;
        }
        let root = self.find(p);
        self.parent.insert(x, root);
        root
    }

    fn union(&mut self, a: i64, b: i64) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra != rb {
            self.parent.insert(rb, ra);
        }
    }
}

/// Build franchises from the relation graph.
///
/// Only clusters that contain at least one title from the user's library are
/// returned - a chain of titles the user does not own is noise.
pub fn build_clusters(graph: &RelationGraph, library: &Library) -> Vec<Cluster> {
    let mut uf = UnionFind::new();
    for anime in graph.media.values() {
        uf.find(anime.mal_id);
    }

    // Franchise edges merge titles into one family.
    for (from, edges) in &graph.edges {
        for edge in edges {
            if !edge.kind.is_franchise() {
                continue;
            }
            if graph.contains(*from) && graph.contains(edge.to) {
                uf.union(*from, edge.to);
            }
        }
    }

    let mut groups: HashMap<i64, Vec<i64>> = HashMap::new();
    for anime in graph.media.values() {
        let root = uf.find(anime.mal_id);
        groups.entry(root).or_default().push(anime.mal_id);
    }

    let mut clusters: Vec<Cluster> = groups
        .into_values()
        .filter(|ids| ids.iter().any(|id| library.contains(*id)))
        .filter_map(|ids| build_cluster(graph, library, &ids))
        .collect();

    // Most to catch up on first; franchises the user rates highly win ties.
    clusters.sort_by(|a, b| {
        b.missing_count()
            .cmp(&a.missing_count())
            .then_with(|| b.best_score.cmp(&a.best_score))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    clusters
}

/// The titles related to one seed, following franchise edges outwards.
///
/// This is the "franchise of this title": it is rooted at the seed and does
/// not swallow the rest of the component. Diving from *Yuru Camp* into
/// *Room Camp* therefore shows Room Camp and its own entries, not the whole
/// Yuru Camp tree again.
pub fn related_to(
    graph: &RelationGraph,
    library: &Library,
    seed: i64,
    hops: usize,
) -> Option<Cluster> {
    if !graph.contains(seed) {
        return None;
    }
    let mut seen: HashSet<i64> = HashSet::new();
    let mut ids: Vec<i64> = vec![seed];
    seen.insert(seed);
    let mut frontier: Vec<i64> = vec![seed];

    for _ in 0..hops {
        let mut next: Vec<i64> = Vec::new();
        for id in &frontier {
            for edge in graph.edges_of(*id) {
                if !edge.kind.is_franchise() {
                    continue;
                }
                for candidate in [edge.to, *id] {
                    if candidate != *id && graph.contains(candidate) && seen.insert(candidate) {
                        next.push(candidate);
                        ids.push(candidate);
                    }
                }
            }
            // Relations can point back, so also follow incoming edges.
            for (other, edges) in graph.edges() {
                if edges.iter().any(|e| e.to == *id && e.kind.is_franchise())
                    && seen.insert(other)
                    && graph.contains(other)
                {
                    next.push(other);
                    ids.push(other);
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }

    let ordered = order_members(graph, &ids);
    if ordered.is_empty() {
        return None;
    }
    let members: Vec<ClusterMember> = ordered
        .into_iter()
        .map(|(anime, relation)| ClusterMember {
            entry: library.get(anime.mal_id).cloned(),
            relation,
            anime,
        })
        .collect();
    let best_score = library.best_score_for(
        &members
            .iter()
            .map(|member| member.anime.mal_id)
            .collect::<Vec<_>>(),
    );

    let mut cluster = Cluster {
        root_id: seed,
        name: graph
            .media(seed)
            .map(|a| a.title.clone())
            .unwrap_or_else(|| "Unknown".into()),
        members,
        best_score,
    };
    relabel_towards_root(graph, &mut cluster);
    Some(cluster)
}

/// Assemble and order a single franchise.
fn build_cluster(graph: &RelationGraph, library: &Library, ids: &[i64]) -> Option<Cluster> {
    let ordered = order_members(graph, ids);
    if ordered.is_empty() {
        return None;
    }
    let root_id = pick_root(graph, &ordered);
    let name = graph
        .media(root_id)
        .map(|a| a.title.clone())
        .unwrap_or_else(|| "Unknown".into());

    let members: Vec<ClusterMember> = ordered
        .into_iter()
        .map(|(anime, relation)| ClusterMember {
            entry: library.get(anime.mal_id).cloned(),
            relation,
            anime,
        })
        .collect();

    let best_score = library.best_score_for(
        &members
            .iter()
            .map(|member| member.anime.mal_id)
            .collect::<Vec<_>>(),
    );

    let mut cluster = Cluster {
        root_id,
        name,
        members,
        best_score,
    };
    relabel_towards_root(graph, &mut cluster);
    Some(cluster)
}

/// Describe every member by its relation to the franchise root.
///
/// "Yuru Camp△ Season 2" is a sequel of the root, "Heya Camp" is a side story
/// of it - which reads far better than "whatever came before it in the list".
fn relabel_towards_root(graph: &RelationGraph, cluster: &mut Cluster) {
    let root = cluster.root_id;
    let order: Vec<i64> = cluster.members.iter().map(|m| m.anime.mal_id).collect();

    for (index, member) in cluster.members.iter_mut().enumerate() {
        if member.anime.mal_id == root {
            member.relation = RelationKind::Other;
            continue;
        }

        // Prefer a direct relation to the root, otherwise take the closest
        // relation to an earlier title in watch order.
        let mut label = relation_between(graph, member.anime.mal_id, root);
        if label.is_none() {
            for earlier in order.iter().take(index) {
                if let Some(kind) = relation_between(graph, member.anime.mal_id, *earlier) {
                    label = Some(kind);
                    break;
                }
            }
        }
        if let Some(kind) = label {
            member.relation = kind;
        }
    }
}

/// How does `target` relate to `source`?
fn relation_between(graph: &RelationGraph, target: i64, source: i64) -> Option<RelationKind> {
    for edge in graph.edges_of(source) {
        if edge.to == target {
            return Some(match edge.kind {
                RelationKind::Prequel => RelationKind::Prequel,
                RelationKind::Sequel => RelationKind::Sequel,
                RelationKind::SideStory => RelationKind::SideStory,
                RelationKind::Parent => RelationKind::Parent,
                RelationKind::SpinOff => RelationKind::SpinOff,
                RelationKind::Summary => RelationKind::Summary,
                RelationKind::Alternative => RelationKind::Alternative,
                RelationKind::Character => RelationKind::Character,
                RelationKind::Adaptation => RelationKind::Adaptation,
                RelationKind::Other => RelationKind::Other,
            });
        }
    }
    for edge in graph.edges_of(target) {
        if edge.to == source {
            return Some(match edge.kind {
                // "target has parent story source" => target is a spin-off.
                RelationKind::Parent => RelationKind::SideStory,
                // "target has prequel source" => target comes after source.
                RelationKind::Prequel => RelationKind::Sequel,
                // "target has sequel source" => source comes after target.
                RelationKind::Sequel => RelationKind::Prequel,
                RelationKind::SideStory => RelationKind::SideStory,
                RelationKind::SpinOff => RelationKind::SpinOff,
                RelationKind::Summary => RelationKind::Summary,
                RelationKind::Alternative => RelationKind::Alternative,
                RelationKind::Character => RelationKind::Character,
                RelationKind::Adaptation => RelationKind::Adaptation,
                RelationKind::Other => RelationKind::Other,
            });
        }
    }
    None
}

/// Pick the title that holds the franchise together.
///
/// The root is the member that the other titles hang off - the one with the
/// most franchise links inside the cluster. Earlier titles win ties, which
/// keeps "Yuru Camp△" as the name of its franchise rather than some
/// unrelated older title that happens to be linked to it.
fn pick_root(graph: &RelationGraph, members: &[(Anime, RelationKind)]) -> i64 {
    let set: HashSet<i64> = members.iter().map(|(a, _)| a.mal_id).collect();
    let mut best: Option<(usize, i32, u8, i64)> = None;

    for (anime, _) in members {
        let links = graph
            .edges_of(anime.mal_id)
            .iter()
            .filter(|edge| edge.kind.is_franchise() && set.contains(&edge.to))
            .count();
        let key = (
            links,
            anime.year.map(|y| -y).unwrap_or(i32::MIN),
            anime.format_rank(),
            anime.mal_id,
        );
        if best.is_none_or(|current| key > current) {
            best = Some(key);
        }
    }
    best.map(|(_, _, _, id)| id).unwrap_or_default()
}
