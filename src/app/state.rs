//! Application state and pure state transitions.

use crate::app::fuzzy;
use crate::config::Scope;
use crate::domain::cluster::{Cluster, RelationGraph, build_clusters};
use crate::domain::model::{Anime, Library, LibraryEntry};
use crate::domain::relation::RelationKind;

/// Order in which the library list is sorted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    /// Alphabetical by display title (A→Z).
    Alphabetical,
    /// Original MAL list order (most-recently-updated first).
    LastUpdated,
    /// User score descending; unscored entries last.
    Score,
    /// Release year descending; unknown-year entries last.
    Year,
}

impl SortOrder {
    /// Short label shown in the footer.
    pub fn label(self) -> &'static str {
        match self {
            Self::Alphabetical => "A–Z",
            Self::LastUpdated => "Last Updated",
            Self::Score => "Score",
            Self::Year => "Year",
        }
    }

    /// Cycle to the next sort mode.
    pub fn next(self) -> Self {
        match self {
            Self::Alphabetical => Self::LastUpdated,
            Self::LastUpdated => Self::Score,
            Self::Score => Self::Year,
            Self::Year => Self::Alphabetical,
        }
    }
}

/// Which list the main pane is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Your own MyAnimeList entries.
    Library,
    /// Related entries missing from the loaded franchises.
    Missing,
}

impl View {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Library => "Library",
            Self::Missing => "Missing",
        }
    }
}

/// Which pane has keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    List,
    Detail,
}

/// A row in the main list.
#[derive(Debug, Clone, Copy)]
pub enum Row {
    /// An entry of the user's library, by index.
    Library(usize),
    /// A missing entry of a loaded franchise.
    Gap { cluster: usize, member: usize },
}

/// A load that is currently in flight.
#[derive(Debug, Clone)]
pub struct Loading {
    pub anime: Anime,
    pub detail: String,
    /// How many relation hops to follow.
    pub hops: usize,
}

/// Initial library load state when starting with cold cache.
#[derive(Debug, Clone)]
pub struct InitialLoading {
    pub user: String,
    pub count: usize,
    pub error: Option<String>,
}

/// Scan progress shown in the status bar.
#[derive(Debug, Clone, Default)]
pub struct ScanStatus {
    pub done: usize,
    pub total: usize,
    pub running: bool,
    pub error: Option<String>,
}

/// Transient message shown under the status bar.
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub kind: MessageKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Info,
    Success,
    Error,
}

/// The complete application state.
pub struct AppState {
    pub user: String,
    pub library: Library,
    pub graph: RelationGraph,
    /// Franchises that have been loaded so far.
    pub clusters: Vec<Cluster>,
    pub view: View,
    pub sort: SortOrder,
    pub filter: String,
    pub search_mode: bool,
    pub selected: usize,
    pub member_selected: usize,
    pub focus: Focus,
    pub show_help: bool,
    pub should_quit: bool,
    pub message: Option<StatusMessage>,
    pub loading: Option<Loading>,
    pub initial_loading: Option<InitialLoading>,
    pub scan: ScanStatus,
    pub scope: Scope,
    /// Which franchise the detail pane shows. `None` follows the selection.
    pub detail_root: Option<i64>,
    /// A franchise rooted at a specific title, shown after a dive.
    pub focus_cluster: Option<crate::domain::cluster::Cluster>,
    rows: Vec<Row>,
}

/// Does a title match the filter? Original romaji, English or Japanese title.
fn matches_filter(anime: &Anime, filter: &str) -> bool {
    fuzzy::matches(&anime.title, filter)
        || anime
            .title_english
            .as_deref()
            .is_some_and(|title| fuzzy::matches(title, filter))
        || anime
            .title_japanese
            .as_deref()
            .is_some_and(|title| fuzzy::matches(title, filter))
}

impl AppState {
    pub fn new(user: String, library: Library, scope: Scope) -> Self {
        let mut state = Self {
            user,
            library,
            graph: RelationGraph::new(),
            clusters: Vec::new(),
            view: View::Library,
            sort: SortOrder::Alphabetical,
            filter: String::new(),
            search_mode: false,
            selected: 0,
            member_selected: 0,
            focus: Focus::List,
            show_help: false,
            should_quit: false,
            message: None,
            loading: None,
            initial_loading: None,
            scan: ScanStatus::default(),
            scope,
            detail_root: None,
            focus_cluster: None,
            rows: Vec::new(),
        };
        state.rebuild_rows();
        state
    }

    pub fn set_initial_loading(&mut self, user: String) {
        self.initial_loading = Some(InitialLoading {
            user,
            count: 0,
            error: None,
        });
    }

    pub fn update_initial_progress(&mut self, count: usize) {
        if let Some(loading) = &mut self.initial_loading {
            loading.count = count;
        }
    }

    pub fn complete_initial_loading(&mut self, library: Library) {
        self.initial_loading = None;
        self.library = library;
        self.rebuild();
    }

    pub fn fail_initial_loading(&mut self, error: String) {
        if let Some(loading) = &mut self.initial_loading {
            loading.error = Some(error);
        }
    }

    /// Integrate newly resolved titles and recompute the franchises.
    pub fn ingest(&mut self, anime: Anime, relations: Vec<(i64, RelationKind)>) {
        let from = anime.mal_id;
        if let Some(entry) = self.library.get_mut(from) {
            entry.anime = anime.clone();
        }
        self.graph.insert_media(anime);
        for (target, kind) in relations {
            self.graph.add_edge(from, target, kind);
        }
    }

    /// Recompute franchises and the visible rows.
    pub fn rebuild(&mut self) {
        self.clusters = build_clusters(&self.graph, &self.library);
        self.rebuild_rows();
    }

    fn rebuild_rows(&mut self) {
        let filter = self.filter.to_lowercase();
        let mut rows = Vec::new();

        match self.view {
            View::Library => {
                let mut entries: Vec<(usize, &LibraryEntry)> =
                    self.library.entries.iter().enumerate().collect();
                match self.sort {
                    SortOrder::Alphabetical => {
                        entries.sort_by_key(|(_, e)| e.anime.display_title().to_lowercase());
                    }
                    SortOrder::LastUpdated => {
                        // MAL list order is chronological ascending; reverse for most-recent first.
                        entries.reverse();
                    }
                    SortOrder::Score => {
                        entries.sort_by_key(|(_, a)| std::cmp::Reverse(a.score));
                    }
                    SortOrder::Year => {
                        entries.sort_by_key(|(_, a)| std::cmp::Reverse(a.anime.year));
                    }
                }
                for (index, entry) in entries {
                    if !filter.is_empty() && !matches_filter(&entry.anime, &filter) {
                        continue;
                    }
                    rows.push(Row::Library(index));
                }
            }
            View::Missing => {
                for (cluster_index, cluster) in self.clusters.iter().enumerate() {
                    for (member_index, member) in cluster.members.iter().enumerate() {
                        if !member.needs_action() || !member.relation.is_gap_relevant() {
                            continue;
                        }
                        if !filter.is_empty()
                            && !cluster.name.to_lowercase().contains(&filter)
                            && !matches_filter(&member.anime, &filter)
                        {
                            continue;
                        }
                        rows.push(Row::Gap {
                            cluster: cluster_index,
                            member: member_index,
                        });
                    }
                }
            }
        }

        self.rows = rows;
        if self.selected >= self.rows.len() {
            self.selected = self.rows.len().saturating_sub(1);
        }
        self.member_selected = 0;
    }

    pub fn set_sort(&mut self, sort: SortOrder) {
        self.sort = sort;
        self.selected = 0;
        self.member_selected = 0;
        self.rebuild_rows();
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    pub fn set_view(&mut self, view: View) {
        self.view = view;
        self.selected = 0;
        self.member_selected = 0;
        self.rebuild_rows();
    }

    pub fn set_filter(&mut self, filter: String) {
        self.filter = filter;
        self.selected = 0;
        self.member_selected = 0;
        self.rebuild_rows();
    }

    /// The library entry the current row points at.
    pub fn current_entry(&self) -> Option<&LibraryEntry> {
        match self.rows.get(self.selected) {
            Some(Row::Library(index)) => self.library.entries.get(*index),
            Some(Row::Gap { cluster, member }) => self
                .clusters
                .get(*cluster)
                .and_then(|c| c.members.get(*member))
                .and_then(|m| self.library.get(m.anime.mal_id)),
            None => None,
        }
    }

    /// The franchise the detail pane shows.
    ///
    /// When a related title has been loaded from inside a franchise, the
    /// pane follows it - that is how you dive from Yuru Camp into
    /// Heya Camp's own subtree.
    pub fn detail_cluster(&self) -> Option<&Cluster> {
        if let Some(cluster) = &self.focus_cluster {
            return Some(cluster);
        }
        if let Some(root) = self.detail_root
            && let Some(cluster) = self.clusters.iter().find(|c| c.root_id == root)
        {
            return Some(cluster);
        }
        self.selection_cluster()
    }

    /// The member highlighted in the detail pane.
    pub fn detail_member(&self) -> Option<&crate::domain::cluster::ClusterMember> {
        self.detail_cluster()?.members.get(self.member_selected)
    }

    /// Point the detail pane at a specific franchise.
    pub fn show_cluster(&mut self, root_id: i64) {
        self.detail_root = Some(root_id);
        self.focus_cluster = None;
        self.member_selected = 0;
    }

    /// Point the detail pane at a franchise rooted at one title.
    pub fn show_focus_cluster(&mut self, cluster: crate::domain::cluster::Cluster) {
        self.focus_cluster = Some(cluster);
        self.detail_root = None;
        self.member_selected = 0;
    }

    /// The franchise the current row belongs to, if it has been loaded.
    pub fn selection_cluster(&self) -> Option<&Cluster> {
        match self.rows.get(self.selected) {
            Some(Row::Library(index)) => {
                let mal_id = self.library.entries.get(*index)?.anime.mal_id;
                self.cluster_for(mal_id)
            }
            Some(Row::Gap { cluster, .. }) => self.clusters.get(*cluster),
            None => None,
        }
    }

    /// The loaded franchise that contains this title.
    pub fn cluster_for(&self, mal_id: i64) -> Option<&Cluster> {
        self.clusters
            .iter()
            .find(|cluster| cluster.members.iter().any(|m| m.anime.mal_id == mal_id))
    }

    /// The title the keyboard acts on.
    ///
    /// In the franchise pane that is the highlighted member - so `L` loads
    /// *that* title's franchise and `o` opens *that* title's page. In the
    /// library pane it is simply the selected entry.
    pub fn focus_target(&self) -> Option<&Anime> {
        match self.focus {
            Focus::Detail => self.detail_member().map(|member| &member.anime),
            Focus::List => self.current_anime(),
        }
    }

    /// The anime the user should act on (open/copy/load).
    pub fn current_anime(&self) -> Option<&Anime> {
        let mal_id = match self.rows.get(self.selected) {
            Some(Row::Library(index)) => self.library.entries.get(*index)?.anime.mal_id,
            Some(Row::Gap { cluster, member }) => {
                self.clusters
                    .get(*cluster)?
                    .members
                    .get(*member)?
                    .anime
                    .mal_id
            }
            None => return None,
        };
        if let Some(anime) = self.graph.media(mal_id) {
            return Some(anime);
        }
        match self.rows.get(self.selected) {
            Some(Row::Library(index)) => Some(&self.library.entries.get(*index)?.anime),
            Some(Row::Gap { cluster, member }) => {
                Some(&self.clusters.get(*cluster)?.members.get(*member)?.anime)
            }
            None => None,
        }
    }

    /// Total number of missing or unwatched entries in the loaded franchises.
    pub fn total_gaps(&self) -> usize {
        self.clusters
            .iter()
            .flat_map(|c| c.members.iter())
            .filter(|m| m.needs_action() && m.relation.is_gap_relevant())
            .count()
    }

    pub fn library_entry(&self, mal_id: i64) -> Option<&LibraryEntry> {
        self.library.get(mal_id)
    }
}
