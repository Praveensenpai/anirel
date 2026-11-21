//! The franchise detail pane: member titles, watch order, and unloaded state.

use ratatui::prelude::*;
use ratatui::widgets::{List, ListItem, ListState, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::app::state::{AppState, Focus};
use crate::domain::cluster::ClusterMember;
use crate::ui::panes::status_glyph;
use crate::ui::theme::Theme;
use crate::ui::widgets::{fit_title, panel};

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

    let mut items = build_cluster_summary_items(cluster);
    for (index, member) in cluster.members.iter().enumerate() {
        let selected = index == state.member_selected;
        let (main_line, sub_line) = build_member_lines(member, selected, focused, inner.width);
        items.push(ListItem::new(main_line));
        items.push(ListItem::new(sub_line));
    }

    let mut list_state = ListState::default().with_selected(Some(state.member_selected * 2 + 2));
    f.render_stateful_widget(List::new(items), inner, &mut list_state);
}

fn build_cluster_summary_items(
    cluster: &crate::domain::cluster::Cluster,
) -> Vec<ListItem<'static>> {
    let total_dur = cluster.total_duration_minutes();
    let watched_dur = cluster.watched_duration_minutes();
    let left_dur = cluster.left_duration_minutes();

    let mut summary = vec![
        Span::styled(
            format!("{}/{} owned", cluster.owned_count(), cluster.total_count()),
            Theme::muted(),
        ),
        Span::styled(" · ", Theme::muted()),
        Span::styled(
            format!("{} missing", cluster.missing_count()),
            if cluster.missing_count() == 0 {
                Theme::ok()
            } else {
                Theme::missing()
            },
        ),
    ];
    if total_dur > 0 {
        summary.push(Span::styled(" · ", Theme::muted()));
        summary.push(Span::styled(
            format!(
                "{} total ({} watched · {} left)",
                crate::domain::model::format_duration(total_dur),
                crate::domain::model::format_duration(watched_dur),
                crate::domain::model::format_duration(left_dur)
            ),
            Theme::accent(),
        ));
    }
    if let Some(score) = cluster.best_score {
        summary.push(Span::styled(" · ", Theme::muted()));
        summary.push(Span::styled(format!("your score {score}"), Theme::warn()));
    }

    vec![
        ListItem::new(Line::from(summary)),
        ListItem::new(Line::default()),
    ]
}

fn build_member_lines(
    member: &ClusterMember,
    selected: bool,
    focused: bool,
    inner_width: u16,
) -> (Line<'static>, Line<'static>) {
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

    let meta_text = build_member_meta(member, inner_width);
    let title = fit_title(member.anime.display_title(), inner_width, meta_text.width());

    let main_line = Line::from(vec![
        Span::styled(marker, marker_style),
        Span::styled(format!("{glyph} "), glyph_style),
        Span::styled(title, title_style),
        Span::styled("  ", Theme::muted()),
        Span::styled(meta_text, Theme::muted()),
    ]);

    let sub = build_member_sub(member);
    let sub_line = Line::from(vec![Span::raw("    "), Span::styled(sub, Theme::muted())]);
    (main_line, sub_line)
}

fn build_member_meta(member: &ClusterMember, inner_width: u16) -> String {
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
    if inner_width >= 54
        && let Some(dur) = member.anime.format_total_duration()
    {
        meta.push(dur);
    }
    if !member.anime.is_aired() {
        meta.push("not aired".into());
    }
    meta.join(" · ")
}

fn build_member_sub(member: &ClusterMember) -> String {
    let mut sub = match &member.entry {
        Some(entry) => format!("{} · {}", entry.status.label(), member.relation.label()),
        None => format!("missing · {}", member.relation.label()),
    };
    if let Some(entry) = &member.entry {
        if entry.score > 0 {
            sub.push_str(&format!(" · score {}", entry.score));
        }
        sub.push_str(&format!(" · {}", entry.format_progress()));
    }
    if let Some(dur_stats) = format_member_duration_stats(member) {
        sub.push_str(&dur_stats);
    }
    sub
}

fn format_member_duration_stats(member: &ClusterMember) -> Option<String> {
    let total = member.anime.format_total_duration()?;
    let watched = member.watched_duration_minutes();
    let left = member.left_duration_minutes();
    let stats = if watched > 0 && left > 0 {
        format!(
            " · {total} total ({} watched · {} left)",
            crate::domain::model::format_duration(watched),
            crate::domain::model::format_duration(left)
        )
    } else if watched > 0 {
        format!(
            " · {total} total ({} watched)",
            crate::domain::model::format_duration(watched)
        )
    } else if left > 0 {
        format!(
            " · {total} total ({} left)",
            crate::domain::model::format_duration(left)
        )
    } else {
        format!(" · {total} total")
    };
    Some(stats)
}

/// The selected title has not been loaded yet.
fn render_unloaded(f: &mut Frame, area: Rect, state: &AppState) {
    let Some(anime) = state.current_anime() else {
        return;
    };
    let mut lines: Vec<Line> = Vec::new();
    append_unloaded_titles(&mut lines, anime);
    append_unloaded_meta(&mut lines, anime);

    if let Some(entry) = state.library_entry(anime.mal_id) {
        lines.push(Line::from(vec![
            Span::styled(entry.status.label(), Theme::base()),
            Span::styled("  ·  ", Theme::muted()),
            Span::styled(entry.format_progress(), Theme::base()),
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

fn append_unloaded_titles<'a>(lines: &mut Vec<Line<'a>>, anime: &'a crate::domain::model::Anime) {
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
}

fn append_unloaded_meta(lines: &mut Vec<Line>, anime: &crate::domain::model::Anime) {
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
}
