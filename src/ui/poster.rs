//! The poster card: artwork, metadata and the completion bar.

use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::app::state::{AppState, Focus};
use crate::art::{self, CoverArt};
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

    // Artwork.
    let art_height = (inner.height / 3).clamp(6, 18);
    if let Some(cover) = state.covers.get(&anime.mal_id) {
        draw_cover(
            f,
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: art_height,
            },
            cover,
        );
    } else {
        draw_placeholder(
            f,
            Rect {
                x: inner.x,
                y,
                width: inner.width,
                height: art_height,
            },
            anime.display_title(),
        );
    }
    y += art_height + 1;

    // Title.
    let title = fit(anime.display_title(), inner.width as usize);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title,
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

/// Draw the cover as half blocks: upper pixel in the foreground, lower in the background.
fn draw_cover(f: &mut Frame, area: Rect, cover: &CoverArt) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let pixel_width = area.width as usize;
    let pixel_height = (area.height as usize) * 2;
    let pixels = cover.resample(pixel_width, pixel_height);

    let mut lines: Vec<Line> = Vec::with_capacity(area.height as usize);
    for row in 0..area.height as usize {
        let mut spans: Vec<Span> = Vec::with_capacity(area.width as usize);
        for column in 0..area.width as usize {
            let top = ((row * 2) * pixel_width + column) * 3;
            let bottom = ((row * 2 + 1) * pixel_width + column) * 3;
            if top + 2 >= pixels.len() || bottom + 2 >= pixels.len() {
                spans.push(Span::raw(" "));
                continue;
            }
            spans.push(Span::styled(
                "▀",
                Style::default()
                    .fg(art::color(pixels[top], pixels[top + 1], pixels[top + 2]))
                    .bg(art::color(
                        pixels[bottom],
                        pixels[bottom + 1],
                        pixels[bottom + 2],
                    )),
            ));
        }
        lines.push(Line::from(spans));
    }
    f.render_widget(Paragraph::new(lines), area);
}

/// A stylised placeholder for titles whose cover is not loaded yet.
fn draw_placeholder(f: &mut Frame, area: Rect, title: &str) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let initial = title.chars().find(|c| c.is_alphanumeric()).unwrap_or('?');
    let mut lines: Vec<Line> = Vec::new();

    let middle = area.height as usize / 2;
    for row in 0..area.height as usize {
        if row == middle {
            let padding = (area.width as usize).saturating_sub(3) / 2;
            lines.push(Line::from(vec![
                Span::raw(" ".repeat(padding)),
                Span::styled(
                    format!(" {initial} "),
                    Theme::bold(Theme::accent()).bg(ratatui::style::Color::DarkGray),
                ),
            ]));
        } else {
            let pattern: String = (0..area.width as usize)
                .map(|column| {
                    if (row + column) % 2 == 0 {
                        '▚'
                    } else {
                        '▞'
                    }
                })
                .collect();
            lines.push(Line::from(Span::styled(
                pattern,
                ratatui::style::Style::default().fg(ratatui::style::Color::Rgb(28, 32, 38)),
            )));
        }
    }
    f.render_widget(Paragraph::new(lines), area);
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
