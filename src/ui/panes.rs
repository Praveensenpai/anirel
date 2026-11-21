//! The main list pane: your library, or the gaps of the loaded franchises.

use ratatui::prelude::*;
use ratatui::widgets::{Gauge, List, ListItem, ListState, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::app::state::{AppState, Focus, Row, View};
use crate::domain::model::LibraryStatus;
pub use crate::ui::detail::render_detail;
use crate::ui::theme::Theme;
use crate::ui::widgets::{fit, panel};

/// Render the main list: your library, or the gaps of the loaded franchises.
pub fn render_list(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::List;
    let title = match state.view {
        View::Library => " MY LIBRARY ",
        View::Missing => " MISSING RELATED ",
    };
    let block = panel(title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.rows().is_empty() {
        render_empty_list(f, inner, state);
        return;
    }

    let items: Vec<ListItem> = state
        .rows()
        .iter()
        .enumerate()
        .map(|(index, row)| {
            ListItem::new(list_line(
                row,
                index == state.selected,
                focused,
                state,
                inner.width,
            ))
        })
        .collect();

    let mut list_state = ListState::default().with_selected(Some(state.selected));
    f.render_stateful_widget(
        List::new(items).highlight_style(Style::default()),
        inner,
        &mut list_state,
    );

    if state.scan.running && inner.height > 2 {
        render_scan_gauge(f, inner, state);
    }
}

fn render_empty_list(f: &mut Frame, area: Rect, state: &AppState) {
    let empty = match state.view {
        View::Library if !state.filter.is_empty() => {
            format!("Nothing matches \"{}\"", state.filter)
        }
        View::Library => "Your library is empty.".to_string(),
        View::Missing if !state.filter.is_empty() => {
            format!("Nothing missing matching \"{}\"", state.filter)
        }
        View::Missing => {
            "No franchise loaded yet.\n\nSelect a title and press L to load its franchise.".into()
        }
    };
    f.render_widget(
        Paragraph::new(empty)
            .style(Theme::muted())
            .wrap(Wrap { trim: true }),
        area,
    );
}

fn render_scan_gauge(f: &mut Frame, inner: Rect, state: &AppState) {
    let gauge_area = Rect {
        x: inner.x,
        y: inner.y + inner.height - 1,
        width: inner.width,
        height: 1,
    };
    let ratio = if state.scan.total == 0 {
        0.0
    } else {
        (state.scan.done as f64 / state.scan.total as f64).clamp(0.0, 1.0)
    };
    f.render_widget(
        Gauge::default()
            .ratio(ratio)
            .gauge_style(Theme::accent())
            .label(Span::styled(
                format!(" scanning {}/{} ", state.scan.done, state.scan.total),
                Theme::base(),
            )),
        gauge_area,
    );
}

fn list_line(
    row: &Row,
    selected: bool,
    focused: bool,
    state: &AppState,
    inner_width: u16,
) -> Line<'static> {
    let marker_style = if selected {
        if focused {
            Theme::bold(Theme::accent())
        } else {
            Theme::muted()
        }
    } else {
        Style::default()
    };
    let marker = if selected { "▌ " } else { "  " };

    match row {
        Row::Library(index) => {
            library_line(*index, marker, marker_style, selected, state, inner_width)
        }
        Row::Gap { cluster, member } => gap_line(
            *cluster,
            *member,
            marker,
            marker_style,
            selected,
            state,
            inner_width,
        ),
    }
}

fn library_line(
    index: usize,
    marker: &'static str,
    marker_style: Style,
    selected: bool,
    state: &AppState,
    inner_width: u16,
) -> Line<'static> {
    let Some(entry) = state.library.entries.get(index) else {
        return Line::default();
    };
    let (glyph, glyph_style) = status_glyph(entry.status);
    let ep_prog = entry.format_progress();
    let score = if entry.score > 0 {
        format!(" {}", entry.score)
    } else {
        String::new()
    };
    let tail = format!("{ep_prog}  {glyph}{score}");
    let title = fit(
        entry.anime.display_title(),
        (inner_width as usize).saturating_sub(6 + tail.width()),
    );
    let title_style = if selected {
        Theme::bold(Theme::base())
    } else {
        Theme::base()
    };
    Line::from(vec![
        Span::styled(marker, marker_style),
        Span::styled(title, title_style),
        Span::raw("  "),
        Span::styled(ep_prog, Theme::muted()),
        Span::raw(" "),
        Span::styled(format!("{glyph}{score}"), glyph_style),
    ])
}

fn gap_line(
    cluster_idx: usize,
    member_idx: usize,
    marker: &'static str,
    marker_style: Style,
    selected: bool,
    state: &AppState,
    inner_width: u16,
) -> Line<'static> {
    let (Some(_cluster), Some(member)) = (
        state.clusters.get(cluster_idx),
        state
            .clusters
            .get(cluster_idx)
            .and_then(|c| c.members.get(member_idx)),
    ) else {
        return Line::default();
    };
    let (glyph, glyph_style) = match &member.entry {
        None => ("◌", Theme::missing()),
        Some(entry) => status_glyph(entry.status),
    };
    let title = fit(
        member.anime.display_title(),
        (inner_width as usize).saturating_sub(16),
    );
    let title_style = if selected {
        Theme::bold(Theme::base())
    } else {
        Theme::base()
    };
    Line::from(vec![
        Span::styled(marker, marker_style),
        Span::styled(glyph.to_string(), glyph_style),
        Span::styled(" ", Theme::base()),
        Span::styled(title, title_style),
        Span::styled("  ", Theme::muted()),
        Span::styled(member.relation.label().to_string(), Theme::muted()),
    ])
}

pub fn status_glyph(status: LibraryStatus) -> (&'static str, Style) {
    match status {
        LibraryStatus::Completed => ("✓", Theme::ok()),
        LibraryStatus::Watching => ("▶", Theme::warn()),
        LibraryStatus::PlanToWatch => ("○", Theme::warn()),
        LibraryStatus::OnHold => ("⏸", Theme::muted()),
        LibraryStatus::Dropped => ("✗", Theme::missing()),
        LibraryStatus::Other(_) => ("?", Theme::muted()),
    }
}
