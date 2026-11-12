//! Error types shared across the crate.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AnirelError {
    #[error("network request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("could not parse data from {context}: {detail}")]
    Parse { context: String, detail: String },

    #[error("upstream api error: {0}")]
    Api(String),

    #[error("could not read the MyAnimeList list for '{user}': {detail}")]
    UserNotFound { user: String, detail: String },

    #[error("cache problem: {0}")]
    Cache(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Other(String),
}

impl AnirelError {
    pub fn parse(context: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Parse {
            context: context.into(),
            detail: detail.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, AnirelError>;
