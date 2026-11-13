//! The poster card: title, metadata and the completion bar.

use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::app::state::{AppState, Focus};
use crate::ui::theme::Theme;
use crate::ui::widgets::{fit, panel};

/// Render the poster card of the selected title.
pub fn render_poster(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::Detail;
    let block = panel(" POSTER ", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // The poster follows whatever the keyboard acts on.
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

    // Romaji title (primary).
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

    // English title, if present.
    if let Some(english) = anime.english_title()
        && y < max_y
    {
        let title_eng = fit(english, inner.width.saturating_sub(4) as usize);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("EN  ", Theme::muted()),
                Span::styled(title_eng, Theme::base()),
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

    // Nihongo (Japanese) title, if present.
    if let Some(japanese) = anime.japanese_title()
        && y < max_y
    {
        let title_ja = fit(japanese, inner.width.saturating_sub(4) as usize);
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("JA  ", Theme::muted()),
                Span::styled(title_ja, Theme::base()),
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

    if y < max_y {
        y += 1;
    }

    // Facts.
    let mut facts = vec![anime.format.clone()];
    facts.push(match anime.episodes {
        Some(episodes) => format!("{episodes} ep"),
        None => "?".into(),
    });
    facts.push(
        anime
            .year
            .map(|year| year.to_string())
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
    y += 2;

    // Completion bar of the franchise.
    if let Some(cluster) = state.detail_cluster() {
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

        let summary = format!(
            "{} missing · {} unwatched",
            cluster.missing_count(),
            cluster
                .members
                .iter()
                .filter(|member| member.is_unwatched())
                .count()
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

        if let Some(score) = cluster.best_score {
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
}

/// Width of the poster pane for a given terminal width.
pub fn poster_width(terminal_width: u16) -> u16 {
    match terminal_width {
        w if w >= 150 => 40,
        w if w >= 120 => 34,
        w if w >= 100 => 30,
        w if w >= 80 => 26,
        _ => 0,
    }
}

/// True when the poster pane fits.
pub fn fits(terminal_width: u16) -> bool {
    poster_width(terminal_width) > 0
}

/// Longest title that fits the card, for layout maths.
pub fn title_width(area: Rect) -> usize {
    area.width.saturating_sub(2) as usize
}

/// Wrap a title into at most `max_lines` lines.
pub fn wrap_title(title: &str, width: usize, max_lines: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in title.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if candidate.width() > width && !current.is_empty() {
            lines.push(std::mem::take(&mut current));
            if lines.len() == max_lines {
                break;
            }
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    lines
}
