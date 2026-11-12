//! Core domain types: anime, relations and library entries.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// A single anime title.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Anime {
    pub mal_id: i64,
    pub title: String,
    pub title_english: Option<String>,
    pub format: String,
    pub episodes: Option<i64>,
    pub airing_status: String,
    pub year: Option<i32>,
    pub score: Option<f64>,
    /// Cover image URL, when one is known.
    pub cover_url: Option<String>,
}

impl Anime {
    /// Prefer the English title, but never an empty one.
    pub fn display_title(&self) -> &str {
        match self.title_english.as_deref() {
            Some(english) if !english.trim().is_empty() => english,
            _ => &self.title,
        }
    }

    pub fn url(&self) -> String {
        format!("https://myanimelist.net/anime/{}", self.mal_id)
    }

    pub fn is_aired(&self) -> bool {
        !self.airing_status.contains("Not yet aired")
    }

    /// Sort weight: franchises come before their specials, OVAs and films.
    pub fn format_rank(&self) -> u8 {
        match self.format.as_str() {
            "TV" => 0,
            "TV_SHORT" => 1,
            "OVA" => 2,
            "MOVIE" => 3,
            "SPECIAL" => 4,
            "ONA" => 5,
            _ => 6,
        }
    }
}

/// MyAnimeList list status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LibraryStatus {
    Watching,
    Completed,
    OnHold,
    Dropped,
    PlanToWatch,
    Other(i64),
}

impl LibraryStatus {
    pub fn from_code(code: i64) -> Self {
        match code {
            1 => Self::Watching,
            2 => Self::Completed,
            3 => Self::OnHold,
            4 => Self::Dropped,
            6 => Self::PlanToWatch,
            other => Self::Other(other),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Watching => "watching",
            Self::Completed => "completed",
            Self::OnHold => "on-hold",
            Self::Dropped => "dropped",
            Self::PlanToWatch => "plan-to-watch",
            Self::Other(_) => "unknown",
        }
    }

    /// Symbol + text so status never depends on colour alone.
    pub fn glyph(&self) -> &'static str {
        match self {
            Self::Watching => "▶",
            Self::Completed => "✓",
            Self::OnHold => "⏸",
            Self::Dropped => "✗",
            Self::PlanToWatch => "○",
            Self::Other(_) => "?",
        }
    }

    pub fn is_completed(&self) -> bool {
        matches!(self, Self::Completed)
    }
}

/// One row of the user's MyAnimeList library.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryEntry {
    pub anime: Anime,
    pub status: LibraryStatus,
    pub score: i64,
    pub episodes_watched: i64,
}

impl LibraryEntry {
    pub fn url(&self) -> String {
        self.anime.url()
    }
}

/// The user's whole library, indexed for fast lookup.
#[derive(Debug, Clone, Default)]
pub struct Library {
    pub entries: Vec<LibraryEntry>,
    index: HashMap<i64, usize>,
}

impl Library {
    pub fn new(entries: Vec<LibraryEntry>) -> Self {
        let index = entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.anime.mal_id, i))
            .collect();
        Self { entries, index }
    }

    pub fn get(&self, mal_id: i64) -> Option<&LibraryEntry> {
        self.index.get(&mal_id).map(|i| &self.entries[*i])
    }

    pub fn contains(&self, mal_id: i64) -> bool {
        self.index.contains_key(&mal_id)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Highest score the user gave to any title in this franchise.
    pub fn best_score_for(&self, mal_ids: &[i64]) -> Option<i64> {
        mal_ids
            .iter()
            .filter_map(|id| self.get(*id))
            .map(|e| e.score)
            .filter(|s| *s > 0)
            .max()
    }
}
