//! MyAnimeList web backend: the user's list and, as a fallback, related entries.

use serde::Deserialize;
use serde_json::Value;

use crate::domain::model::{Anime, Library, LibraryEntry, LibraryStatus};
use crate::domain::relation::RelationKind;
use crate::error::{AnirelError, Result};

const LIST_URL: &str = "https://myanimelist.net/animelist/{user}?status=7&offset={offset}&order=5";
const ANIME_URL: &str = "https://myanimelist.net/anime/{id}";
const PAGE_SIZE: usize = 300;
const USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Safari/537.36";

/// Reads public MyAnimeList pages.
#[derive(Debug, Clone)]
pub struct MalWebClient {
    http: reqwest::Client,
}

impl MalWebClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build http client");
        Self { http }
    }

    /// Fetch every entry of a user's list.
    pub async fn fetch_library(&self, user: &str) -> Result<Library> {
        self.fetch_library_with_progress(user, |_| {}).await
    }

    /// Fetch every entry of a user's list, invoking `on_progress(count)` as batches arrive.
    pub async fn fetch_library_with_progress<F>(
        &self,
        user: &str,
        mut on_progress: F,
    ) -> Result<Library>
    where
        F: FnMut(usize),
    {
        let mut entries: Vec<LibraryEntry> = Vec::new();
        let mut offset = 0usize;

        loop {
            let url = LIST_URL
                .replace("{user}", user)
                .replace("{offset}", &offset.to_string());
            let html = self.get(&url).await.map_err(|e| match e {
                AnirelError::Api(detail) => AnirelError::UserNotFound {
                    user: user.to_string(),
                    detail,
                },
                other => other,
            })?;

            let (raw_count, page) = parse_list_items(&html);
            entries.extend(page);
            on_progress(entries.len());
            if raw_count == 0 {
                break;
            }
            offset += PAGE_SIZE;
            tokio::time::sleep(std::time::Duration::from_millis(600)).await;
            if offset > 20_000 {
                break;
            }
        }

        if entries.is_empty() {
            return Err(AnirelError::UserNotFound {
                user: user.to_string(),
                detail: "the list came back empty - is the username correct?".into(),
            });
        }
        Ok(Library::new(entries))
    }

    /// Fallback relation source: scrape the "Related Entries" block of an anime page.
    pub async fn fetch_related(&self, mal_id: i64) -> Result<Vec<(i64, RelationKind)>> {
        let url = ANIME_URL.replace("{id}", &mal_id.to_string());
        let html = self.get(&url).await?;
        Ok(parse_related_entries(&html))
    }

    async fn get(&self, url: &str) -> Result<String> {
        let response = self
            .http
            .get(url)
            .header("Accept-Language", "en-US,en;q=0.9")
            .send()
            .await
            .map_err(AnirelError::Http)?;

        if response.status() == 404 {
            return Err(AnirelError::Api("page not found".into()));
        }
        if !response.status().is_success() {
            return Err(AnirelError::Api(format!("HTTP {}", response.status())));
        }
        response.text().await.map_err(AnirelError::Http)
    }
}

impl Default for MalWebClient {
    fn default() -> Self {
        Self::new()
    }
}

/// One row of the embedded list JSON.
///
/// `anime_title` is a string for almost every entry, but MyAnimeList stores
/// numeric titles as a bare number ("86" for "86 Eighty-Six"), so it is
/// parsed leniently.
#[derive(Debug, Deserialize)]
struct ListItem {
    anime_id: i64,
    anime_title: serde_json::Value,
    #[serde(default)]
    anime_title_eng: Option<String>,
    #[serde(default)]
    anime_num_episodes: Option<i64>,
    #[serde(default)]
    anime_airing_status: i64,
    #[serde(default)]
    anime_start_date_string: Option<String>,
    #[serde(default)]
    status: i64,
    #[serde(default)]
    score: i64,
    #[serde(default)]
    num_watched_episodes: i64,
    #[serde(default)]
    anime_image_path: Option<String>,
    #[serde(default)]
    updated_at: Option<i64>,
}

impl ListItem {
    fn title(&self) -> String {
        match &self.anime_title {
            serde_json::Value::String(text) => text.clone(),
            serde_json::Value::Number(number) => number.to_string(),
            serde_json::Value::Null => String::new(),
            other => other.to_string(),
        }
    }
}

/// Pull the embedded `data-items` JSON out of a list page.
///
/// Returns the number of raw rows and the parsed entries, so the caller can
/// tell "end of the list" from "one odd row failed to parse".
fn parse_list_items(html: &str) -> (usize, Vec<LibraryEntry>) {
    let Some(json) = extract_data_items(html) else {
        return (0, Vec::new());
    };
    let Ok(value) = serde_json::from_str::<Value>(&json) else {
        return (0, Vec::new());
    };
    let Some(items) = value.as_array() else {
        return (0, Vec::new());
    };
    let raw_count = items.len();

    let entries = items
        .iter()
        .filter_map(|item| {
            let raw: ListItem = serde_json::from_value(item.clone()).ok()?;
            let title = raw.title();
            if title.is_empty() {
                return None;
            }
            Some(LibraryEntry {
                anime: Anime {
                    mal_id: raw.anime_id,
                    title,
                    title_english: raw.anime_title_eng.clone(),
                    title_japanese: None,
                    format: item
                        .get("anime_media_type_string")
                        .and_then(Value::as_str)
                        .unwrap_or("TV")
                        .to_string(),
                    episodes: raw.anime_num_episodes,
                    duration: None,
                    airing_status: airing_status(raw.anime_airing_status),
                    year: parse_year(raw.anime_start_date_string.as_deref()),
                    aired_from: parse_date(raw.anime_start_date_string.as_deref()),
                    score: None,
                    cover_url: raw.anime_image_path.clone(),
                },
                status: LibraryStatus::from_code(raw.status),
                score: raw.score,
                episodes_watched: raw.num_watched_episodes,
                updated_at: raw.updated_at,
            })
        })
        .collect();
    (raw_count, entries)
}

fn airing_status(code: i64) -> String {
    match code {
        1 => "Currently Airing".into(),
        2 => "Finished Airing".into(),
        3 => "Not yet aired".into(),
        _ => "Unknown".into(),
    }
}

/// MAL dates look like `01-04-18`.
fn parse_date(date: Option<&str>) -> Option<(i32, i32, i32)> {
    let date = date?;
    let mut parts = date.split('-');
    let month: i32 = parts.next()?.parse().ok()?;
    let day: i32 = parts.next()?.parse().ok()?;
    let year_part: i32 = parts.next()?.parse().ok()?;
    let year = if year_part < 100 {
        2000 + year_part
    } else {
        year_part
    };
    Some((year, month, day))
}

/// MAL dates look like `01-04-18`.
fn parse_year(date: Option<&str>) -> Option<i32> {
    let date = date?;
    let year_part = date.split('-').nth(2)?;
    let year: i32 = year_part.parse().ok()?;
    Some(if year < 100 { 2000 + year } else { year })
}

/// Extract the HTML-escaped `data-items` attribute value.
fn extract_data_items(html: &str) -> Option<String> {
    let start = html.find("data-items=\"")? + "data-items=\"".len();
    let rest = &html[start..];
    let end = rest.find('"')?;
    Some(html_unescape(&rest[..end]))
}

/// Decode the handful of HTML entities MyAnimeList emits inside the attribute.
pub fn html_unescape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(idx) = rest.find('&') {
        out.push_str(&rest[..idx]);
        rest = &rest[idx..];
        let Some(semi) = rest.find(';').filter(|s| *s <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let entity = &rest[1..semi];
        let decoded = match entity {
            "quot" => Some('"'),
            "amp" => Some('&'),
            "apos" => Some('\''),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "nbsp" => Some(' '),
            other => other
                .strip_prefix('#')
                .and_then(|n| {
                    if let Some(hex) = n.strip_prefix('x').or_else(|| n.strip_prefix('X')) {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        n.parse::<u32>().ok()
                    }
                })
                .and_then(char::from_u32),
        };
        match decoded {
            Some(ch) => {
                out.push(ch);
                rest = &rest[semi + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

/// Parse the "Related Entries" block of an anime page (fallback backend).
pub fn parse_related_entries(html: &str) -> Vec<(i64, RelationKind)> {
    let Some(start) = html.find("id=\"related_entries\"") else {
        return Vec::new();
    };
    let section = &html[start..];
    let end = section[1..]
        .find("<h2")
        .map(|i| i + 1)
        .unwrap_or(section.len());
    let section = &section[..end];

    let mut results: Vec<(i64, RelationKind)> = Vec::new();

    // Tile layout: <div class="relation">Sequel (TV)</div> ... <div class="title">...<a href="URL">NAME
    let mut cursor = 0usize;
    while let Some(rel_pos) = section[cursor..].find("class=\"relation\"") {
        let rel_start = cursor + rel_pos;
        let Some(div_end) = section[rel_start..].find("</div>") else {
            break;
        };
        let rel_text = slice(section, rel_start, rel_start + div_end);
        let relation = relation_label(rel_text);

        let Some(title_pos) = section[rel_start..].find("class=\"title\"") else {
            break;
        };
        let title_start = rel_start + title_pos;
        let Some(link) = first_link(slice(section, title_start, title_start + 400)) else {
            cursor = rel_start + div_end;
            continue;
        };
        cursor = title_start + 200;

        if let Some((mal_id, is_anime)) = mal_id_from_url(&link.0)
            && is_anime
            && relation != RelationKind::Adaptation
        {
            results.push((mal_id, relation));
        }
    }

    // Table layout: <td ...>Side Story:</td> ... <li><a href="URL">NAME</a> (Special)</li>
    let mut cursor = 0usize;
    while let Some(td_pos) = section[cursor..].find("nowrap\">") {
        let td_start = cursor + td_pos;
        let Some(close) = section[td_start..].find("</td>") else {
            break;
        };
        let label_text = slice(section, td_start, td_start + close);
        let relation = relation_label(label_text);
        cursor = td_start + close;

        let Some(block_end) = section[cursor..].find("</tr>").map(|i| cursor + i) else {
            break;
        };
        let block = slice(section, cursor, block_end);
        for link in all_links(block) {
            if let Some((mal_id, is_anime)) = mal_id_from_url(&link)
                && is_anime
                && relation != RelationKind::Adaptation
            {
                results.push((mal_id, relation));
            }
        }
        cursor = block_end;
    }

    results.dedup();
    results
}

/// Safe substring: clamps the end so a malformed page cannot panic.
fn slice(text: &str, start: usize, end: usize) -> &str {
    let start = start.min(text.len());
    let end = end.min(text.len());
    if start >= end {
        return "";
    }
    &text[start..end]
}

fn relation_label(text: &str) -> RelationKind {
    // Skip the opening tag, then take the text before the media type.
    let after_tag = match text.find('>') {
        Some(index) => &text[index + 1..],
        None => text,
    };
    let label = after_tag
        .split('(')
        .next()
        .unwrap_or("")
        .split('<')
        .next()
        .unwrap_or("")
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_string();
    RelationKind::parse(&label)
}

/// Extract the first `<a href="URL">TEXT</a>` from a fragment.
fn first_link(fragment: &str) -> Option<(String, String)> {
    let href = fragment.find("href=\"")? + "href=\"".len();
    let url_end = fragment[href..].find('"')? + href;
    let url = fragment[href..url_end].to_string();
    let after = &fragment[url_end..];
    let text_start = after.find('>')? + 1;
    let text_end = after[text_start..].find('<').map(|i| i + text_start)?;
    let text = after[text_start..text_end].trim().to_string();
    Some((url, text))
}

fn all_links(fragment: &str) -> Vec<String> {
    let mut links = Vec::new();
    let mut cursor = 0usize;
    while let Some(pos) = fragment[cursor..].find("href=\"") {
        let href = cursor + pos + "href=\"".len();
        let Some(end) = fragment[href..].find('"') else {
            break;
        };
        links.push(fragment[href..href + end].to_string());
        cursor = href + end;
    }
    links
}

/// `(mal_id, is_anime)` for a MAL link.
fn mal_id_from_url(url: &str) -> Option<(i64, bool)> {
    let (kind, rest) = if let Some(rest) = url.split("/anime/").nth(1) {
        ("anime", rest)
    } else {
        let rest = url.split("/manga/").nth(1)?;
        ("manga", rest)
    };
    let id: i64 = rest.split(['/', '?']).next()?.parse().ok()?;
    Some((id, kind == "anime"))
}
