//! The two main panes: your library and the detail view of a loaded franchise.

use ratatui::prelude::*;
use ratatui::widgets::{Gauge, List, ListItem, ListState, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::app::state::{AppState, Focus, Row, View};
use crate::domain::model::LibraryStatus;
use crate::ui::theme::Theme;
use crate::ui::widgets::{fit, fit_title, panel};

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
        let empty = match state.view {
            View::Library if !state.filter.is_empty() => {
                format!("Nothing matches \"{}\"", state.filter)
            }
            View::Library => "Your library is empty.".to_string(),
            View::Missing if !state.filter.is_empty() => {
                format!("Nothing missing matching \"{}\"", state.filter)
            }
            View::Missing => {
                "No franchise loaded yet.\n\nSelect a title and press L to load its franchise."
                    .into()
            }
        };
        f.render_widget(
            Paragraph::new(empty)
                .style(Theme::muted())
                .wrap(Wrap { trim: true }),
            inner,
        );
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
            let Some(entry) = state.library.entries.get(*index) else {
                return Line::default();
            };
            let (glyph, glyph_style) = status_glyph(entry.status);
            let score = if entry.score > 0 {
                format!(" {}", entry.score)
            } else {
                String::new()
            };
            let tail = format!("{glyph}{score}");
            let title = fit(
                entry.anime.display_title(),
                (inner_width as usize).saturating_sub(8 + tail.width()),
            );
            Line::from(vec![
                Span::styled(marker, marker_style),
                Span::styled(
                    title,
                    if selected {
                        Theme::bold(Theme::base())
                    } else {
                        Theme::base()
                    },
                ),
                Span::raw(" "),
                Span::styled(tail, glyph_style),
            ])
        }
        Row::Gap { cluster, member } => {
            let (Some(_cluster), Some(member)) = (
                state.clusters.get(*cluster),
                state
                    .clusters
                    .get(*cluster)
                    .and_then(|c| c.members.get(*member)),
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
            Line::from(vec![
                Span::styled(marker, marker_style),
                Span::styled(glyph.to_string(), glyph_style),
                Span::styled(" ", Theme::base()),
                Span::styled(
                    title,
                    if selected {
                        Theme::bold(Theme::base())
                    } else {
                        Theme::base()
                    },
                ),
                Span::styled("  ", Theme::muted()),
                Span::styled(member.relation.label().to_string(), Theme::muted()),
            ])
        }
    }
}

fn status_glyph(status: LibraryStatus) -> (&'static str, Style) {
    match status {
        LibraryStatus::Completed => ("✓", Theme::ok()),
        LibraryStatus::Watching => ("▶", Theme::warn()),
        LibraryStatus::PlanToWatch => ("○", Theme::warn()),
        LibraryStatus::OnHold => ("⏸", Theme::muted()),
        LibraryStatus::Dropped => ("✗", Theme::missing()),
        LibraryStatus::Other(_) => ("?", Theme::muted()),
    }
}

/// Render the detail pane: the franchise of the selected title.
pub fn render_detail(f: &mut Frame, area: Rect, state: &AppState) {
    let focused = state.focus == Focus::Detail;
    let title = match state.detail_cluster() {
        Some(cluster) => format!(" {} ", cluster.name.to_uppercase()),
        None => match state.current_anime() {
            Some(anime) => format!(" {} ", anime.display_title().to_uppercase()),
            None => " DETAIL ".to_string(),
        },
    };
    let block = panel(&title, focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Nothing selected.
    if state.current_anime().is_none() {
        f.render_widget(
            Paragraph::new("Select a title from your library.")
                .style(Theme::muted())
                .wrap(Wrap { trim: true }),
            inner,
        );
        return;
    }

    let Some(cluster) = state.detail_cluster() else {
        render_unloaded(f, inner, state);
        return;
    };

    let mut items: Vec<ListItem> = Vec::new();
    let mut summary = vec![
        Span::styled(
            format!("{}/{} owned", cluster.owned_count(), cluster.total_count()),
            Theme::muted(),
        ),
        Span::styled("  ·  ", Theme::muted()),
        Span::styled(
            format!("{} missing", cluster.missing_count()),
            if cluster.missing_count() == 0 {
                Theme::ok()
            } else {
                Theme::missing()
            },
        ),
    ];
    if let Some(score) = cluster.best_score {
        summary.push(Span::styled("  ·  ", Theme::muted()));
        summary.push(Span::styled(format!("your score {score}"), Theme::warn()));
    }
    items.push(ListItem::new(Line::from(summary)));
    items.push(ListItem::new(Line::default()));

    for (index, member) in cluster.members.iter().enumerate() {
        let selected = index == state.member_selected;
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

        let (glyph, glyph_style) = match &member.entry {
            Some(entry) => status_glyph(entry.status),
            None => ("◌", Theme::missing()),
        };

        let title_style = if selected {
            Theme::bold(Theme::base())
        } else {
            Theme::base()
        };

        let mut meta = vec![
            member.anime.format.clone(),
            match member.anime.episodes {
                Some(episodes) => format!("{episodes} ep"),
                None => "?".to_string(),
            },
            member
                .anime
                .year
                .map(|y| y.to_string())
                .unwrap_or_else(|| "?".to_string()),
        ];
        if !member.anime.is_aired() {
            meta.push("not aired".into());
        }
        let meta_text = meta.join(" · ");
        let title = fit_title(member.anime.display_title(), inner.width, meta_text.width());

        items.push(ListItem::new(Line::from(vec![
            Span::styled(marker, marker_style),
            Span::styled(format!("{glyph} "), glyph_style),
            Span::styled(title, title_style),
            Span::styled("  ", Theme::muted()),
            Span::styled(meta_text, Theme::muted()),
        ])));

        let mut sub = match &member.entry {
            Some(entry) => format!("{} · {}", entry.status.label(), member.relation.label()),
            None => format!("missing · {}", member.relation.label()),
        };
        if let Some(entry) = &member.entry
            && entry.score > 0
        {
            sub.push_str(&format!(" · score {}", entry.score));
        }
        items.push(ListItem::new(Line::from(vec![
            Span::raw("    "),
            Span::styled(sub, Theme::muted()),
        ])));
    }

    let mut list_state = ListState::default().with_selected(Some(state.member_selected * 2 + 2));
    f.render_stateful_widget(List::new(items), inner, &mut list_state);
}

/// The selected title has not been loaded yet.
fn render_unloaded(f: &mut Frame, area: Rect, state: &AppState) {
    let Some(anime) = state.current_anime() else {
        return;
    };
    let mut lines: Vec<Line> = Vec::new();

    let mut alt = Vec::new();
    if let Some(english) = anime.english_title() {
        alt.push(Span::styled("EN: ", Theme::muted()));
        alt.push(Span::styled(english, Theme::base()));
    }
    if let Some(japanese) = anime.japanese_title() {
        if !alt.is_empty() {
            alt.push(Span::styled("  ·  ", Theme::muted()));
        }
        alt.push(Span::styled("JA: ", Theme::muted()));
        alt.push(Span::styled(japanese, Theme::base()));
    }
    if !alt.is_empty() {
        lines.push(Line::from(alt));
        lines.push(Line::default());
    }

    let mut meta = vec![anime.format.clone()];
    meta.push(match anime.episodes {
        Some(episodes) => format!("{episodes} ep"),
        None => "?".to_string(),
    });
    meta.push(
        anime
            .year
            .map(|year| year.to_string())
            .unwrap_or_else(|| "?".to_string()),
    );
    if !anime.is_aired() {
        meta.push("not aired".into());
    }
    lines.push(Line::from(Span::styled(meta.join(" · "), Theme::muted())));
    lines.push(Line::default());

    if let Some(entry) = state.library_entry(anime.mal_id) {
        lines.push(Line::from(vec![
            Span::styled(entry.status.label(), Theme::base()),
            Span::styled("  ·  ", Theme::muted()),
            Span::styled(
                if entry.score > 0 {
                    format!("your score {}", entry.score)
                } else {
                    "not rated".to_string()
                },
                Theme::warn(),
            ),
        ]));
        lines.push(Line::default());
    }

    if state.loading.is_some() {
        lines.push(Line::from(Span::styled(
            "⟳ loading its franchise…",
            Theme::accent(),
        )));
    } else {
        lines.push(Line::from(vec![
            Span::styled("press ", Theme::muted()),
            Span::styled(" L ", Theme::bold(Theme::accent())),
            Span::styled("to load its franchise", Theme::base()),
        ]));
        lines.push(Line::from(Span::styled(
            "seasons, OVAs, films and spin-offs",
            Theme::muted(),
        )));
    }

    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: true }), area);
}
