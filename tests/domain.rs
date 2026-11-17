//! Tests for the relation graph, franchise clustering and watch order.

use anirel::domain::cluster::{RelationGraph, build_clusters};
use anirel::domain::model::{Anime, Library, LibraryEntry, LibraryStatus};
use anirel::domain::relation::RelationKind;

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

fn entry(mal_id: i64, title: &str, year: i32) -> LibraryEntry {
    LibraryEntry {
        anime: anime(mal_id, title, year, "TV"),
        status: LibraryStatus::Completed,
        score: 10,
        episodes_watched: 12,
    }
}

/// The Yuru Camp shape: a season chain with side stories and a film.
fn camp_graph() -> RelationGraph {
    let mut graph = RelationGraph::new();
    for (id, title, year, format) in [
        (1, "Camp", 2018, "TV"),
        (2, "Camp Season 2", 2021, "TV"),
        (3, "Camp Season 3", 2024, "TV"),
        (4, "Camp Movie", 2022, "MOVIE"),
        (5, "Camp OVA", 2018, "OVA"),
        (6, "Room Camp", 2020, "TV_SHORT"),
    ] {
        graph.insert_media(anime(id, title, year, format));
    }

    graph.add_edge(1, 2, RelationKind::Sequel);
    graph.add_edge(2, 3, RelationKind::Sequel);
    graph.add_edge(2, 4, RelationKind::Sequel);
    graph.add_edge(1, 5, RelationKind::SideStory);
    graph.add_edge(1, 6, RelationKind::SideStory);
    graph.add_edge(6, 1, RelationKind::Parent);
    // The OVA is the prequel of the spin-off, as MAL and AniList have it.
    graph.add_edge(6, 5, RelationKind::Prequel);
    graph
}

#[test]
fn franchise_contains_every_related_title() {
    let graph = camp_graph();
    let library = Library::new(vec![
        entry(1, "Camp", 2018),
        entry(2, "Camp Season 2", 2021),
    ]);
    let clusters = build_clusters(&graph, &library);

    assert_eq!(clusters.len(), 1);
    let cluster = &clusters[0];
    assert_eq!(cluster.total_count(), 6);
    assert_eq!(cluster.owned_count(), 2);
    assert_eq!(cluster.missing_count(), 4);
}

#[test]
fn watch_order_follows_the_release_chain() {
    let graph = camp_graph();
    let library = Library::new(vec![entry(1, "Camp", 2018)]);
    let clusters = build_clusters(&graph, &library);
    let order: Vec<i64> = clusters[0].members.iter().map(|m| m.anime.mal_id).collect();

    // The first season leads, the film follows season 2, season 3 is last.
    assert_eq!(order[0], 1, "the first season comes first");
    let season2 = order.iter().position(|id| *id == 2).unwrap();
    let season3 = order.iter().position(|id| *id == 3).unwrap();
    let movie = order.iter().position(|id| *id == 4).unwrap();
    assert!(season2 < movie, "the film comes after season 2");
    assert!(season2 < season3, "season 3 comes after season 2");
}

#[test]
fn relations_are_described_against_the_franchise_root() {
    let graph = camp_graph();
    let library = Library::new(vec![entry(1, "Camp", 2018)]);
    let clusters = build_clusters(&graph, &library);
    let cluster = &clusters[0];

    let relation_of = |mal_id: i64| {
        cluster
            .members
            .iter()
            .find(|m| m.anime.mal_id == mal_id)
            .map(|m| m.relation)
    };

    assert_eq!(relation_of(2), Some(RelationKind::Sequel));
    assert_eq!(relation_of(5), Some(RelationKind::SideStory));
    // "Room Camp" declares the first season as its parent story.
    assert_eq!(relation_of(6), Some(RelationKind::SideStory));
}

#[test]
fn franchises_without_library_titles_are_dropped() {
    let graph = camp_graph();
    let library = Library::new(vec![entry(99, "Something Else", 2020)]);
    assert!(build_clusters(&graph, &library).is_empty());
}

#[test]
fn library_lookup_and_best_score() {
    let library = Library::new(vec![
        entry(1, "Camp", 2018),
        entry(2, "Camp Season 2", 2021),
    ]);
    assert!(library.contains(1));
    assert!(!library.contains(42));
    assert_eq!(library.best_score_for(&[1, 2]), Some(10));
    assert_eq!(library.best_score_for(&[404]), None);
}

#[test]
fn relation_kinds_parse_from_both_spellings() {
    assert_eq!(RelationKind::parse("SIDE_STORY"), RelationKind::SideStory);
    assert_eq!(RelationKind::parse("Side Story"), RelationKind::SideStory);
    assert_eq!(RelationKind::parse("Parent Story"), RelationKind::Parent);
    assert_eq!(RelationKind::parse("spin-off"), RelationKind::SpinOff);
    assert_eq!(
        RelationKind::parse("Character(s) in"),
        RelationKind::Character
    );
    assert!(RelationKind::SideStory.is_franchise());
    assert!(!RelationKind::Adaptation.is_franchise());
}

#[test]
fn related_to_is_rooted_at_the_seed() {
    use anirel::domain::cluster::related_to;

    let graph = camp_graph();
    let library = Library::new(vec![entry(1, "Camp", 2018), entry(6, "Room Camp", 2020)]);

    let cluster = related_to(&graph, &library, 1, 2).expect("a cluster");
    assert!(
        cluster.total_count() >= 5,
        "the whole franchise is reachable: {:?}",
        cluster
            .members
            .iter()
            .map(|m| m.anime.mal_id)
            .collect::<Vec<_>>()
    );

    // Diving into a side story shows that side story's own neighbourhood.
    let room = related_to(&graph, &library, 6, 1).expect("a cluster");
    let ids: Vec<i64> = room.members.iter().map(|m| m.anime.mal_id).collect();
    assert!(
        ids.contains(&6),
        "the seed is in its own franchise: {ids:?}"
    );
    assert!(ids.contains(&5), "the prequel is included: {ids:?}");
    assert!(!ids.contains(&2), "the other season is not: {ids:?}");
}

#[test]
fn anime_title_prefers_romaji_and_exposes_english_and_japanese() {
    let mut item = anime(34798, "Yuru Camp△", 2018, "TV");
    item.title_english = Some("Laid-Back Camp".into());
    item.title_japanese = Some("ゆるキャン△".into());

    assert_eq!(item.display_title(), "Yuru Camp△");
    assert_eq!(item.romaji_title(), "Yuru Camp△");
    assert_eq!(item.english_title(), Some("Laid-Back Camp"));
    assert_eq!(item.japanese_title(), Some("ゆるキャン△"));
}

#[test]
fn duration_calculations_and_formatting() {
    use anirel::domain::model::format_duration;

    assert_eq!(format_duration(0), "0m");
    assert_eq!(format_duration(24), "24m");
    assert_eq!(format_duration(60), "1h");
    assert_eq!(format_duration(120), "2h");
    assert_eq!(format_duration(140), "2h 20m");
    assert_eq!(format_duration(288), "4h 48m");

    let mut tv = anime(1, "TV Show", 2020, "TV");
    tv.episodes = Some(12);
    tv.duration = Some(24);
    assert_eq!(tv.total_duration_minutes(), Some(288));
    assert_eq!(tv.format_total_duration(), Some("4h 48m".into()));

    let mut movie = anime(2, "Movie", 2021, "MOVIE");
    movie.episodes = Some(1);
    movie.duration = None; // falls back to 100m for movie
    assert_eq!(movie.total_duration_minutes(), Some(100));
    assert_eq!(movie.format_total_duration(), Some("1h 40m".into()));
}

#[test]
fn franchise_total_watched_and_left_durations() {
    let graph = camp_graph();
    let library = Library::new(vec![
        // Completed 12 ep of 12 ep (288m)
        entry(1, "Camp", 2018),
        // Watching 6 ep of 12 ep (144m watched, 144m left)
        LibraryEntry {
            anime: anime(2, "Camp Season 2", 2021, "TV"),
            status: LibraryStatus::Watching,
            score: 9,
            episodes_watched: 6,
        },
    ]);

    let clusters = build_clusters(&graph, &library);
    let cluster = clusters
        .into_iter()
        .find(|c| c.members.iter().any(|m| m.anime.mal_id == 1))
        .expect("camp cluster");

    let total = cluster.total_duration_minutes();
    let watched = cluster.watched_duration_minutes();
    let left = cluster.left_duration_minutes();

    assert!(total > 0, "total duration is positive");
    assert!(watched > 0, "watched duration is positive");
    assert_eq!(watched, 288 + 144);
    assert_eq!(left, total - watched);
}
