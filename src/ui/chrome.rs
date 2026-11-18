//! Header, view tabs and the key-hint footer.

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::app::state::{AppState, View};
use crate::ui::theme::Theme;
pub fn render_header(f: &mut Frame, area: Rect, state: &AppState) {
    let left = Line::from(vec![
        Span::styled(" anirel ", Theme::bold(Theme::accent())),
        Span::styled("·", Theme::muted()),
        Span::styled(format!(" {}", state.user), Theme::base()),
    ]);

    let (right, right_style) = if let Some(error) = &state.scan.error {
        (
            Line::from(vec![
                Span::styled("✗ scan failed", Theme::missing()),
                Span::styled(format!(" {error}"), Theme::muted()),
            ]),
            Theme::missing(),
        )
    } else if state.scan.running {
        (
            Line::from(vec![
                Span::styled("⟳ scanning ", Theme::accent()),
                Span::styled(
                    format!(
                        "{}/{}",
                        state.scan.done,
                        state.scan.total.max(state.scan.done)
                    ),
                    Theme::base(),
                ),
            ]),
            Theme::accent(),
        )
    } else {
        (
            Line::from(vec![
                Span::styled("● ", Theme::ok()),
                Span::styled(format!("{} titles", state.library.len()), Theme::muted()),
            ]),
            Theme::ok(),
        )
    };

    let gap = area
        .width
        .saturating_sub(left.width() as u16)
        .saturating_sub(right.width() as u16);
    let mut spans = left.spans;
    spans.push(Span::raw(" ".repeat(gap as usize)));
    spans.extend(right.spans);

    f.render_widget(Paragraph::new(Line::from(spans)), area);
    let _ = right_style;
}

pub fn render_tabs(f: &mut Frame, area: Rect, state: &AppState) {
    let mut spans: Vec<Span> = Vec::new();
    let tabs = [
        (View::Library, state.library.len()),
        (View::Missing, state.total_gaps()),
    ];

    for (index, (view, count)) in tabs.iter().enumerate() {
        let active = state.view == *view;
        let style = if active {
            Theme::bold(Theme::accent())
        } else {
            Theme::muted()
        };
        if index > 0 {
            spans.push(Span::styled("   ", Theme::muted()));
        }
        spans.push(Span::styled(
            if active {
                format!("▸ {} ", view.label())
            } else {
                format!("  {} ", view.label())
            },
            style,
        ));
        spans.push(Span::styled(format!("{count}"), style));
    }

    let filter_text = if state.search_mode {
        format!("⌕ {}_", state.filter)
    } else if state.filter.is_empty() {
        "⌕ press / to filter".to_string()
    } else {
        format!("⌕ {}", state.filter)
    };
    let filter_style = if state.search_mode {
        Theme::accent()
    } else if state.filter.is_empty() {
        Theme::muted()
    } else {
        Theme::base()
    };

    let sort_label = format!("⇅ {}", state.sort.label());
    let right_parts = Line::from(vec![
        Span::styled(sort_label, Theme::muted()),
        Span::raw("   "),
        Span::styled(filter_text, filter_style),
    ]);
    let gap = area
        .width
        .saturating_sub(spans.iter().map(|s| s.width()).sum::<usize>() as u16)
        .saturating_sub(right_parts.width() as u16);

    let mut all = spans;
    all.push(Span::raw(" ".repeat(gap as usize)));
    all.extend(right_parts.spans);
    f.render_widget(Paragraph::new(Line::from(all)), area);
}

pub fn render_footer(f: &mut Frame, area: Rect, state: &AppState) {
    let mut spans: Vec<Span> = vec![Span::styled(" ", Theme::muted())];

    if let Some(message) = &state.message {
        let style = match message.kind {
            crate::app::state::MessageKind::Info => Theme::accent(),
            crate::app::state::MessageKind::Success => Theme::ok(),
            crate::app::state::MessageKind::Error => Theme::missing(),
        };
        spans.push(Span::styled(message.text.clone(), style));
        spans.push(Span::styled("   ·   ", Theme::muted()));
    }

    let full: &[(&str, &str)] = &[
        ("tab", "focus"),
        ("/", "filter"),
        ("s", "sort"),
        ("L", "load"),
        ("F", "refresh list"),
        ("o", "open"),
        ("c", "copy"),
        ("a", "scan all"),
        ("e", "export"),
        ("?", "help"),
        ("q", "quit"),
    ];
    let short: &[(&str, &str)] = &[
        ("/", "filter"),
        ("s", "sort"),
        ("L", "load"),
        ("o", "open"),
        ("?", "help"),
        ("q", "quit"),
    ];
    let hints = if area.width >= 104 { full } else { short };
    for (key, label) in hints {
        spans.push(Span::styled(
            format!(" {key} "),
            Theme::bold(Theme::accent()),
        ));
        spans.push(Span::styled(format!("{label}  "), Theme::muted()));
    }

    // Sort indicator right-aligned.
    let sort_indicator = format!("⇅ {}  ", state.sort.label());
    let used: usize = spans.iter().map(|s| s.width()).sum();
    let sort_w = sort_indicator.len();
    let gap = area.width as usize;
    if used + sort_w < gap {
        let padding = gap - used - sort_w;
        spans.push(Span::raw(" ".repeat(padding)));
    }
    spans.push(Span::styled(sort_indicator, Theme::muted()));

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}
