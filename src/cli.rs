//! Command line interface definition.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "anirel",
    version,
    about = "Find the related anime you have not watched yet",
    long_about = "anirel maps your MyAnimeList library against the Related Entries graph and \
                  tells you which seasons, OVAs, films and spin-offs you are missing.\n\n\
                  Run without a subcommand to open the interactive TUI when attached to a \
                  terminal, or print a summary when piped.",
    disable_help_subcommand = false
)]
pub struct Cli {
    /// MyAnimeList username.
    #[arg(short, long, global = true, env = "ANIREL_USER")]
    pub user: Option<String>,

    /// Emit machine readable JSON instead of formatted text.
    #[arg(long, global = true)]
    pub json: bool,

    /// Ignore the local cache and refetch everything.
    #[arg(long, global = true)]
    pub refresh: bool,

    /// Subcommand; omit for the interactive interface.
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Open the interactive interface (default when attached to a terminal).
    Tui,

    /// Show every title related to a given anime.
    Related {
        /// Title to look up, e.g. "Yuru Camp".
        query: String,
        /// How many relation hops to follow.
        #[arg(long, default_value_t = 3)]
        hops: usize,
    },

    /// List every related entry missing from your library.
    Gaps {
        /// Only consider titles you have already completed or started.
        #[arg(long)]
        watched_only: bool,
    },

    /// Export the gap report to a file.
    Export {
        /// Output format.
        #[arg(long, value_enum, default_value_t = ExportFormat::Markdown)]
        format: ExportFormat,
        /// Destination file; "-" writes to stdout.
        #[arg(short, long, default_value = "-")]
        output: PathBuf,
    },

    /// Search MyAnimeList for a title.
    Search {
        /// Title to search for.
        query: String,
    },

    /// Check for a newer release and update the binary if needed.
    Update {
        /// Force re-download and install even if already up to date.
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
pub enum ExportFormat {
    Markdown,
    Csv,
}
