//! Shared rendering helpers.

use ratatui::prelude::*;
use ratatui::widgets::{Block, BorderType, Borders};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::ui::theme::Theme;

pub fn panel(title: &str, focused: bool) -> Block<'static> {
    let border_style = if focused {
        Theme::accent()
    } else {
        Theme::muted()
    };
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Span::styled(
            title.to_string(),
            if focused {
                Theme::bold(Theme::accent())
            } else {
                Theme::muted()
            },
        ))
        .title_alignment(Alignment::Left)
        .padding(ratatui::widgets::Padding::horizontal(1))
}

pub fn fit(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    if text.width() <= width {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let w = ch.width().unwrap_or(0);
        if used + w > width.saturating_sub(1) {
            break;
        }
        out.push(ch);
        used += w;
    }
    out.push('…');
    out
}

pub fn fit_title(title: &str, inner_width: u16, meta_width: usize) -> String {
    let reserved = 4 + meta_width + 2; // marker, glyph, spacing
    let available = (inner_width as usize).saturating_sub(reserved).max(4);
    fit(title, available)
}
