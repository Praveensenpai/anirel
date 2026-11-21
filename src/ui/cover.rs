//! Cover art rendering in the terminal using half-block characters.

use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::art::{CoverArt, color};
use crate::ui::theme::Theme;

/// Typical anime poster aspect ratio (width : height ≈ 1 : 1.45).
const POSTER_ASPECT: f32 = 1.45;

/// Compute the (columns, rows) for displaying a poster inside `area`.
pub fn compute_cover_dimensions(area: Rect) -> (usize, usize) {
    if area.width < 6 || area.height < 4 {
        return (0, 0);
    }
    let max_rows = area.height as usize;
    let max_cols = area.width as usize;

    let mut rows = max_rows;
    let mut cols = ((2.0 * rows as f32) / POSTER_ASPECT).round() as usize;
    if cols > max_cols {
        cols = max_cols;
        rows = ((cols as f32 * POSTER_ASPECT) / 2.0).round() as usize;
    }

    let rows = rows.clamp(1, max_rows);
    let cols = cols.clamp(1, max_cols);
    (cols, rows)
}

/// Render the cover art centered within `area`.
pub fn render_cover(
    f: &mut Frame,
    area: Rect,
    art: &CoverArt,
    mal_id: i64,
    state: &crate::app::state::AppState,
) {
    let (cols, rows) = compute_cover_dimensions(area);
    if cols == 0 || rows == 0 {
        return;
    }

    let start_x = area.x + (area.width.saturating_sub(cols as u16)) / 2;
    let start_y = area.y + (area.height.saturating_sub(rows as u16)) / 2;

    if crate::art::detect_graphics() == crate::art::GraphicsSupport::Kitty {
        skip_cover_cells(f, start_x, start_y, cols as u16, rows as u16);
        state
            .active_cover
            .set(Some(crate::app::state::CoverPlacement {
                mal_id,
                x: start_x,
                y: start_y,
                cols: cols as u16,
                rows: rows as u16,
            }));
    } else {
        render_halfblock_cover(f, start_x, start_y, cols, rows, art);
    }
}

fn skip_cover_cells(f: &mut Frame, start_x: u16, start_y: u16, cols: u16, rows: u16) {
    for cy in start_y..start_y + rows {
        for cx in start_x..start_x + cols {
            let cell = &mut f.buffer_mut()[(cx, cy)];
            cell.reset();
            cell.set_diff_option(ratatui::buffer::CellDiffOption::Skip);
        }
    }
}

/// Render the half-block Unicode fallback for standard terminals.
pub fn render_halfblock_cover(
    f: &mut Frame,
    start_x: u16,
    start_y: u16,
    cols: usize,
    rows: usize,
    art: &CoverArt,
) {
    let pixels = art.resample(cols, rows * 2);
    for r in 0..rows {
        let spans = build_row_spans(&pixels, cols, r);
        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect {
                x: start_x,
                y: start_y + r as u16,
                width: cols as u16,
                height: 1,
            },
        );
    }
}

/// Flush Kitty native image updates to stdout when the cover placement changes.
pub fn sync_kitty_cover(
    state: &crate::app::state::AppState,
    last_placed: &mut Option<crate::app::state::CoverPlacement>,
) {
    let current = state.active_cover.get();
    if current == *last_placed {
        return;
    }
    use std::io::Write;
    let mut stdout = std::io::stdout();
    if last_placed.is_some() {
        let _ = stdout.write_all(crate::art::kitty_clear_visible().as_bytes());
    }
    if let Some(placement) = current
        && let Some(art) = state.cover_for(placement.mal_id)
    {
        let seq = crate::art::kitty_render_art(
            art,
            placement.x,
            placement.y,
            placement.cols,
            placement.rows,
        );
        let _ = stdout.write_all(seq.as_bytes());
    }
    let _ = stdout.flush();
    *last_placed = current;
}

/// Clear Kitty graphics placements on exit.
pub fn cleanup_kitty_cover(last_placed: Option<crate::app::state::CoverPlacement>) {
    if last_placed.is_some() {
        use std::io::Write;
        let mut stdout = std::io::stdout();
        let _ = stdout.write_all(crate::art::kitty_clear_all().as_bytes());
        let _ = stdout.flush();
    }
}

/// Build the half-block spans for a single row of the cover.
fn build_row_spans(pixels: &[u8], cols: usize, r: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::with_capacity(cols);
    for c in 0..cols {
        let top_i = ((2 * r) * cols + c) * 3;
        let bot_i = ((2 * r + 1) * cols + c) * 3;
        let fg = color(pixels[top_i], pixels[top_i + 1], pixels[top_i + 2]);
        let bg = color(pixels[bot_i], pixels[bot_i + 1], pixels[bot_i + 2]);
        spans.push(Span::styled("▀", Style::default().fg(fg).bg(bg)));
    }
    spans
}

/// Render a loading placeholder while the cover image is being downloaded.
pub fn render_placeholder(f: &mut Frame, area: Rect) {
    if area.height < 2 || area.width < 10 {
        return;
    }
    let p = Paragraph::new("⟳ loading cover…")
        .style(Theme::muted())
        .alignment(Alignment::Center);
    let y = area.y + area.height / 2;
    f.render_widget(
        p,
        Rect {
            x: area.x,
            y,
            width: area.width,
            height: 1,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_cover_dimensions_respects_bounds_and_ratio() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 30,
            height: 15,
        };
        let (cols, rows) = compute_cover_dimensions(area);
        assert!(cols > 0 && cols <= 30);
        assert!(rows > 0 && rows <= 15);
    }

    #[test]
    fn compute_cover_dimensions_returns_zero_on_tiny_areas() {
        let area = Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 2,
        };
        let (cols, rows) = compute_cover_dimensions(area);
        assert_eq!((cols, rows), (0, 0));
    }
}
