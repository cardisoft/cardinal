use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    #[clap(long, default_value = "false")]
    /// Open enabled, cache was ignored and filesystem will be rewalked.
    pub refresh: bool,
    #[clap(long, default_value = "false")]
    /// Launch the ratatui search interface instead of the line-based prompt.
    pub tui: bool,
    #[clap(long, default_value = "false")]
    /// Exit the TUI immediately without a quit confirmation prompt.
    pub no_quit_confirm: bool,
    #[clap(long, default_value = "~/.cardinal/cache.zstd")]
    /// Cache file path. Supports a leading `~/`.
    pub cache_path: PathBuf,
    #[clap(long, default_value = "/")]
    pub path: PathBuf,
    /// Path to a TOML keybinding file. Defaults to ~/.cardinal/lsf-keys.toml
    /// if the file exists; built-in defaults are used otherwise.
    #[clap(long)]
    pub keymap: Option<PathBuf>,
    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity,
}
