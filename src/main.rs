//! Entry point.

use anirel::cli::Cli;
use anirel::config::{self, Config};
use anirel::sys;
use clap::Parser;

fn main() {
    let cli = Cli::parse();

    let user = cli
        .user
        .clone()
        .or_else(config::last_user)
        .unwrap_or_default();

    if user.is_empty() {
        eprintln!(
            "anirel needs a MyAnimeList username.\n\n  \
             anirel --user YOUR_NAME\n  \
             or set ANIREL_USER=YOUR_NAME"
        );
        std::process::exit(2);
    }
    config::remember_user(&user);

    let mut config = Config::new(user);
    if cli.refresh {
        config.library_ttl = std::time::Duration::from_secs(0);
        config.media_ttl = std::time::Duration::from_secs(0);
    }

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("could not start the async runtime: {error}");
            std::process::exit(1);
        }
    };

    let result = runtime.block_on(anirel::commands::dispatch(&cli, &config));
    let code = match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            1
        }
    };

    if !sys::stdout_is_tty() && cli.json {
        // JSON mode already printed a valid document on stdout.
    }
    std::process::exit(code);
}
