//! The poster card: cover artwork, title, metadata and the completion bar.

use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::{AppState, Focus};
use crate::domain::model::Anime;
use crate::ui::cover::{compute_cover_dimensions, render_cover, render_placeholder};
use crate::ui::theme::Theme;
use crate::ui::widgets::{fit, panel};

/// Render the poster card of the selected title.
pub fn render_poster(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::Detail;
    let block = panel(" POSTER ", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let Some(anime) = state.focus_target().or_else(|| state.current_anime()) else {
        f.render_widget(
            Paragraph::new("Nothing selected")
                .style(Theme::muted())
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    };

    let mut y = inner.y;
    let max_y = inner.y + inner.height;

    y = render_titles(f, inner, y, max_y, anime);
    y = render_cover_section(f, inner, y, max_y, anime, state);
    y = render_facts(f, inner, y, max_y, anime);
    y = render_watch_progress(f, inner, y, max_y, anime, state);
    render_franchise_summary(f, inner, y, max_y, state);
}

fn render_titles(f: &mut Frame, inner: Rect, mut y: u16, max_y: u16, anime: &Anime) -> u16 {
    if y < max_y {
        let title_romaji = fit(anime.romaji_title(), inner.width as usize);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                title_romaji,
                Theme::bold(Theme::accent()),
            ))),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }

    y = render_alt_title(f, inner, y, max_y, "EN  ", anime.english_title());
    y = render_alt_title(f, inner, y, max_y, "JA  ", anime.japanese_title());

    if y < max_y {
        y += 1;
    }
    y
}

fn render_alt_title(
    f: &mut Frame,
    inner: Rect,
    mut y: u16,
    max_y: u16,
    tag: &'static str,
    title: Option<&str>,
) -> u16 {
    if let Some(text) = title
        && y < max_y
    {
        let fitted = fit(text, inner.width.saturating_sub(4) as usize);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(tag, Theme::muted()),
                Span::styled(fitted, Theme::base()),
            ])),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }
    y
}

fn render_cover_section(
    f: &mut Frame,
    inner: Rect,
    mut y: u16,
    max_y: u16,
    anime: &Anime,
    state: &AppState,
) -> u16 {
    let space_left = max_y.saturating_sub(y);
    if space_left < 10 {
        return y;
    }

    let cover_rows = (space_left.saturating_sub(6)).min(13);
    let cover_area = Rect {
        x: inner.x,
        y,
        width: inner.width,
        height: cover_rows,
    };

    if let Some(art) = state.cover_for(anime.mal_id) {
        render_cover(f, cover_area, art);
        let (_, actual_rows) = compute_cover_dimensions(cover_area);
        y += actual_rows as u16 + 1;
    } else if state.is_fetching_cover(anime.mal_id) {
        render_placeholder(f, cover_area);
        y += 2;
    }
    y
}

fn render_facts(f: &mut Frame, inner: Rect, y: u16, max_y: u16, anime: &Anime) -> u16 {
    if y >= max_y {
        return y;
    }
    let mut facts = vec![
        anime.format.clone(),
        anime
            .episodes
            .map(|e| format!("{e} ep"))
            .unwrap_or_else(|| "?".into()),
    ];
    if let Some(dur) = anime.format_total_duration() {
        facts.push(dur);
    }
    facts.push(
        anime
            .year
            .map(|y| y.to_string())
            .unwrap_or_else(|| "?".into()),
    );
    if !anime.is_aired() {
        facts.push("not aired".into());
    }
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(facts.join(" · "), Theme::muted()))),
        Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        },
    );
    y + 1
}

fn render_watch_progress(
    f: &mut Frame,
    inner: Rect,
    mut y: u16,
    max_y: u16,
    anime: &Anime,
    state: &AppState,
) -> u16 {
    if y >= max_y {
        return y;
    }
    if let Some(entry) = state.library_entry(anime.mal_id) {
        let (glyph, glyph_style) = crate::ui::panes::status_glyph(entry.status);
        let ep_prog = entry.format_progress();
        let mut spans = vec![
            Span::styled(format!("{glyph} "), glyph_style),
            Span::styled(entry.status.label(), glyph_style),
            Span::styled(" · ", Theme::muted()),
            Span::styled(format!("{ep_prog} watched"), Theme::base()),
        ];
        if entry.score > 0 {
            spans.push(Span::styled(" · ", Theme::muted()));
            spans.push(Span::styled(
                format!("score {}", entry.score),
                Theme::warn(),
            ));
        }
        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
        y = render_mini_progress_bar(f, inner, y, max_y, anime, entry, glyph_style);
    } else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "◌ not in your library",
                Theme::missing(),
            ))),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }

    if y < max_y {
        y += 1;
    }
    y
}

fn render_mini_progress_bar(
    f: &mut Frame,
    inner: Rect,
    mut y: u16,
    max_y: u16,
    anime: &Anime,
    entry: &crate::domain::model::LibraryEntry,
    glyph_style: Style,
) -> u16 {
    if let Some(total) = anime.episodes
        && total > 0
        && y < max_y
    {
        let bar_width = (inner.width as usize).saturating_sub(6).clamp(4, 24);
        let watched = entry.episodes_watched.min(total) as usize;
        let filled = ((watched as f64 / total as f64) * bar_width as f64).round() as usize;
        let bar = format!(
            "{}{}",
            "▰".repeat(filled),
            "▱".repeat(bar_width.saturating_sub(filled)),
        );
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(bar, glyph_style))),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }
    y
}

fn render_franchise_summary(f: &mut Frame, inner: Rect, mut y: u16, max_y: u16, state: &AppState) {
    let Some(cluster) = state.detail_cluster() else {
        return;
    };
    if y >= max_y {
        return;
    }

    let owned = cluster.owned_count();
    let total = cluster.total_count().max(1);
    let bar_width = (inner.width as usize).saturating_sub(10).clamp(4, 40);
    let filled = ((owned as f64 / total as f64) * bar_width as f64).round() as usize;
    let bar = format!(
        "{}{}  {owned}/{total}",
        "▰".repeat(filled),
        "▱".repeat(bar_width.saturating_sub(filled)),
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(bar, Theme::accent()))),
        Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        },
    );
    y += 1;
    render_franchise_metrics(f, inner, y, max_y, cluster);
}

fn render_franchise_metrics(
    f: &mut Frame,
    inner: Rect,
    mut y: u16,
    max_y: u16,
    cluster: &crate::domain::cluster::Cluster,
) {
    if y < max_y {
        let summary = format!(
            "{} missing · {} unwatched",
            cluster.missing_count(),
            cluster.members.iter().filter(|m| m.is_unwatched()).count()
        );
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(summary, Theme::missing()))),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
        y += 1;
    }

    if y < max_y {
        render_cluster_duration(f, inner, y, cluster);
        y += 1;
    }

    if let Some(score) = cluster.best_score
        && y < max_y
    {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("your score {score}"),
                Theme::warn(),
            ))),
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: 1,
            },
        );
    }
}

fn render_cluster_duration(
    f: &mut Frame,
    inner: Rect,
    y: u16,
    cluster: &crate::domain::cluster::Cluster,
) {
    let dur_summary = format!(
        "{} total · {} watched · {} left",
        crate::domain::model::format_duration(cluster.total_duration_minutes()),
        crate::domain::model::format_duration(cluster.watched_duration_minutes()),
        crate::domain::model::format_duration(cluster.left_duration_minutes()),
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(dur_summary, Theme::accent()))),
        Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        },
    );
}

/// Width of the poster pane for a given terminal width.
pub fn poster_width(width: u16) -> u16 {
    match width {
        w if w >= 150 => 40,
        w if w >= 120 => 34,
        w if w >= 100 => 30,
        w if w >= 80 => 26,
        _ => 0,
    }
}

/// True when the poster pane fits.
pub fn fits(width: u16) -> bool {
    poster_width(width) > 0
}

pub use crate::ui::widgets::{title_width, wrap_title};
