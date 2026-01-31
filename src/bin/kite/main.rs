use std::path::PathBuf;

use clap::Parser;
use cli::Cli;

mod cli;

fn main() {
    let cli = Cli::parse();

    let cli_dir: PathBuf = cli.root.canonicalize().unwrap_or_else(|e| {
        tracing::error!(
            "Could not find canonical path of root dir: {}",
            cli.root.display()
        );
        tracing::error!("{}", e);
        std::process::exit(1);
    });

    tracing::debug!("cli_dir={}", cli_dir.display());
}
