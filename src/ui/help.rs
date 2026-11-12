//! The keyboard help overlay.

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};

use crate::ui::theme::Theme;

pub fn render_help(f: &mut Frame, area: Rect) {
    let width = 64.min(area.width.saturating_sub(4));
    let height = 24.min(area.height.saturating_sub(2));
    let popup = Rect {
        x: area.x + (area.width.saturating_sub(width)) / 2,
        y: area.y + (area.height.saturating_sub(height)) / 2,
        width,
        height,
    };

    f.render_widget(Clear, popup);

    let rows = [
        ("↑ ↓ / j k", "move selection"),
        ("tab / shift+tab", "switch focus between panes"),
        ("/", "filter (esc to clear)"),
        ("1 / 2", "franchises / missing"),
        ("o / enter", "open on MyAnimeList"),
        ("c", "copy link"),
        ("e", "export report"),
        ("r", "rescan library"),
        ("?", "close this help"),
        ("q / ctrl+c", "quit"),
    ];

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(" Keyboard", Theme::bold(Theme::accent()))),
        Line::default(),
    ];
    for (key, description) in rows {
        lines.push(Line::from(vec![
            Span::styled(format!("  {key:<16}"), Theme::bold(Theme::base())),
            Span::styled(description.to_string(), Theme::muted()),
        ]));
    }
    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "  press ? or esc to close",
        Theme::muted(),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Theme::muted())
        .style(Style::default().bg(ratatui::style::Color::Reset));
    let inner = block.inner(popup);
    f.render_widget(block, popup);
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), inner);
}
