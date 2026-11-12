//! Runtime configuration: paths, cache lifetimes and relation scope switches.

use std::env;
use std::path::PathBuf;
use std::time::Duration;

/// Which relation types count as "the same franchise" and therefore as a gap.
#[derive(Debug, Clone, Copy)]
pub struct Scope {
    /// Character (s) in - crossovers into other shows.
    pub include_character: bool,
    /// "Other" - vague links, often promotional.
    pub include_other: bool,
    /// Alternative setting / alternative version.
    pub include_alternative: bool,
}

impl Default for Scope {
    fn default() -> Self {
        Self {
            include_character: true,
            include_other: false,
            include_alternative: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub user: String,
    pub cache_dir: PathBuf,
    pub library_ttl: Duration,
    pub media_ttl: Duration,
    pub scope: Scope,
}

impl Config {
    pub fn new(user: impl Into<String>) -> Self {
        Self {
            user: user.into(),
            cache_dir: cache_dir(),
            library_ttl: Duration::from_secs(6 * 3600),
            media_ttl: Duration::from_secs(30 * 24 * 3600),
            scope: Scope::default(),
        }
    }

    pub fn library_dir(&self) -> PathBuf {
        self.cache_dir.join("library")
    }

    pub fn media_dir(&self) -> PathBuf {
        self.cache_dir.join("media")
    }

    pub fn state_dir(&self) -> PathBuf {
        self.cache_dir.join("state")
    }
}

/// XDG-compliant cache directory: `$XDG_CACHE_HOME/anirel` or `~/.cache/anirel`.
pub fn cache_dir() -> PathBuf {
    if let Ok(dir) = env::var("XDG_CACHE_HOME")
        && !dir.is_empty()
    {
        return PathBuf::from(dir).join("anirel");
    }
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cache").join("anirel")
}

/// Remember the last used username so `anirel` alone works on repeat runs.
pub fn remember_user(user: &str) {
    let dir = cache_dir().join("state");
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let _ = std::fs::write(dir.join("last_user"), user);
}

pub fn last_user() -> Option<String> {
    let path = cache_dir().join("state").join("last_user");
    let raw = std::fs::read_to_string(path).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
