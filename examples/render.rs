//! Render the TUI to a string so the layout can be inspected without a TTY.
//!
//! Usage: cargo run --example render -- [width] [height]

use anirel::app::state::AppState;
use anirel::config::Scope;
use anirel::domain::model::{Anime, Library, LibraryEntry, LibraryStatus};
use anirel::domain::relation::RelationKind;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

fn anime(mal_id: i64, title: &str, english: &str, year: i32, format: &str, eps: i64) -> Anime {
    Anime {
        mal_id,
        title: title.into(),
        title_english: Some(english.into()),
        title_japanese: None,
        format: format.into(),
        episodes: Some(eps),
        duration: None,
        airing_status: "Finished Airing".into(),
        year: Some(year),
        aired_from: Some((year, 1, 1)),
        score: Some(8.0),
        cover_url: None,
    }
}

fn with_ja(mut a: Anime, ja: &str) -> Anime {
    a.title_japanese = Some(ja.into());
    a
}

fn main() {
    let width: u16 = std::env::args()
        .nth(1)
        .and_then(|v| v.parse().ok())
        .unwrap_or(140);
    let height: u16 = std::env::args()
        .nth(2)
        .and_then(|v| v.parse().ok())
        .unwrap_or(30);

    let library = Library::new(vec![
        LibraryEntry {
            anime: with_ja(
                anime(34798, "Yuru Camp△", "Laid-Back Camp", 2018, "TV", 12),
                "ゆるキャン△",
            ),
            status: LibraryStatus::Completed,
            score: 10,
            episodes_watched: 12,
            updated_at: None,
        },
        LibraryEntry {
            anime: with_ja(
                anime(
                    38474,
                    "Yuru Camp△ Season 2",
                    "Laid-Back Camp Season 2",
                    2021,
                    "TV",
                    13,
                ),
                "ゆるキャン△ SEASON２",
            ),
            status: LibraryStatus::Completed,
            score: 10,
            episodes_watched: 13,
            updated_at: None,
        },
        LibraryEntry {
            anime: anime(58855, "Yuru Camp△ Season 3 Specials", "", 2024, "OVA", 3),
            status: LibraryStatus::PlanToWatch,
            score: 0,
            episodes_watched: 0,
            updated_at: None,
        },
        LibraryEntry {
            anime: with_ja(
                anime(38000, "Kimetsu no Yaiba", "Demon Slayer", 2018, "TV", 26),
                "鬼滅の刃",
            ),
            status: LibraryStatus::Watching,
            score: 9,
            episodes_watched: 13,
            updated_at: None,
        },
        LibraryEntry {
            anime: with_ja(
                anime(1535, "Death Note", "Death Note", 2006, "TV", 37),
                "デスノート",
            ),
            status: LibraryStatus::Completed,
            score: 10,
            episodes_watched: 37,
            updated_at: None,
        },
    ]);

    let mut state = AppState::new("Praveen20".into(), library, Scope::default());
    state.selected = 2;

    state.ingest(
        with_ja(
            anime(34798, "Yuru Camp△", "Laid-Back Camp", 2018, "TV", 12),
            "ゆるキャン△",
        ),
        vec![
            (38474, RelationKind::Sequel),
            (38475, RelationKind::SideStory),
            (37341, RelationKind::SideStory),
            (38476, RelationKind::SideStory),
            (41061, RelationKind::SideStory),
        ],
    );
    state.ingest(
        anime(
            38474,
            "Yuru Camp△ Season 2",
            "Laid-Back Camp Season 2",
            2021,
            "TV",
            13,
        ),
        vec![
            (38475, RelationKind::Sequel),
            (53410, RelationKind::Sequel),
            (49026, RelationKind::SideStory),
        ],
    );
    state.ingest(
        anime(
            53410,
            "Yuru Camp△ Season 3",
            "Laid-Back Camp Season 3",
            2024,
            "TV",
            12,
        ),
        vec![
            (58855, RelationKind::SideStory),
            (60267, RelationKind::Sequel),
        ],
    );
    state.ingest(
        anime(
            38475,
            "Yuru Camp△ Movie",
            "Laid-Back Camp: The Movie",
            2022,
            "MOVIE",
            1,
        ),
        vec![],
    );
    state.ingest(
        anime(
            37341,
            "Yuru Camp△ Specials",
            "Laid-Back Camp Specials",
            2018,
            "OVA",
            3,
        ),
        vec![],
    );
    state.ingest(
        anime(38476, "Heya Camp△", "Room Camp", 2020, "TV_SHORT", 12),
        vec![],
    );
    state.ingest(
        anime(
            41061,
            "Heya Camp△: Sauna to Gohan to Sanrin Bike",
            "",
            2020,
            "OVA",
            1,
        ),
        vec![],
    );
    state.ingest(
        anime(49026, "Yuru Camp△ Season 2 Specials", "", 2021, "OVA", 2),
        vec![],
    );
    state.ingest(
        anime(58855, "Yuru Camp△ Season 3 Specials", "", 2024, "OVA", 3),
        vec![],
    );
    state.ingest(
        anime(
            60267,
            "Yuru Camp△ Season 4",
            "Laid-Back Camp Season 4",
            2027,
            "TV",
            12,
        ),
        vec![],
    );
    state.rebuild();

    // Put the cursor on the Yuru Camp title so a franchise is shown.
    if let Some(index) = state.rows().iter().position(|row| {
        if let anirel::app::state::Row::Library(i) = row {
            state
                .library
                .entries
                .get(*i)
                .map(|e| e.anime.title.as_str())
                == Some("Yuru Camp△")
        } else {
            false
        }
    }) {
        state.selected = index;
    }

    let mut cover_pixels = Vec::with_capacity(30 * 45 * 3);
    for y in 0..45 {
        for x in 0..30 {
            cover_pixels.push((x * 7) as u8);
            cover_pixels.push((80 + y * 3) as u8);
            cover_pixels.push(200);
        }
    }
    state.set_cover(
        34798,
        Some(anirel::art::CoverArt {
            width: 30,
            height: 45,
            pixels: cover_pixels,
        }),
    );

    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| anirel::ui::draw(f, &state)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    for y in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        println!("{line}");
    }
}
