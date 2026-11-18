//! Disk cache for library snapshots and resolved media.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::domain::model::{Library, LibraryEntry};
use crate::error::{AnirelError, Result};
use crate::infra::anilist::Media;

#[derive(Serialize, Deserialize)]
struct Cached<T> {
    fetched_at: u64,
    payload: T,
}

/// File-backed cache rooted at a directory.
#[derive(Debug, Clone)]
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn library_path(&self, user: &str) -> PathBuf {
        self.root
            .join("library")
            .join(format!("{}.json", sanitize(user)))
    }

    fn media_path(&self, mal_id: i64) -> PathBuf {
        self.root.join("media").join(format!("{mal_id}.json"))
    }

    pub fn read_library(&self, user: &str, ttl: Duration) -> Option<Library> {
        let path = self.library_path(user);
        let cached: Cached<Vec<LibraryEntry>> = read_json(&path).ok()?;
        if expired(cached.fetched_at, ttl) {
            return None;
        }
        if !cached.payload.is_empty() && cached.payload.iter().all(|e| e.updated_at.is_none()) {
            return None;
        }
        Some(Library::new(cached.payload))
    }

    pub fn write_library(&self, user: &str, library: &Library) -> Result<()> {
        let path = self.library_path(user);
        write_json(
            &path,
            &Cached {
                fetched_at: now(),
                payload: library.entries.clone(),
            },
        )
    }

    pub fn read_media(&self, mal_id: i64, ttl: Duration) -> Option<Media> {
        let path = self.media_path(mal_id);
        let cached: Cached<Media> = read_json(&path).ok()?;
        if expired(cached.fetched_at, ttl) {
            return None;
        }
        Some(cached.payload)
    }

    pub fn write_media(&self, mal_id: i64, media: &Media) -> Result<()> {
        write_json(
            &self.media_path(mal_id),
            &Cached {
                fetched_at: now(),
                payload: media.clone(),
            },
        )
    }

    /// Drop every cached media file (used by `rescan --hard`).
    pub fn clear_media(&self) -> Result<()> {
        let dir = self.root.join("media");
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        Ok(())
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn expired(fetched_at: u64, ttl: Duration) -> bool {
    now().saturating_sub(fetched_at) > ttl.as_secs()
}

fn sanitize(user: &str) -> String {
    user.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| AnirelError::Cache(format!("{}: {e}", path.display())))?;
    serde_json::from_str(&raw).map_err(|e| AnirelError::Cache(format!("{}: {e}", path.display())))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AnirelError::Cache(format!("{}: {e}", parent.display())))?;
    }
    let raw = serde_json::to_string_pretty(value)
        .map_err(|e| AnirelError::Cache(format!("{}: {e}", path.display())))?;
    std::fs::write(path, raw).map_err(|e| AnirelError::Cache(format!("{}: {e}", path.display())))
}
