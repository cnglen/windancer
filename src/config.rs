#![allow(warnings)]

use config::{Config, File};
use serde::Deserialize;

use crate::compiler::CompilerConfig;
use crate::export::ssg::renderer::RendererConfig;

#[derive(Debug, Deserialize)]
pub(crate) struct WindancerConfig {
    pub general: General,
    pub compiler: CompilerConfig,
    pub renderer: RendererConfig,
}

#[derive(Debug, Deserialize)]
pub(crate) struct General {
    pub input_directory: String,
    pub tracing_max_level: String,
}

#[derive(Debug, Deserialize)]
struct Ssg {
    output_directory: String,
}

pub(crate) fn load_config() -> Result<WindancerConfig, config::ConfigError> {
    let builder = Config::builder().add_source(File::with_name("config").required(false));

    let config = builder.build()?;
    config.try_deserialize()
}
