mod cli;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use lsf::tui::{
    app::{AppConfig, AppRuntime, resolve_cache_path},
    keymap::Keymap,
    run_with_options,
};
use std::{
    io::{self, Write},
    path::PathBuf,
};
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    let cli = Cli::parse();

    let builder = tracing_subscriber::fmt();
    if let Ok(filter) = EnvFilter::try_from_default_env() {
        builder.with_env_filter(filter).init();
    } else {
        builder.with_max_level(cli.verbosity.tracing_level()).init();
    }

    let runtime = AppRuntime::start(AppConfig {
        path: cli.path,
        cache_path: resolve_cache_path(&cli.cache_path)?,
        refresh: cli.refresh,
    })?;

    if cli.tui {
        let keymap = load_keymap(cli.keymap)?;
        let tui_result = run_with_options(&runtime, !cli.no_quit_confirm, keymap);
        runtime.shutdown()?;
        return tui_result;
    }

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut line = String::new();
        let read = io::stdin().read_line(&mut line)?;
        if read == 0 {
            eprintln!("EOF");
            break;
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        } else if line == "/bye" {
            break;
        }

        runtime.record_history(line)?;

        match runtime.search(line.to_string()) {
            Ok(path_set) => {
                for (i, path) in path_set.results.into_iter().enumerate() {
                    println!("[{i}] {:?} {:?}", path.path, path.metadata);
                }
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    runtime.shutdown()
}

/// Resolve and load the keymap. Explicit `--keymap` path is required to exist;
/// the default path is tried silently and falls back to built-in defaults.
fn load_keymap(explicit: Option<PathBuf>) -> Result<Keymap> {
    if let Some(path) = explicit {
        return Keymap::load(&path);
    }
    let default_path = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".cardinal").join("lsf-keys.toml"))
        .unwrap_or_default();
    Keymap::load(&default_path)
}
