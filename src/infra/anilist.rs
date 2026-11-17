//! AniList GraphQL backend: relation graph + metadata, keyed by MyAnimeList ID.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::domain::model::Anime;
use crate::domain::relation::RelationKind;
use crate::error::{AnirelError, Result};

const ENDPOINT: &str = "https://graphql.anilist.co";
/// How many MAL ids to resolve per GraphQL request.
///
/// AniList rejects queries above a complexity of 500; twenty of these
/// aliases stay comfortably below that ceiling.
pub const BATCH_SIZE: usize = 20;

/// Pause between requests so the public rate limit is never hit.
pub const REQUEST_PAUSE: Duration = Duration::from_millis(2200);

/// Fields for the resolved title.
///
/// Relation nodes only carry the fields needed to recognise them as anime;
/// their full metadata is fetched when they are resolved on their own.
const MEDIA_FIELDS: &str = "id idMal title { romaji english native } format episodes duration status \
     startDate { year month day } averageScore coverImage { large } \
     relations { edges { relationType node { idMal title { romaji english native } format episodes duration } } }";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Title {
    #[serde(default)]
    pub romaji: Option<String>,
    #[serde(default)]
    pub english: Option<String>,
    #[serde(default)]
    pub native: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverImage {
    #[serde(default)]
    pub large: Option<String>,
    #[serde(default)]
    pub medium: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FuzzyDate {
    #[serde(default)]
    pub year: Option<i32>,
    #[serde(default)]
    pub month: Option<i32>,
    #[serde(default)]
    pub day: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationEdge {
    #[serde(rename = "relationType")]
    pub relation_type: String,
    #[serde(default)]
    pub node: Box<Media>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Relations {
    #[serde(default)]
    pub edges: Vec<RelationEdge>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Media {
    /// AniList's internal id. Relation nodes do not always carry one.
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(rename = "idMal", default)]
    pub id_mal: Option<i64>,
    #[serde(default)]
    pub title: Title,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub episodes: Option<i64>,
    #[serde(default)]
    pub duration: Option<i64>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(rename = "startDate", default)]
    pub start_date: Option<FuzzyDate>,
    #[serde(rename = "averageScore", default)]
    pub average_score: Option<f64>,
    #[serde(rename = "siteUrl", default)]
    pub site_url: Option<String>,
    #[serde(rename = "coverImage", default)]
    pub cover_image: Option<CoverImage>,
    #[serde(default)]
    pub relations: Option<Relations>,
}

/// Formats that represent an anime title (manga/novel relations are skipped).
const ANIME_FORMATS: [&str; 7] = ["TV", "TV_SHORT", "OVA", "MOVIE", "SPECIAL", "ONA", "MUSIC"];

impl Media {
    pub fn to_anime(&self) -> Option<Anime> {
        let mal_id = self.id_mal?;
        let format = self.format.clone().unwrap_or_else(|| "TV".into());
        if !ANIME_FORMATS.contains(&format.as_str()) {
            return None;
        }
        let title = self
            .title
            .romaji
            .clone()
            .or_else(|| self.title.english.clone())
            .unwrap_or_else(|| match self.id {
                Some(id) => format!("Anime {id}"),
                None => "Unknown title".to_string(),
            });
        let aired_from = self
            .start_date
            .as_ref()
            .and_then(|d| Some((d.year?, d.month.unwrap_or(1), d.day.unwrap_or(1))));

        Some(Anime {
            mal_id,
            title,
            title_english: self.title.english.clone(),
            title_japanese: self.title.native.clone(),
            format,
            episodes: self.episodes,
            duration: self.duration,
            airing_status: airing_status(self.status.as_deref()),
            year: self.start_date.as_ref().and_then(|d| d.year),
            aired_from,
            score: self.average_score.map(|s| s / 10.0),
            cover_url: self
                .cover_image
                .as_ref()
                .and_then(|image| image.large.clone().or_else(|| image.medium.clone())),
        })
    }
}

fn airing_status(status: Option<&str>) -> String {
    match status {
        Some("FINISHED") => "Finished Airing".into(),
        Some("RELEASING") => "Currently Airing".into(),
        Some("NOT_YET_RELEASED") => "Not yet aired".into(),
        Some("CANCELLED") => "Cancelled".into(),
        _ => "Unknown".into(),
    }
}

/// A resolved title plus the relations that were discovered with it.
#[derive(Debug, Clone)]
pub struct ResolvedMedia {
    pub anime: Anime,
    pub relations: Vec<(i64, RelationKind)>,
}

/// Client for the AniList GraphQL API.
#[derive(Debug, Clone)]
pub struct AniListClient {
    http: reqwest::Client,
}

impl AniListClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent("anirel/0.1 (+https://github.com/anirel)")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build http client");
        Self { http }
    }

    /// Resolve up to `BATCH_SIZE` MAL ids in a single request.
    ///
    /// AniList answers an unknown MyAnimeList id with a `Not Found.` error
    /// that aborts the whole query, so the offending id is identified from
    /// the error position and the request is retried without it.
    pub async fn resolve(&self, mal_ids: &[i64]) -> Result<Vec<Option<Media>>> {
        let mut out: Vec<Option<Media>> = vec![None; mal_ids.len()];
        let mut pending: Vec<usize> = (0..mal_ids.len()).collect();
        let mut attempts = 0usize;

        while !pending.is_empty() {
            attempts += 1;
            if attempts > 12 {
                break;
            }
            let ids: Vec<i64> = pending.iter().map(|i| mal_ids[*i]).collect();
            let (media, unknown) = self.resolve_once(&ids).await?;
            for (slot, item) in pending.iter().zip(media) {
                if let Some(item) = item {
                    out[*slot] = Some(item);
                }
            }
            if unknown.is_empty() {
                break;
            }
            let unknown: std::collections::HashSet<usize> = unknown.into_iter().collect();
            pending.retain(|index| !unknown.contains(index));
            tokio::time::sleep(REQUEST_PAUSE).await;
        }
        Ok(out)
    }

    /// One request; returns the media plus the indexes of unknown MAL ids.
    async fn resolve_once(&self, mal_ids: &[i64]) -> Result<(Vec<Option<Media>>, Vec<usize>)> {
        if mal_ids.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let mut parts: Vec<String> = Vec::new();
        let mut variables = serde_json::Map::new();
        for (i, id) in mal_ids.iter().enumerate() {
            parts.push(format!(
                "a{i}: Media(idMal: $m{i}, type: ANIME) {{ {MEDIA_FIELDS} }}"
            ));
            variables.insert(format!("m{i}"), json!(id));
        }
        let declaration: Vec<String> = (0..mal_ids.len()).map(|i| format!("$m{i}: Int")).collect();
        let prefix = format!("query ({}) {{ ", declaration.join(", "));
        let query = format!("{prefix}{} }}", parts.join(" "));

        // Column of every alias, so an error can be traced back to its id.
        let mut offsets: Vec<usize> = Vec::with_capacity(parts.len());
        let mut cursor = prefix.len();
        for part in &parts {
            offsets.push(cursor);
            cursor += part.len() + 1;
        }

        let payload = json!({ "query": query, "variables": Value::Object(variables.clone()) });

        if std::env::var_os("ANIREL_DEBUG").is_some() {
            eprintln!(
                "--- QUERY ---\n{query}\n--- VARS ---\n{}",
                Value::Object(variables)
            );
        }

        let mut attempt = 0u8;
        let response = loop {
            let response = self
                .http
                .post(ENDPOINT)
                .json(&payload)
                .send()
                .await
                .map_err(AnirelError::Http)?;

            if response.status() == 429 && attempt < 3 {
                attempt += 1;
                tokio::time::sleep(std::time::Duration::from_secs(15 * attempt as u64)).await;
                continue;
            }
            break response;
        };

        let status = response.status();
        let raw = response.text().await.map_err(AnirelError::Http)?;
        if std::env::var_os("ANIREL_DEBUG").is_some() {
            eprintln!("--- RESPONSE ---\n{}", &raw[..raw.len().min(600)]);
        }

        let body: Value = serde_json::from_str(&raw)
            .map_err(|e| AnirelError::parse("anilist", format!("{e}: {raw}")))?;

        let mut unknown: Vec<usize> = Vec::new();
        if let Some(errors) = body.get("errors").and_then(Value::as_array) {
            for error in errors {
                let message = error
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                if message.contains("Not Found") {
                    let column = error
                        .pointer("/locations/0/column")
                        .and_then(Value::as_u64)
                        .unwrap_or(0) as usize;
                    let position = column.saturating_sub(1);
                    if let Some(index) = offsets
                        .iter()
                        .enumerate()
                        .filter(|(_, offset)| **offset <= position)
                        .map(|(index, _)| index)
                        .next_back()
                    {
                        if !unknown.contains(&index) {
                            unknown.push(index);
                        }
                        continue;
                    }
                }
                if !status.is_success() {
                    return Err(AnirelError::Api(format!("HTTP {status}: {message}")));
                }
                return Err(AnirelError::Api(message.to_string()));
            }
        }

        let data = body
            .get("data")
            .and_then(Value::as_object)
            .ok_or_else(|| AnirelError::parse("anilist", format!("response has no data: {raw}")))?;

        let mut out = Vec::with_capacity(mal_ids.len());
        for i in 0..mal_ids.len() {
            let media = data
                .get(&format!("a{i}"))
                .filter(|v| !v.is_null())
                .and_then(|v| serde_json::from_value::<Media>(v.clone()).ok());
            out.push(media);
        }
        Ok((out, unknown))
    }
}

impl Default for AniListClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Search AniList by title, returning MAL ids.
pub async fn search(_client: &AniListClient, query: &str) -> Result<Vec<Anime>> {
    let http = reqwest::Client::new();
    let gql = r#"query ($q: String) { Page(page: 1, perPage: 10) { media(search: $q, type: ANIME, sort: SEARCH_MATCH) { id idMal title { romaji english native } format episodes status startDate { year } averageScore } } }"#;
    let payload = json!({ "query": gql, "variables": { "q": query } });
    let response = http
        .post(ENDPOINT)
        .json(&payload)
        .send()
        .await
        .map_err(AnirelError::Http)?;
    if !response.status().is_success() {
        return Err(AnirelError::Api(format!("HTTP {}", response.status())));
    }
    let body: Value = response.json().await.map_err(AnirelError::Http)?;
    let media = body
        .pointer("/data/Page/media")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(media
        .into_iter()
        .filter_map(|v| serde_json::from_value::<Media>(v).ok())
        .filter_map(|m| m.to_anime())
        .collect())
}
