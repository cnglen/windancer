use std::collections::HashMap;
use std::path::PathBuf;

use serde::Deserialize;

use crate::compiler::CompilerConfig;
use crate::export::ssg::SsgConfig;

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub(crate) struct WindancerConfig {
    pub general: General,
    pub compiler: CompilerConfig,
    pub ssg: SsgConfig,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub(crate) struct General {
    pub input_directory: PathBuf,
    pub tracing_max_level: String,
    pub output_directory: HashMap<String, PathBuf>,
}

#[allow(unused)]
impl General {
    /// standardize the paths
    pub fn standardize_paths(&mut self) -> std::io::Result<()> {
        if !self.input_directory.exists() {
            tracing::error!("`{}` doesn't exist!", self.input_directory.display());
            panic!("An EXISTED input directory should be provided!");
        }

        self.input_directory = std::fs::canonicalize(&self.input_directory)?;
        for value in self.output_directory.values_mut() {
            if !value.exists() {
                std::fs::create_dir_all(&mut *value).expect("create directory");
            }
            *value = std::fs::canonicalize(&value)?;
        }
        Ok(())
    }
}
