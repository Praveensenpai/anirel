//! Report building and formatting (plain text, JSON, Markdown, CSV).

use serde_json::{Value, json};
use unicode_width::UnicodeWidthStr;

use crate::domain::cluster::Cluster;
use crate::domain::model::Library;

/// One title inside a franchise, flattened for reporting.
#[derive(Debug, Clone)]
pub struct MemberInfo {
    pub mal_id: i64,
    pub title: String,
    pub title_english: Option<String>,
    pub title_japanese: Option<String>,
    pub format: String,
    pub episodes: Option<i64>,
    pub year: Option<i32>,
    pub airing_status: String,
    pub relation: String,
    pub in_library: bool,
    pub status: Option<String>,
    pub score: Option<i64>,
    /// Missing from the library, or present but unwatched.
    pub needs_action: bool,
}

impl MemberInfo {
    pub fn url(&self) -> String {
        format!("https://myanimelist.net/anime/{}", self.mal_id)
    }

    pub fn display_title(&self) -> &str {
        if !self.title.trim().is_empty() {
            &self.title
        } else if let Some(english) = self
            .title_english
            .as_deref()
            .filter(|s| !s.trim().is_empty())
        {
            english
        } else {
            &self.title
        }
    }
}

/// A franchise with its members.
#[derive(Debug, Clone)]
pub struct FranchiseReport {
    pub name: String,
    pub members: Vec<MemberInfo>,
}

impl FranchiseReport {
    pub fn owned(&self) -> usize {
        self.members.iter().filter(|m| m.in_library).count()
    }

    pub fn total(&self) -> usize {
        self.members.len()
    }

    /// Titles that are absent from the library or still unwatched.
    pub fn missing(&self) -> Vec<&MemberInfo> {
        self.missing_with(false)
    }

    /// Like [`FranchiseReport::missing`], but optionally keeping the vague
    /// "other" links that MyAnimeList lists as related entries.
    pub fn missing_with(&self, include_other: bool) -> Vec<&MemberInfo> {
        self.members
            .iter()
            .filter(|m| m.needs_action)
            .filter(|m| m.relation != "adaptation")
            .filter(|m| include_other || m.relation != "other")
            .collect()
    }
}

/// Flatten a cluster into a report.
pub fn report_from_cluster(cluster: &Cluster, library: &Library) -> FranchiseReport {
    let members = cluster
        .members
        .iter()
        .map(|member| (member.anime.clone(), member.relation))
        .collect();
    report_from_members(cluster.name.clone(), members, library)
}

/// Build a report from an explicit member list.
pub fn report_from_members(
    name: String,
    members: Vec<(
        crate::domain::model::Anime,
        crate::domain::relation::RelationKind,
    )>,
    library: &Library,
) -> FranchiseReport {
    let members = members
        .into_iter()
        .map(|(anime, relation)| {
            let entry = library.get(anime.mal_id);
            MemberInfo {
                mal_id: anime.mal_id,
                title: anime.title.clone(),
                title_english: anime.title_english.clone(),
                title_japanese: anime.title_japanese.clone(),
                format: anime.format.clone(),
                episodes: anime.episodes,
                year: anime.year,
                airing_status: anime.airing_status.clone(),
                relation: relation.label().to_string(),
                in_library: entry.is_some(),
                status: entry.map(|e| e.status.label().to_string()),
                score: entry.map(|e| e.score).filter(|s| *s > 0),
                needs_action: entry.map(|e| !e.status.is_completed()).unwrap_or(true),
            }
        })
        .collect();
    FranchiseReport { name, members }
}

/// Pad a string to a display width.
fn pad(text: &str, width: usize) -> String {
    let len = text.width();
    if len >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - len))
    }
}

/// Truncate to a display width with an ellipsis.
fn truncate(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let w = ch.to_string().width();
        if used + w > width.saturating_sub(1) {
            break;
        }
        out.push(ch);
        used += w;
    }
    out.push('…');
    out
}

/// Plain-text report for a single franchise.
pub fn format_franchise(report: &FranchiseReport) -> String {
    let missing = report
        .missing_with(true)
        .iter()
        .filter(|m| !m.in_library)
        .count();
    let unwatched = report
        .missing_with(true)
        .iter()
        .filter(|m| m.in_library)
        .count();
    let mut out = String::new();
    let mut summary = format!("{} of {} titles owned", report.owned(), report.total());
    if missing > 0 {
        summary.push_str(&format!(", {missing} missing"));
    }
    if unwatched > 0 {
        summary.push_str(&format!(", {unwatched} unwatched"));
    }
    out.push_str(&format!("{} — {summary}\n", report.name));
    out.push('\n');

    let title_width = report
        .members
        .iter()
        .map(|m| m.display_title().width())
        .max()
        .unwrap_or(10)
        .clamp(10, 44);

    for member in &report.members {
        let glyph = match (member.in_library, member.status.as_deref()) {
            (false, _) => "◌",
            (true, Some("completed")) => "✓",
            (true, Some("watching")) => "▶",
            (true, Some("plan-to-watch")) => "○",
            (true, Some("on-hold")) => "⏸",
            (true, Some("dropped")) => "✗",
            _ => "•",
        };
        let mut meta = vec![member.format.clone()];
        meta.push(match member.episodes {
            Some(episodes) => format!("{episodes} ep"),
            None => "?".into(),
        });
        meta.push(member.year.map(|y| y.to_string()).unwrap_or("?".into()));
        if !member.airing_status.contains("Finished") {
            meta.push("not aired".into());
        }
        if let Some(score) = member.score {
            meta.push(format!("score {score}"));
        }

        let title = pad(&truncate(member.display_title(), title_width), title_width);
        out.push_str(&format!("  {glyph} {title}  {}\n", meta.join(" · ")));
        let detail = if member.in_library {
            format!(
                "      {} · {}",
                member.status.as_deref().unwrap_or(""),
                member.relation
            )
        } else {
            format!("      MISSING · {}", member.relation)
        };
        out.push_str(&format!("{}\n", detail));
    }
    out
}

/// Plain-text report of every missing entry.
pub fn format_gaps(reports: &[FranchiseReport], user: &str) -> String {
    let total: usize = reports.iter().map(|r| r.missing_with(false).len()).sum();
    let absent: usize = reports
        .iter()
        .map(|r| {
            r.missing_with(false)
                .iter()
                .filter(|m| !m.in_library)
                .count()
        })
        .sum();
    let mut out = String::new();
    if total == 0 {
        out.push_str(&format!(
            "Nothing missing — every related entry of {user}'s library is on the list.\n"
        ));
        return out;
    }
    out.push_str(&format!(
        "{total} related {} outstanding for {user}: {absent} not on your list, {} still unwatched\n",
        if total == 1 { "entry" } else { "entries" },
        total - absent,
    ));
    out.push_str(&format!("across {} franchises\n\n", reports.len()));

    for report in reports {
        let missing = report.missing_with(false);
        if missing.is_empty() {
            continue;
        }
        out.push_str(&format!(
            "{}  ({} of {})\n",
            report.name,
            report.owned(),
            report.total()
        ));
        for member in missing {
            let glyph = if member.in_library { "○" } else { "◌" };
            let state = if member.in_library {
                member.status.clone().unwrap_or_default()
            } else {
                "missing".to_string()
            };
            out.push_str(&format!(
                "  {glyph} {:<38} {:<12} {:<12} {}\n",
                truncate(member.display_title(), 38),
                member.relation,
                state,
                member.url()
            ));
        }
        out.push('\n');
    }
    out
}

/// JSON report for a single franchise.
pub fn json_franchise(report: &FranchiseReport, user: &str) -> Value {
    json!({
        "user": user,
        "franchise": report.name,
        "owned": report.owned(),
        "total": report.total(),
        "missing_count": report.missing_with(true).len(),
        "members": report.members.iter().map(member_json).collect::<Vec<_>>(),
    })
}

/// JSON report for every missing entry.
pub fn json_gaps(reports: &[FranchiseReport], user: &str) -> Value {
    let franchises: Vec<Value> = reports
        .iter()
        .filter(|r| !r.missing_with(false).is_empty())
        .map(|report| {
            json!({
                "name": report.name,
                "owned": report.owned(),
                "total": report.total(),
                "missing": report
                    .missing_with(false)
                    .iter()
                    .map(|m| member_json(m))
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    json!({
        "user": user,
        "total_missing": franchises.iter().map(|f| f["missing"].as_array().map(|a| a.len()).unwrap_or(0)).sum::<usize>(),
        "franchises": franchises,
    })
}

fn member_json(member: &MemberInfo) -> Value {
    json!({
        "mal_id": member.mal_id,
        "title": member.title,
        "title_english": member.title_english,
        "title_japanese": member.title_japanese,
        "format": member.format,
        "episodes": member.episodes,
        "year": member.year,
        "airing_status": member.airing_status,
        "relation": member.relation,
        "in_library": member.in_library,
        "status": member.status,
        "score": member.score,
        "missing": !member.in_library,
        "needs_action": member.needs_action,
        "url": member.url(),
    })
}

/// Markdown report for the whole library.
pub fn markdown_report(reports: &[FranchiseReport], user: &str) -> String {
    let total: usize = reports.iter().map(|r| r.missing_with(false).len()).sum();
    let mut out = format!("# Missing related anime — {user}\n\n");
    out.push_str(&format!(
        "{} missing related entries across {} franchises.\n\n",
        total,
        reports.len()
    ));
    out.push_str("| Franchise | Missing | Owned | Title | Relation | Year | Link |\n");
    out.push_str("|---|---|---|---|---|---|---|\n");
    for report in reports {
        for member in report.missing_with(false) {
            out.push_str(&format!(
                "| {} | ◌ {} | {}/{} | {} | {} | {} | [MAL]({}) |\n",
                report.name,
                member.display_title(),
                report.owned(),
                report.total(),
                member.display_title(),
                member.relation,
                member.year.map(|y| y.to_string()).unwrap_or("?".into()),
                member.url()
            ));
        }
    }
    out
}

/// CSV report for the whole library.
pub fn csv_report(reports: &[FranchiseReport]) -> String {
    let mut out = String::from("franchise,title,relation,format,episodes,year,mal_id,url\n");
    for report in reports {
        for member in report.missing_with(false) {
            out.push_str(&format!(
                "{},{},{},{},{},{},{},{}\n",
                csv_cell(&report.name),
                csv_cell(member.display_title()),
                csv_cell(&member.relation),
                csv_cell(&member.format),
                member.episodes.map(|e| e.to_string()).unwrap_or_default(),
                member.year.map(|y| y.to_string()).unwrap_or_default(),
                member.mal_id,
                member.url()
            ));
        }
    }
    out
}

fn csv_cell(text: &str) -> String {
    if text.contains(',') || text.contains('"') {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}
