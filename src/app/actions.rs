//! Keyboard input mapping.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::state::View;

/// A user intent, independent of the key that produced it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    Help,
    Up,
    Down,
    PageUp,
    PageDown,
    FocusNext,
    FocusPrev,
    FocusList,
    FocusDetail,
    StartSearch,
    EndSearch,
    Backspace,
    ClearFilter,
    NextView,
    PrevView,
    ShowFranchises,
    ShowGaps,
    Open,
    Copy,
    Export,
    Refresh,
    SoftRefresh,
    /// Load the franchise of the selected title (lazy, on demand).
    Load,
    /// Load every franchise in the library at once.
    ScanAll,
    Insert(char),
    Noop,
}

/// Translate a key press into an action.
pub fn action_for(key: KeyEvent, search_mode: bool) -> Action {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    if ctrl {
        return match key.code {
            KeyCode::Char('c') => Action::Quit,
            KeyCode::Char('n') => Action::NextView,
            KeyCode::Char('p') => Action::PrevView,
            KeyCode::Char('r') => Action::SoftRefresh,
            KeyCode::Char('l') => Action::Load,
            KeyCode::Char('u') => Action::Open,
            KeyCode::Char('y') => Action::Copy,
            _ => Action::Noop,
        };
    }

    if search_mode {
        return match key.code {
            KeyCode::Esc => Action::EndSearch,
            KeyCode::Enter => Action::EndSearch,
            KeyCode::Backspace => Action::Backspace,
            KeyCode::Up => Action::Up,
            KeyCode::Down => Action::Down,
            KeyCode::Char(c) => Action::Insert(c),
            _ => Action::Noop,
        };
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
        KeyCode::Char('?') => Action::Help,
        KeyCode::Char('j') | KeyCode::Char('J') | KeyCode::Down => Action::Down,
        KeyCode::Char('k') | KeyCode::Char('K') | KeyCode::Up => Action::Up,
        KeyCode::PageDown => Action::PageDown,
        KeyCode::PageUp => Action::PageUp,
        KeyCode::Tab => Action::FocusNext,
        KeyCode::BackTab => Action::FocusPrev,
        KeyCode::Char('/') => Action::StartSearch,
        KeyCode::Esc => Action::ClearFilter,
        KeyCode::Char('1') => Action::ShowFranchises,
        KeyCode::Char('2') => Action::ShowGaps,
        KeyCode::Char('o') | KeyCode::Char('O') | KeyCode::Enter => Action::Open,
        KeyCode::Char('c') | KeyCode::Char('C') => Action::Copy,
        KeyCode::Char('e') | KeyCode::Char('E') => Action::Export,
        KeyCode::Char('r') | KeyCode::Char('R') => Action::Refresh,
        KeyCode::Char('l') | KeyCode::Char('L') => Action::Load,
        KeyCode::Char('a') | KeyCode::Char('A') => Action::ScanAll,
        KeyCode::Char('n') | KeyCode::Char('N') => Action::NextView,
        KeyCode::Char('p') | KeyCode::Char('P') => Action::PrevView,
        _ => Action::Noop,
    }
}

/// Cycle through the available views.
pub fn next_view(view: View) -> View {
    match view {
        View::Library => View::Missing,
        View::Missing => View::Library,
    }
}

pub fn prev_view(view: View) -> View {
    next_view(view)
}
