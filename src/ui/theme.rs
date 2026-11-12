//! Visual language: colours, spacing and status symbols.

use ratatui::style::{Color, Modifier, Style};

/// Colour palette.
///
/// Body text uses the terminal's default colour so the UI stays readable on
/// both dark and light themes; accents are limited to four hues that each
/// carry one meaning, always paired with a symbol so colour is never the only
/// signal.
pub struct Theme;

impl Theme {
    pub const ACCENT: Color = Color::Cyan;
    pub const OK: Color = Color::Green;
    pub const WARN: Color = Color::Yellow;
    pub const MISSING: Color = Color::Magenta;
    pub const MUTED: Color = Color::DarkGray;
    pub const TEXT: Color = Color::Reset;

    pub const MIN_WIDTH: u16 = 64;
    pub const MIN_HEIGHT: u16 = 16;

    pub fn base() -> Style {
        Style::default().fg(Self::TEXT)
    }

    pub fn muted() -> Style {
        Style::default().fg(Self::MUTED)
    }

    pub fn accent() -> Style {
        Style::default().fg(Self::ACCENT)
    }

    pub fn ok() -> Style {
        Style::default().fg(Self::OK)
    }

    pub fn warn() -> Style {
        Style::default().fg(Self::WARN)
    }

    pub fn missing() -> Style {
        Style::default().fg(Self::MISSING)
    }

    pub fn bold(style: Style) -> Style {
        style.add_modifier(Modifier::BOLD)
    }

    pub fn dim(style: Style) -> Style {
        style.add_modifier(Modifier::DIM)
    }
}

/// Glyph for a library status.
pub fn status_glyph(status: crate::domain::model::LibraryStatus) -> &'static str {
    status.glyph()
}

/// Glyph for a title that is not in the library at all.
pub const MISSING_GLYPH: &str = "◌";
