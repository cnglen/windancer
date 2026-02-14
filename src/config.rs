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

impl WindancerConfig {
    pub(crate) fn update(&mut self, force: bool) {
        if !force {
            if self.ssg.renderer.input_directory.as_os_str().is_empty() {
                self.ssg.renderer.input_directory = self.general.input_directory.clone();
            }

            if self.ssg.renderer.output_directory.as_os_str().is_empty() {
                self.ssg.renderer.output_directory = self.ssg.output_directory.clone();
            }

            if self.ssg.site.output_directory.as_os_str().is_empty() {
                self.ssg.site.output_directory = self.ssg.output_directory.clone();
            }
        } else {
            self.ssg.renderer.input_directory = self.general.input_directory.clone();
            self.ssg.renderer.output_directory = self.ssg.output_directory.clone();
            self.ssg.site.output_directory = self.ssg.output_directory.clone();
        }
    }

    pub fn update_input_directory(&mut self, input_directory: String) {
        self.general.input_directory = input_directory.clone().into();
        self.ssg.renderer.input_directory = input_directory.into();
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub(crate) struct General {
    pub input_directory: PathBuf,
    pub tracing_max_level: String,
}
