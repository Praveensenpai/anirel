//! Pure state transitions: actions in, state changes and effects out.

use crate::app::actions::Action;
use crate::app::state::{AppState, Focus, MessageKind, SortOrder, View};

/// A side effect the host must perform (I/O stays out of the state layer).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    Open(String),
    Copy(String),
    Export,
    Rescan,
    /// Re-download the user's list, bypassing the library cache.
    RefreshLibrary,
    /// Fetch the relations of one title, `hops` levels deep.
    Load {
        mal_id: i64,
        hops: usize,
    },
    /// Fetch the relations of the whole library.
    ScanAll,
}

/// Apply an action to the state.
pub fn apply(state: &mut AppState, action: Action) -> Option<Effect> {
    match action {
        Action::Quit => {
            state.should_quit = true;
            None
        }
        Action::Help => {
            state.show_help = !state.show_help;
            None
        }
        Action::Up => {
            move_selection(state, -1);
            None
        }
        Action::Down => {
            move_selection(state, 1);
            None
        }
        Action::PageUp => {
            move_selection(state, -10);
            None
        }
        Action::PageDown => {
            move_selection(state, 10);
            None
        }
        Action::FocusNext => {
            state.focus = match state.focus {
                Focus::List => Focus::Detail,
                Focus::Detail => Focus::List,
            };
            None
        }
        Action::FocusPrev => {
            state.focus = match state.focus {
                Focus::List => Focus::Detail,
                Focus::Detail => Focus::List,
            };
            None
        }
        Action::FocusList => {
            state.focus = Focus::List;
            None
        }
        Action::FocusDetail => {
            state.focus = Focus::Detail;
            None
        }
        Action::StartSearch => {
            state.search_mode = true;
            state.focus = Focus::List;
            None
        }
        Action::EndSearch => {
            state.search_mode = false;
            None
        }
        Action::Insert(ch) => {
            state.set_filter(format!("{}{ch}", state.filter));
            None
        }
        Action::Backspace => {
            let mut filter = state.filter.clone();
            filter.pop();
            state.set_filter(filter);
            None
        }
        Action::ClearFilter => {
            if state.search_mode {
                state.search_mode = false;
            } else {
                state.set_filter(String::new());
            }
            None
        }
        Action::ShowFranchises => {
            state.set_view(View::Library);
            None
        }
        Action::ShowGaps => {
            state.set_view(View::Missing);
            None
        }
        Action::NextView => {
            state.set_view(crate::app::actions::next_view(state.view));
            None
        }
        Action::PrevView => {
            state.set_view(crate::app::actions::prev_view(state.view));
            None
        }
        Action::Open => state.focus_target().map(|anime| Effect::Open(anime.url())),
        Action::Copy => state.focus_target().map(|anime| Effect::Copy(anime.url())),
        Action::Export => Some(Effect::Export),
        Action::Refresh => Some(Effect::Rescan),
        Action::RefreshLibrary => Some(Effect::RefreshLibrary),
        Action::Load => {
            // From the library you want the whole franchise; diving into a
            // related title you want that title's own neighbourhood.
            let hops = match state.focus {
                Focus::Detail => 1,
                Focus::List => 3,
            };
            state.focus_target().map(|anime| Effect::Load {
                mal_id: anime.mal_id,
                hops,
            })
        }
        Action::ScanAll => Some(Effect::ScanAll),
        Action::ToggleSort => {
            let next = SortOrder::next(state.sort);
            state.set_sort(next);
            None
        }
        Action::Noop => None,
    }
}

fn move_selection(state: &mut AppState, delta: i64) {
    match state.focus {
        Focus::List => {
            let len = state.rows().len();
            if len == 0 {
                return;
            }
            let next = (state.selected as i64 + delta).clamp(0, len as i64 - 1);
            state.selected = next as usize;
            state.member_selected = 0;
            state.detail_root = None;
            state.focus_cluster = None;
        }
        Focus::Detail => {
            // The cursor moves inside the franchise the detail pane shows.
            let Some(cluster) = state.detail_cluster() else {
                return;
            };
            let len = cluster.members.len();
            if len == 0 {
                return;
            }
            let next = (state.member_selected as i64 + delta).clamp(0, len as i64 - 1);
            state.member_selected = next as usize;
        }
    }
}

/// Set a transient message in the status bar.
pub fn notify(state: &mut AppState, text: impl Into<String>, kind: MessageKind) {
    state.message = Some(crate::app::state::StatusMessage {
        text: text.into(),
        kind,
    });
}
