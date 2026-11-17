//! Non-interactive commands.

use std::path::Path;

use crate::cli::{Cli, Command, ExportFormat};
use crate::config::Config;
use crate::domain::cluster::{RelationGraph, build_clusters};
use crate::engine::Engine;
use crate::error::{AnirelError, Result};
use crate::output::{self, FranchiseReport};

/// Run the requested command.
pub async fn dispatch(cli: &Cli, config: &Config) -> Result<i32> {
    match &cli.command {
        Some(Command::Tui) => crate::tui::run(config.clone(), engine(config).await?, cli.refresh)
            .await
            .map(|_| 0)
            .map_err(|e| AnirelError::Other(e.to_string())),
        Some(Command::Related { query, hops }) => related(cli, config, query, *hops).await,
        Some(Command::Gaps { watched_only }) => gaps(cli, config, *watched_only).await,
        Some(Command::Export { format, output }) => export(cli, config, *format, output).await,
        Some(Command::Search { query }) => search(cli, query).await,
        Some(Command::Update { force }) => update(cli, *force).await,
        None => {
            if crate::sys::stdout_is_tty() {
                crate::tui::run(config.clone(), engine(config).await?, cli.refresh)
                    .await
                    .map(|_| 0)
                    .map_err(|e| AnirelError::Other(e.to_string()))
            } else {
                println!(
                    "anirel - find the related anime you have not watched yet\n\n\
                     Try:\n  \
                     anirel --user NAME            open the interactive interface\n  \
                     anirel related \"Yuru Camp\"    show everything related to a title\n  \
                     anirel gaps --user NAME        list every missing related entry\n  \
                     anirel --help                  show every option"
                );
                Ok(0)
            }
        }
    }
}

async fn engine(config: &Config) -> Result<Engine> {
    Ok(Engine::new(
        config.cache_dir.clone(),
        config.media_ttl,
        config.library_ttl,
    ))
}

async fn load_library(config: &Config, cli: &Cli) -> Result<crate::domain::model::Library> {
    let engine = engine(config).await?;
    let cached = if !cli.refresh {
        engine.cache.read_library(&config.user, config.library_ttl)
    } else {
        None
    };
    if let Some(library) = cached {
        return Ok(library);
    }
    if !cli.json && crate::sys::stdout_is_tty() {
        eprint!("🌸 Fetching MyAnimeList library for {}…", config.user);
    }
    let library = engine
        .library_with_progress(&config.user, true, |count| {
            if !cli.json && crate::sys::stdout_is_tty() {
                eprint!(
                    "\r🌸 Fetching MyAnimeList library for {}… ({} titles)",
                    config.user, count
                );
            }
        })
        .await?;
    if !cli.json && crate::sys::stdout_is_tty() {
        eprintln!(
            "\r🌸 Fetched {} titles for {}.           ",
            library.entries.len(),
            config.user
        );
    }
    Ok(library)
}

/// `anirel related <query>`
async fn related(cli: &Cli, config: &Config, query: &str, hops: usize) -> Result<i32> {
    let library = load_library(config, cli).await?;
    let engine = engine(config).await?;

    let candidates = crate::engine::search_titles(query).await?;
    let seed = candidates
        .iter()
        .find(|a| !a.title.is_empty())
        .ok_or_else(|| AnirelError::Other(format!("no anime found for '{query}'")))?;

    let mut graph = RelationGraph::new();
    engine.resolve_into(&[seed.mal_id], &mut graph).await?;
    engine.expand(&mut graph, &[seed.mal_id], hops).await?;

    let clusters = build_clusters(&graph, &library);
    let Some(cluster) = clusters
        .iter()
        .find(|c| c.members.iter().any(|m| m.anime.mal_id == seed.mal_id))
    else {
        return Err(AnirelError::Other(format!(
            "no related entries found for '{query}'"
        )));
    };

    // Deep pass: merge in MyAnimeList's own Related Entries so the result
    // matches what the user sees on the site.
    let members = deep_pass_with_mal(&engine, &mut graph, cluster).await;

    let report = output::report_from_members(cluster.name.clone(), members, &library);
    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&output::json_franchise(&report, &config.user))
                .unwrap_or_default()
        );
    } else {
        print!("{}", output::format_franchise(&report));
    }
    Ok(0)
}

/// Fetch MyAnimeList's own Related Entries for every title in the cluster and
/// merge in anything the relation graph did not already know about.
async fn deep_pass_with_mal(
    engine: &Engine,
    graph: &mut RelationGraph,
    cluster: &crate::domain::cluster::Cluster,
) -> Vec<(
    crate::domain::model::Anime,
    crate::domain::relation::RelationKind,
)> {
    use crate::domain::relation::RelationKind;

    let mut members: Vec<(crate::domain::model::Anime, RelationKind)> = cluster
        .members
        .iter()
        .map(|member| (member.anime.clone(), member.relation))
        .collect();

    let client = crate::infra::mal_web::MalWebClient::new();
    let mut extra: Vec<(i64, RelationKind)> = Vec::new();

    for member in &cluster.members {
        let Ok(relations) = client.fetch_related(member.anime.mal_id).await else {
            continue;
        };
        for (target, kind) in relations {
            let known = members.iter().any(|(a, _)| a.mal_id == target)
                || extra.iter().any(|(id, _)| *id == target);
            if !known {
                extra.push((target, kind));
            }
        }
    }

    if extra.is_empty() {
        return members;
    }

    let ids: Vec<i64> = extra.iter().map(|(id, _)| *id).collect();
    if engine.resolve_into(&ids, graph).await.is_ok() {
        for (id, kind) in extra {
            if let Some(anime) = graph.media(id) {
                members.push((anime.clone(), kind));
            }
        }
    }
    members
}

/// `anirel gaps`
async fn gaps(cli: &Cli, config: &Config, watched_only: bool) -> Result<i32> {
    let library = load_library(config, cli).await?;
    let engine = engine(config).await?;
    let (reports, _graph) = scan_library(&engine, &library).await?;

    let reports = if watched_only {
        reports
            .into_iter()
            .filter(|r| r.owned() > 0)
            .collect::<Vec<_>>()
    } else {
        reports
    };

    if cli.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&output::json_gaps(&reports, &config.user))
                .unwrap_or_default()
        );
    } else {
        print!("{}", output::format_gaps(&reports, &config.user));
    }
    Ok(0)
}

/// `anirel export`
async fn export(
    cli: &Cli,
    config: &Config,
    format: ExportFormat,
    destination: &Path,
) -> Result<i32> {
    let library = load_library(config, cli).await?;
    let engine = engine(config).await?;
    let (reports, _graph) = scan_library(&engine, &library).await?;

    let content = match format {
        ExportFormat::Markdown => output::markdown_report(&reports, &config.user),
        ExportFormat::Csv => output::csv_report(&reports),
    };

    if destination == Path::new("-") {
        print!("{content}");
    } else {
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(destination, content)?;
        eprintln!("wrote {}", destination.display());
    }
    Ok(0)
}

/// `anirel search`
async fn search(cli: &Cli, query: &str) -> Result<i32> {
    let results = crate::engine::search_titles(query).await?;
    if cli.json {
        let payload: Vec<_> = results
            .iter()
            .map(|a| {
                serde_json::json!({
                    "mal_id": a.mal_id,
                    "title": a.title,
                    "title_english": a.title_english,
                    "title_japanese": a.title_japanese,
                    "format": a.format,
                    "episodes": a.episodes,
                    "year": a.year,
                    "url": a.url(),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        );
    } else {
        for anime in results {
            println!(
                "{:>8}  {:<40} {} ({}, {} ep, {})",
                anime.mal_id,
                anime.display_title(),
                anime.year.map(|y| y.to_string()).unwrap_or("?".into()),
                anime.format,
                anime.episodes.map(|e| e.to_string()).unwrap_or("?".into()),
                anime.airing_status
            );
        }
    }
    Ok(0)
}

/// `anirel update`
async fn update(cli: &Cli, force: bool) -> Result<i32> {
    let report = if cli.json {
        crate::infra::update::run(force).await?
    } else {
        crate::infra::update::run_with_progress(force, |status| {
            eprintln!("🌸 {status}");
        })
        .await?
    };

    if cli.json {
        let payload = serde_json::json!({
            "current": report.current,
            "latest": report.latest,
            "updated": report.updated,
            "target": report.target.map(|p| p.display().to_string()),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&payload).unwrap_or_default()
        );
    } else if report.updated {
        let dest = report
            .target
            .as_ref()
            .map(|p| format!(" ({})", p.display()))
            .unwrap_or_default();
        println!(
            "✨ Updated anirel v{} → v{}{}",
            report.current, report.latest, dest
        );
    } else if report.current == report.latest {
        println!("✔ anirel v{} is up to date", report.current);
    } else {
        println!(
            "ℹ anirel v{} is newer than the latest release (v{})",
            report.current, report.latest
        );
    }
    Ok(0)
}

/// Resolve the whole library and build the franchise reports.
async fn scan_library(
    engine: &Engine,
    library: &crate::domain::model::Library,
) -> Result<(Vec<FranchiseReport>, RelationGraph)> {
    let mut graph = RelationGraph::new();
    let ids: Vec<i64> = library.entries.iter().map(|e| e.anime.mal_id).collect();

    let total = ids.len();
    let mut done = 0usize;
    for chunk in ids.chunks(crate::infra::anilist::BATCH_SIZE) {
        let results = engine.resolve_batch(chunk).await?;
        for result in results.into_iter().flatten() {
            graph.insert_media(result.anime.clone());
            for (target, kind) in &result.relations {
                graph.add_edge(result.anime.mal_id, *target, *kind);
            }
        }
        done += chunk.len();
        eprintln!("\rscanning {done}/{total}…");
    }
    eprintln!("\rscanning {done}/{total}… expanding…");

    engine.expand(&mut graph, &ids, 2).await?;
    eprintln!("\rdone.");

    let clusters = build_clusters(&graph, library);
    let reports = clusters
        .iter()
        .map(|cluster| output::report_from_cluster(cluster, library))
        .collect();
    Ok((reports, graph))
}
