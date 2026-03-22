//! org -> html
// #![allow(warnings)]
use clap::Parser;
use export::ssg::StaticSiteGenerator;
use tracing_subscriber::FmtSubscriber;

mod compiler;
mod config;
mod constants;
mod export;

#[derive(Parser)]
#[command(name = "winancer")]
#[command(version = "0.1")]
#[command(about = "Render a org file to html", long_about = None)]
struct Cli {
    /// Input directory
    #[arg(short = 'i', long)]
    input_directory: Option<String>,

    /// Output path of html file or input directory
    #[arg(short = 'o', long)]
    output: Option<String>,
}

fn load_config() -> Result<config::WindancerConfig, ::config::ConfigError> {
    let builder =
        ::config::Config::builder().add_source(::config::File::with_name("config").required(false));
    let config = builder.build()?;

    config
        .try_deserialize()
        .map(|mut e: config::WindancerConfig| {
            e.update(true);
            e
        })
}

fn main() {
    let mut config = load_config().expect("read config");
    let args = Cli::parse();
    if let Some(input_directory) = args.input_directory {
        config.update_input_directory(input_directory);
    }

    let max_level = match config.general.tracing_max_level.as_str() {
        "error" => tracing::Level::ERROR,
        "warn" => tracing::Level::WARN,
        "info" => tracing::Level::INFO,
        "debug" => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };
    let subscriber = FmtSubscriber::builder().with_max_level(max_level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("set global subscripber failed");
    tracing::info!("config={:#?}", config);

    let mut ssg = StaticSiteGenerator::new(config.compiler, config.ssg);
    let _ = ssg.generate(config.general.input_directory);
}
