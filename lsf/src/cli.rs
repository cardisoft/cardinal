use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
pub struct Cli {
    #[clap(long, default_value = "false")]
    /// Open enabled, cache was ignored and filesystem will be rewalked.
    pub refresh: bool,
    #[clap(long, default_value = "/")]
    pub path: PathBuf,
    #[command(flatten)]
    pub verbosity: clap_verbosity_flag::Verbosity,
}
