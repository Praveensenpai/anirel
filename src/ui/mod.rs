//! Ratatui rendering for the anirel TUI.

use ratatui::Frame;
use ratatui::prelude::*;
use ratatui::widgets::{Paragraph, Wrap};

use crate::app::state::AppState;
use crate::ui::theme::Theme;

pub mod chrome;
pub mod help;
pub mod loading;
pub mod panes;
pub mod poster;
pub mod theme;
pub mod widgets;

/// Top-level draw function.
pub fn draw(f: &mut Frame, state: &AppState) {
    let area = f.area();
    if area.width < Theme::MIN_WIDTH || area.height < Theme::MIN_HEIGHT {
        render_too_small(f, area);
        return;
    }

    let layout = Layout::vertical([
        Constraint::Length(1), // header
        Constraint::Length(1), // tabs + filter
        Constraint::Min(0),    // body
        Constraint::Length(1), // footer
    ])
    .split(area);

    let body = Layout::horizontal(body_constraints(area.width)).split(layout[2]);

    chrome::render_header(f, layout[0], state);
    chrome::render_tabs(f, layout[1], state);
    panes::render_list(f, body[0], state);
    panes::render_detail(f, body[1], state);
    if body.len() > 2 {
        poster::render_poster(f, body[2], state);
    }

    chrome::render_footer(f, layout[3], state);

    if state.initial_loading.is_some() {
        loading::render_loading(f, area, state);
    } else if state.show_help {
        help::render_help(f, area);
    }
}

/// The poster card only appears when there is room for it.
fn body_constraints(width: u16) -> Vec<Constraint> {
    match width {
        w if w >= 140 => vec![
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ],
        w if w >= 110 => vec![
            Constraint::Percentage(30),
            Constraint::Percentage(36),
            Constraint::Percentage(34),
        ],
        w if w >= 96 => vec![
            Constraint::Percentage(32),
            Constraint::Percentage(34),
            Constraint::Percentage(34),
        ],
        _ => vec![Constraint::Percentage(46), Constraint::Percentage(54)],
    }
}

fn render_too_small(f: &mut Frame, area: Rect) {
    let message = format!(
        "anirel needs at least {}x{}\ncurrent: {}x{}",
        Theme::MIN_WIDTH,
        Theme::MIN_HEIGHT,
        area.width,
        area.height
    );
    f.render_widget(
        Paragraph::new(message)
            .style(Theme::warn())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
        area,
    );
}
