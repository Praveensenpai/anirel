//! Snapshot test for the TUI rendering.

use anirel::app::actions::{Action, action_for};
use anirel::app::state::{AppState, Focus, Row, View};
use anirel::app::update::{Effect, apply};
use anirel::config::Scope;
use anirel::domain::model::{Anime, Library, LibraryEntry, LibraryStatus};
use anirel::domain::relation::RelationKind;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn anime(mal_id: i64, title: &str, year: i32, format: &str) -> Anime {
    Anime {
        mal_id,
        title: title.into(),
        title_english: None,
        title_japanese: None,
        format: format.into(),
        episodes: Some(12),
        duration: None,
        airing_status: "Finished Airing".into(),
        year: Some(year),
        aired_from: Some((year, 1, 1)),
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
            updated_at: None,
        },
        LibraryEntry {
            anime: anime(2, "Heya Camp", 2020, "TV_SHORT"),
            status: LibraryStatus::PlanToWatch,
            score: 0,
            episodes_watched: 0,
            updated_at: None,
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
fn poster_renders_romaji_english_and_japanese_titles() {
    let mut state = AppState::new("Praveen20".into(), library(), Scope::default());
    let mut item = anime(1, "Yuru Camp△", 2018, "TV");
    item.title_english = Some("Laid-Back Camp".into());
    item.title_japanese = Some("ゆるキャン△".into());
    state.ingest(item, vec![]);
    state.rebuild();
    // Select Yuru Camp△ which is at index 1 after alphabetical sort
    state.selected = 1;

    let frame = render(&state, 140, 28);
    assert!(frame.contains("Yuru Camp△"), "romaji title shown:\n{frame}");
    assert!(
        frame.contains("Laid-Back Camp"),
        "english title shown:\n{frame}"
    );
    assert!(
        frame.contains('ゆ') && frame.contains('る'),
        "japanese title shown:\n{frame}"
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

#[test]
fn a_typoed_filter_still_narrows_the_library() {
    let mut state = loaded_state();
    state.set_filter("yorucamp".into());
    let rows = state.rows();
    assert_eq!(rows.len(), 1, "only Yuru Camp matches:\n{rows:?}");
    assert_eq!(
        state.current_anime().map(|anime| anime.mal_id),
        Some(1),
        "the fuzzy match selects Yuru Camp"
    );
}

#[test]
fn the_force_refresh_keys_request_a_library_refresh() {
    let f = action_for(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE), false);
    let ctrl_r = action_for(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        false,
    );
    assert_eq!(f, Action::RefreshLibrary);
    assert_eq!(ctrl_r, Action::RefreshLibrary);

    let mut state = loaded_state();
    assert_eq!(
        apply(&mut state, Action::RefreshLibrary),
        Some(Effect::RefreshLibrary)
    );
}

#[test]
fn initial_loading_screen_renders_progress_and_completion() {
    let mut state = AppState::new("Praveen20".into(), Library::default(), Scope::default());
    state.set_initial_loading("Praveen20".into());
    state.update_initial_progress(600);

    let frame = render(&state, 120, 24);
    assert!(
        frame.contains("Loading Library"),
        "the loading title is displayed:\n{frame}"
    );
    assert!(
        frame.contains("600 titles fetched"),
        "the progress count is displayed:\n{frame}"
    );

    // Complete loading
    state.complete_initial_loading(library());
    assert!(state.initial_loading.is_none());
    let frame = render(&state, 120, 24);
    assert!(
        !frame.contains("Loading Library"),
        "the loading overlay is gone after completion:\n{frame}"
    );
    assert!(
        frame.contains("MY LIBRARY"),
        "the library is visible:\n{frame}"
    );
}

#[test]
fn tui_renders_duration_and_watched_left() {
    let state = loaded_state();
    let frame = render(&state, 140, 26);

    assert!(
        frame.contains("total"),
        "total duration is rendered:\n{frame}"
    );
    assert!(
        frame.contains("watched"),
        "watched duration is rendered:\n{frame}"
    );
    assert!(
        frame.contains("left"),
        "left duration is rendered:\n{frame}"
    );
}

#[test]
fn sort_by_last_updated_orders_by_timestamp_descending() {
    let entries = vec![
        LibraryEntry {
            anime: anime(1, "Old Title", 2010, "TV"),
            status: LibraryStatus::Completed,
            score: 7,
            episodes_watched: 12,
            updated_at: Some(1000),
        },
        LibraryEntry {
            anime: anime(2, "Recent Title", 2024, "TV"),
            status: LibraryStatus::Watching,
            score: 9,
            episodes_watched: 3,
            updated_at: Some(2000),
        },
    ];
    let mut state = AppState::new("user".into(), Library::new(entries), Scope::default());
    assert_eq!(state.current_anime().unwrap().title, "Old Title");

    state.set_sort(anirel::app::state::SortOrder::LastUpdated);
    assert_eq!(state.current_anime().unwrap().title, "Recent Title");
}

#[test]
fn library_pane_renders_watched_episodes() {
    let state = loaded_state();
    let frame = render(&state, 130, 26);
    assert!(
        frame.contains("12/12 ep"),
        "watched episodes rendered in library:\n{frame}"
    );
}

#[test]
fn poster_renders_watched_episode_count_and_cover_art() {
    let mut state = loaded_state();
    // Select Yuru Camp (at index 1 after alphabetical sort)
    state.selected = 1;
    let art = anirel::art::CoverArt {
        width: 10,
        height: 15,
        pixels: vec![200; 10 * 15 * 3],
    };
    state.set_cover(1, Some(art));
    let frame = render(&state, 140, 28);

    assert!(
        frame.contains("12/12 ep watched"),
        "watched episodes in poster:\n{frame}"
    );
    assert!(
        frame.contains('▀'),
        "half-block cover art rendered in poster:\n{frame}"
    );
}

#[test]
fn poster_registers_kitty_cover_placement_when_graphics_supported() {
    unsafe {
        std::env::set_var("ANIREL_GRAPHICS", "kitty");
    }
    let mut state = loaded_state();
    state.selected = 1;
    let art = anirel::art::CoverArt {
        width: 10,
        height: 15,
        pixels: vec![200; 10 * 15 * 3],
    };
    state.set_cover(1, Some(art));
    let _ = render(&state, 140, 28);
    let placement = state.active_cover.get();
    assert!(placement.is_some(), "active cover placement recorded");
    let p = placement.unwrap();
    assert_eq!(p.mal_id, 1);
    assert!(p.cols > 0 && p.rows > 0);
    unsafe {
        std::env::remove_var("ANIREL_GRAPHICS");
    }
}
