//! Interactive TUI runner.

use std::io::Stdout;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use tokio::sync::mpsc;

use crate::app::actions::action_for;
use crate::app::scanner::{self, ScanEvent};
use crate::app::state::{AppState, Loading, MessageKind};
use crate::app::update::{Effect, apply, notify};
use crate::config::Config;
use crate::domain::model::Library;
use crate::engine::Engine;
use crate::output::{self, FranchiseReport};
use crate::sys;

/// Run the interactive interface.
pub async fn run(config: Config, library: Library, engine: Engine) -> anyhow::Result<()> {
    let scope = config.scope;
    let (tx, mut rx) = mpsc::unbounded_channel::<ScanEvent>();

    let mut state = AppState::new(config.user.clone(), library, scope);

    let mut terminal = ratatui::init();
    let result = event_loop(&mut terminal, &mut state, &mut rx, &tx, &engine).await;
    ratatui::restore();
    result
}

async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &mut AppState,
    rx: &mut mpsc::UnboundedReceiver<ScanEvent>,
    tx: &mpsc::UnboundedSender<ScanEvent>,
    engine: &Engine,
) -> anyhow::Result<()> {
    let mut pending_covers: std::collections::HashSet<i64> = std::collections::HashSet::new();

    loop {
        terminal.draw(|f| crate::ui::draw(f, state))?;

        if event::poll(Duration::from_millis(80))? {
            match event::read()? {
                Event::Key(key) if key.kind != KeyEventKind::Release => {
                    let action = action_for(key, state.search_mode);
                    if let Some(effect) = apply(state, action) {
                        handle_effect(state, effect, tx, engine);
                    }
                }
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        request_cover(state, tx, &mut pending_covers);

        while let Ok(event) = rx.try_recv() {
            match event {
                ScanEvent::Resolved(items) => {
                    for item in items {
                        state.ingest(item.anime, item.relations);
                    }
                    state.rebuild();
                }
                ScanEvent::Progress { done, total } => {
                    state.scan.done = done;
                    state.scan.total = total;
                }
                ScanEvent::Finished => {
                    state.scan.running = false;
                    state.scan.done = state.scan.total;
                    state.loading = None;
                    state.rebuild();
                }
                ScanEvent::Failed(error) => {
                    state.scan.running = false;
                    state.scan.error = Some(error);
                    state.loading = None;
                }
                ScanEvent::CoverReady(mal_id, art) => {
                    state.covers.insert(mal_id, art);
                }
                ScanEvent::LoadComplete(mal_id) => {
                    let hops = state.loading.as_ref().map(|l| l.hops).unwrap_or(2);
                    // The franchise is rooted at the title that was loaded, so
                    // diving into a side story shows that side story's subtree.
                    let cluster = crate::domain::cluster::related_to(
                        &state.graph,
                        &state.library,
                        mal_id,
                        hops,
                    );
                    if let Some(cluster) = cluster {
                        state.show_focus_cluster(cluster);
                    }
                    let title = state
                        .loading
                        .take()
                        .map(|loading| loading.anime.display_title().to_string())
                        .or_else(|| {
                            state
                                .graph
                                .media(mal_id)
                                .map(|anime| anime.display_title().to_string())
                        })
                        .unwrap_or_default();
                    notify(state, format!("loaded {title}"), MessageKind::Success);
                }
            }
        }

        if state.should_quit {
            break;
        }
    }
    Ok(())
}

fn handle_effect(
    state: &mut AppState,
    effect: Effect,
    tx: &mpsc::UnboundedSender<ScanEvent>,
    engine: &Engine,
) {
    match effect {
        Effect::Open(url) => match sys::open_in_browser(&url) {
            Ok(()) => notify(state, "opened in browser", MessageKind::Success),
            Err(error) => notify(state, error, MessageKind::Error),
        },
        Effect::Copy(url) => match sys::copy_to_clipboard(&url) {
            Ok(()) => notify(state, format!("copied {url}"), MessageKind::Success),
            Err(error) => notify(state, error, MessageKind::Error),
        },
        Effect::Export => {
            let reports = build_reports(state);
            let path = export_path();
            match std::fs::write(&path, output::markdown_report(&reports, &state.user)) {
                Ok(()) => notify(
                    state,
                    format!("report written to {}", path.display()),
                    MessageKind::Success,
                ),
                Err(error) => notify(state, format!("export failed: {error}"), MessageKind::Error),
            }
        }
        Effect::Rescan => {
            let _ = engine.cache.clear_media();
            state.scan = Default::default();
            state.scan.running = true;
            let library = state.library.clone();
            spawn_scan(tx, engine, library);
            notify(state, "rescanning library…", MessageKind::Info);
        }
        Effect::Load { mal_id, hops } => {
            let Some(anime) = state
                .library
                .get(mal_id)
                .map(|entry| entry.anime.clone())
                .or_else(|| state.graph.media(mal_id).cloned())
            else {
                return;
            };
            let already_focused = state
                .focus_cluster
                .as_ref()
                .is_some_and(|cluster| cluster.root_id == mal_id);
            if already_focused {
                notify(
                    state,
                    format!("{} is already loaded", anime.display_title()),
                    MessageKind::Info,
                );
                return;
            }
            state.loading = Some(Loading {
                anime: anime.clone(),
                detail: "fetching related entries…".into(),
                hops,
            });
            spawn_load(tx, engine, mal_id, hops);
        }
        Effect::ScanAll => {
            if state.scan.running {
                return;
            }
            state.scan = Default::default();
            state.scan.running = true;
            let library = state.library.clone();
            spawn_scan(tx, engine, library);
            notify(state, "loading every franchise…", MessageKind::Info);
        }
    }
}

/// Fetch the cover art of the selected title once, in the background.
fn request_cover(
    state: &mut AppState,
    tx: &mpsc::UnboundedSender<ScanEvent>,
    pending: &mut std::collections::HashSet<i64>,
) {
    let Some(anime) = state.current_anime() else {
        return;
    };
    let mal_id = anime.mal_id;
    if state.covers.contains_key(&mal_id) || pending.contains(&mal_id) {
        return;
    }
    let Some(url) = anime.cover_url.clone() else {
        pending.insert(mal_id);
        return;
    };

    let path = crate::art::cover_path(&url);
    if path.exists() {
        if let Ok(art) = crate::art::decode(&path) {
            state.covers.insert(mal_id, art);
        } else {
            pending.insert(mal_id);
        }
        return;
    }

    pending.insert(mal_id);
    if std::env::var_os("ANIREL_DEBUG").is_some() {
        eprintln!("fetching cover for {mal_id}: {url}");
    }
    let tx = tx.clone();
    tokio::spawn(async move {
        if let Some(path) = crate::art::ensure_cached(&url).await
            && let Ok(art) = crate::art::decode(&path)
        {
            tx.send(ScanEvent::CoverReady(mal_id, art)).ok();
        }
    });
}

/// Load the relations of a single title, on demand.
fn spawn_load(tx: &mpsc::UnboundedSender<ScanEvent>, engine: &Engine, mal_id: i64, hops: usize) {
    let engine = engine.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let mut graph = crate::domain::cluster::RelationGraph::new();
        if let Ok(resolved) = engine.resolve_into(&[mal_id], &mut graph).await {
            for item in resolved {
                tx.send(ScanEvent::Resolved(vec![item])).ok();
            }
        }
        if let Err(error) = engine.expand(&mut graph, &[mal_id], hops).await {
            tx.send(ScanEvent::Failed(error.to_string())).ok();
            return;
        }
        for item in graph_media(&graph) {
            tx.send(ScanEvent::Resolved(vec![item])).ok();
        }
        tx.send(ScanEvent::LoadComplete(mal_id)).ok();
    });
}

/// Every resolved title of a graph, as scan events.
///
/// The relations have to travel with the media: the receiving side rebuilds
/// its own graph, so edges that are not sent are lost.
fn graph_media(
    graph: &crate::domain::cluster::RelationGraph,
) -> Vec<crate::infra::anilist::ResolvedMedia> {
    graph
        .all_media()
        .into_iter()
        .map(|anime| {
            let relations = graph
                .edges_of(anime.mal_id)
                .iter()
                .map(|edge| (edge.to, edge.kind))
                .collect();
            crate::infra::anilist::ResolvedMedia { anime, relations }
        })
        .collect()
}

fn spawn_scan(tx: &mpsc::UnboundedSender<ScanEvent>, engine: &Engine, library: Library) {
    let engine = engine.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        scanner::run(engine, library, tx).await;
    });
}

fn build_reports(state: &AppState) -> Vec<FranchiseReport> {
    state
        .clusters
        .iter()
        .map(|cluster| output::report_from_cluster(cluster, &state.library))
        .collect()
}

fn export_path() -> std::path::PathBuf {
    crate::config::cache_dir().join("report.md")
}
