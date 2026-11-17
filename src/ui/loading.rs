//! Initial library loading overlay.

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::app::state::{AppState, InitialLoading};
use crate::ui::theme::Theme;

pub fn render_loading(f: &mut Frame, area: Rect, state: &AppState) {
    let Some(loading) = &state.initial_loading else {
        return;
    };

    let width = 60.min(area.width.saturating_sub(4));
    let height = 9.min(area.height.saturating_sub(2));
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    f.render_widget(Clear, popup);

    let is_error = loading.error.is_some();
    let border_style = if is_error {
        Theme::missing()
    } else {
        Theme::accent()
    };
    let title = if is_error {
        " ❌ Sync Failed "
    } else {
        " 🌸 Loading Library "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Span::styled(title, Theme::bold(border_style)))
        .style(Style::default().bg(ratatui::style::Color::Reset));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let lines = if loading.error.is_some() {
        error_lines(loading)
    } else {
        progress_lines(loading, inner.width)
    };

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}

fn error_lines(loading: &InitialLoading) -> Vec<Line<'static>> {
    let err = loading.error.clone().unwrap_or_default();
    vec![
        Line::from(vec![
            Span::styled("Could not load library for ", Theme::base()),
            Span::styled(format!("@{}", loading.user), Theme::bold(Theme::accent())),
            Span::styled(":", Theme::base()),
        ]),
        Line::default(),
        Line::from(Span::styled(err, Theme::missing())),
        Line::default(),
        Line::from(Span::styled("press q or esc to quit", Theme::muted())),
    ]
}

fn progress_lines(loading: &InitialLoading, inner_width: u16) -> Vec<Line<'static>> {
    let bar_width = (inner_width as usize).saturating_sub(24).clamp(8, 28);
    let filled_steps = (loading.count / 300).min(bar_width);
    let bar = format!(
        "{}{}",
        "▰".repeat(filled_steps),
        "▱".repeat(bar_width.saturating_sub(filled_steps))
    );

    vec![
        Line::from(vec![
            Span::styled("Fetching list for ", Theme::muted()),
            Span::styled(format!("@{}", loading.user), Theme::bold(Theme::accent())),
            Span::styled(" from MyAnimeList…", Theme::muted()),
        ]),
        Line::default(),
        Line::from(vec![
            Span::styled(format!("  [{bar}] "), Theme::accent()),
            Span::styled(
                format!("{} titles fetched", loading.count),
                Theme::bold(Theme::base()),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "  caching for instant future launches · q to cancel",
            Theme::muted(),
        )),
    ]
}
