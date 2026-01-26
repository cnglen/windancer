use clap::{Parser, Subcommand};
use clap_complete::Shell;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, author, about)]
pub struct Cli {
    /// Root directory of project
    #[arg(short = 'r', long, default_value = ".")]
    pub root: PathBuf,

    /// Config file path of project
    #[arg(short = 'c', long, default_value = "config.toml")]
    pub config: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Init the project
    Init {
        /// name of the project.
        #[arg(default_value = ".")]
        name: String,
    },

    /// Build the site. Generate `dist` directory used for http server
    Build {
        #[arg(short = 'o', long)]
        output_dir: Option<PathBuf>,
    },

    /// Serve the site. Rebuild and reload on change automatically
    Serve {
        #[arg(short = 'p', long, default_value_t = 1111)]
        port: u16,
    },

    /// Generate shell completion
    Completion {
        /// Shell to generate completion for
        #[arg(value_enum)]
        shell: Shell,
    },
}
