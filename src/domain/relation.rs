//! Relation kinds and how they are classified.

use serde::{Deserialize, Serialize};

/// The kind of link between two anime titles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationKind {
    Prequel,
    Sequel,
    SideStory,
    Parent,
    SpinOff,
    Summary,
    Alternative,
    Character,
    Adaptation,
    Other,
}

impl RelationKind {
    /// Parse a relation label as used by MAL / AniList.
    ///
    /// Handles the different spellings: `SPIN_OFF`, `Spin-off`, `Side Story`.
    pub fn parse(raw: &str) -> Self {
        let normalised = raw.trim().to_ascii_lowercase().replace(['_', '-'], " ");
        match normalised.as_str() {
            "prequel" => Self::Prequel,
            "sequel" => Self::Sequel,
            "side story" => Self::SideStory,
            "parent story" | "parent" => Self::Parent,
            "spin off" | "spinoff" => Self::SpinOff,
            "summary" => Self::Summary,
            "alternative" | "alternative setting" | "alternative version" | "full version" => {
                Self::Alternative
            }
            "character" | "character(s) in" => Self::Character,
            "adaptation" => Self::Adaptation,
            _ => Self::Other,
        }
    }

    /// Human label used in the UI.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Prequel => "prequel",
            Self::Sequel => "sequel",
            Self::SideStory => "side story",
            Self::Parent => "parent story",
            Self::SpinOff => "spin-off",
            Self::Summary => "summary",
            Self::Alternative => "alternative",
            Self::Character => "crossover",
            Self::Adaptation => "adaptation",
            Self::Other => "other",
        }
    }

    /// True when the link stays inside the same franchise.
    pub fn is_franchise(&self) -> bool {
        matches!(
            self,
            Self::Prequel
                | Self::Sequel
                | Self::SideStory
                | Self::Parent
                | Self::SpinOff
                | Self::Summary
                | Self::Alternative
        )
    }

    /// True when a missing entry of this kind is worth reporting as a gap.
    pub fn is_gap_relevant(&self) -> bool {
        self.is_franchise() || matches!(self, Self::Character)
    }
}
