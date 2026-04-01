//! org -> html
// #![allow(warnings)]
use std::path::Path;

use clap::Parser;
use export::ssg::StaticSiteGenerator;
use tracing_subscriber::FmtSubscriber;

mod compiler;
mod config;
mod constants;
mod export;

#[derive(Parser)]
#[command(version)]
#[command(author)]
#[command(name = "windancer")]
#[command(about = "A toolkit for parsing and rendering org-mode.", long_about = None)]
struct Cli {
    /// Input directory, which contains org-mode files
    #[arg(short = 'i', long)]
    input_directory: Option<String>,

    /// Output directory for static site generator(SSG)
    #[arg(short = 'o', long)]
    ssg_output_directory: Option<String>,

    /// Config file in TOML format
    #[arg(short = 'c', long)]
    config_file: Option<String>,
}

fn main() {
    let args = Cli::parse();

    let default_config = ::config::File::from_str(
        include_str!("../config/default.toml"),
        ::config::FileFormat::Toml,
    );

    let mut builder = ::config::Config::builder().add_source(default_config);

    if let Some(config_file) = args.config_file {
        let config_file_path = Path::new(&config_file);
        if !config_file_path.exists() {
            panic!("Error: args of config-file(-c, --config-file) '{config_file}' does't exists");
        }
        builder = builder.add_source(::config::File::with_name(&config_file).required(false));
    }
    if let Some(input_directory) = args.input_directory {
        builder = builder
            .set_override("general.input_directory", input_directory)
            .unwrap();
    }
    if let Some(ssg_output_directory) = args.ssg_output_directory {
        builder = builder
            .set_override("general.output_directory.ssg", ssg_output_directory)
            .unwrap();
    }
    let config = builder.build().expect("builder.build() failed");

    let config: config::WindancerConfig = config
        .try_deserialize()
        .expect("A config:WindancerConfig should be loaded from default/sources/overrides");

    let max_level = match config.general.tracing_max_level.as_str() {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        "debug" => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };
    let subscriber = FmtSubscriber::builder().with_max_level(max_level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("set global subscripber failed");
    tracing::debug!("config={:#?}", config);

    let mut ssg = StaticSiteGenerator::new(
        config.compiler,
        config.ssg,
        config.general.input_directory.to_str().unwrap(),
        config
            .general
            .output_directory
            .get("ssg")
            .expect("output should have a key named 'ssg'")
            .to_str()
            .unwrap(),
    );
    let _ = ssg.generate(config.general.input_directory);
}
