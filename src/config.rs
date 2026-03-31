use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

use crate::compiler::CompilerConfig;
use crate::export::ssg::SsgConfig;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct WindancerConfig {
    pub general: General,
    pub compiler: CompilerConfig,
    pub ssg: SsgConfig,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct General {
    pub input_directory: PathBuf,
    pub tracing_max_level: String,
    pub output_directory: HashMap<String, PathBuf>,
}
