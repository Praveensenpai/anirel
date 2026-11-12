//! Snapshot test for the TUI rendering.

use anirel::app::state::{AppState, Focus, Row, View};
use anirel::config::Scope;
use anirel::domain::model::{Anime, Library, LibraryEntry, LibraryStatus};
use anirel::domain::relation::RelationKind;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn anime(mal_id: i64, title: &str, year: i32, format: &str) -> Anime {
    Anime {
        mal_id,
        title: title.into(),
        title_english: None,
        format: format.into(),
        episodes: Some(12),
        airing_status: "Finished Airing".into(),
        year: Some(year),
        score: Some(8.0),
        cover_url: None,
    }
}

fn library() -> Library {
    Library::new(vec![
        LibraryEntry {
            anime: anime(1, "Yuru Camp", 2018, "TV"),
            status: LibraryStatus::Completed,
            score: 10,
            episodes_watched: 12,
        },
        LibraryEntry {
            anime: anime(2, "Heya Camp", 2020, "TV_SHORT"),
            status: LibraryStatus::PlanToWatch,
            score: 0,
            episodes_watched: 0,
        },
    ])
}

/// A state where the franchise of "Yuru Camp" has been loaded.
fn loaded_state() -> AppState {
    let mut state = AppState::new("Praveen20".into(), library(), Scope::default());
    state.ingest(
        anime(1, "Yuru Camp", 2018, "TV"),
        vec![(2, RelationKind::Sequel), (3, RelationKind::SideStory)],
    );
    state.ingest(anime(2, "Heya Camp", 2020, "TV_SHORT"), vec![]);
    state.ingest(anime(3, "Room Camp", 2020, "TV_SHORT"), vec![]);
    state.rebuild();
    state
}

fn render(state: &AppState, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| anirel::ui::draw(f, state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn renders_the_library_pane_and_the_poster_card() {
    let state = loaded_state();
    let frame = render(&state, 130, 26);

    assert!(frame.contains("anirel"), "the brand is shown:\n{frame}");
    assert!(frame.contains("Praveen20"), "the user is shown:\n{frame}");
    assert!(
        frame.contains("MY LIBRARY"),
        "the library pane is shown:\n{frame}"
    );
    assert!(frame.contains("Yuru Camp"), "the title is listed:\n{frame}");
    assert!(
        frame.contains("POSTER"),
        "the poster card is shown:\n{frame}"
    );
}

#[test]
fn an_unloaded_title_explains_how_to_load_it() {
    let state = AppState::new("Praveen20".into(), library(), Scope::default());
    let frame = render(&state, 130, 26);

    assert!(
        frame.contains("to load its franchise"),
        "the lazy load hint is shown:\n{frame}"
    );
}

#[test]
fn a_loaded_franchise_lists_its_members_and_marks_the_gaps() {
    let state = loaded_state();
    let frame = render(&state, 130, 26);

    assert!(
        frame.contains("Room Camp"),
        "the member is listed:\n{frame}"
    );
    assert!(
        frame.contains("missing · side story"),
        "the gap is marked:\n{frame}"
    );
    assert!(
        frame.contains("plan-to-watch · sequel"),
        "the unwatched title is marked:\n{frame}"
    );
}

#[test]
fn the_missing_view_lists_the_gaps_of_loaded_franchises() {
    let mut state = loaded_state();
    state.set_view(View::Missing);
    let frame = render(&state, 130, 26);

    assert!(
        frame.contains("MISSING RELATED"),
        "the view switched:\n{frame}"
    );
    assert!(frame.contains("Room Camp"), "the gap is listed:\n{frame}");
}

#[test]
fn the_missing_view_is_empty_before_anything_is_loaded() {
    let mut state = AppState::new("Praveen20".into(), library(), Scope::default());
    state.set_view(View::Missing);
    let frame = render(&state, 130, 26);

    assert!(
        frame.contains("No franchise loaded yet"),
        "the empty state explains itself:\n{frame}"
    );
}

#[test]
fn the_detail_pane_can_follow_a_related_title() {
    let mut state = loaded_state();
    // Point the detail pane at the franchise that contains "Room Camp".
    let root = state
        .clusters
        .iter()
        .find(|cluster| cluster.members.iter().any(|m| m.anime.mal_id == 3))
        .map(|cluster| cluster.root_id)
        .expect("a cluster with Room Camp");
    state.show_cluster(root);
    state.focus = Focus::Detail;

    let frame = render(&state, 130, 26);
    assert!(
        frame.contains("Room Camp"),
        "the followed title is shown:\n{frame}"
    );
}

#[test]
fn the_library_row_is_the_focus_target_until_the_detail_pane_is_focused() {
    let state = loaded_state();
    // The library is sorted by title, so "Heya Camp" comes first.
    let target = state.focus_target().expect("a focus target");
    assert_eq!(target.mal_id, 2, "the selected library entry is the target");

    let mut state = state;
    state.focus = Focus::Detail;
    let target = state.focus_target().expect("a focus target");
    assert_eq!(target.mal_id, 1, "the first member of the franchise");
}

#[test]
fn a_tiny_terminal_asks_for_more_room_instead_of_panicking() {
    let state = loaded_state();
    let frame = render(&state, 30, 8);
    assert!(
        frame.contains("needs at least"),
        "a hint is shown:\n{frame}"
    );
}

#[test]
fn the_library_rows_are_indexed_correctly_after_sorting() {
    let state = loaded_state();
    let row = state.rows().first().copied().expect("a row");
    let Row::Library(index) = row else {
        panic!("expected a library row");
    };
    let entry = state.library.entries.get(index).expect("the entry");
    assert_eq!(entry.anime.title, "Heya Camp", "rows are sorted by title");
}
