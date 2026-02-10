mod engine;
pub mod html;
pub mod renderer;
pub mod site;
pub mod toc;
pub mod view_model;

use std::fs;
use std::path::Path;

use fs_extra::dir::create_all;
use petgraph::dot::Dot;

use crate::compiler::Compiler;
use crate::export::ssg::renderer::Renderer;
// ::renderer_vold::Renderer;
// use crate::export::ssg::renderer::Renderer;
use crate::export::ssg::site::{SiteBuilder, SiteConfig};

pub struct Config {
    site_config: SiteConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            site_config: SiteConfig::default(),
        }
    }
}

pub struct StaticSiteGenerator {
    pub compiler: Compiler,
    pub site_builder: SiteBuilder,
    pub renderer: Renderer,
    pub config: Config,
}

impl Default for StaticSiteGenerator {
    fn default() -> Self {
        Self {
            compiler: Compiler::default(),
            site_builder: SiteBuilder::default(),
            renderer: Renderer::default(),
            config: Config::default(),
        }
    }
}

impl StaticSiteGenerator {
    pub fn generate<P: AsRef<Path>>(&mut self, d_org: P) -> std::io::Result<String> {
        tracing::info!("prepare output director ...");
        let output_directory = Path::new(&self.config.site_config.output_directory);
        if output_directory.exists() {
            let now_utc = chrono::Utc::now();
            let created_ts = now_utc.format("%Y%m%dT%H%M%SZ").to_string();
            let backup_directory = format!("{}.backup_{}", output_directory.display(), created_ts);
            tracing::info!(
                "  backup {} -> {}",
                output_directory.display(),
                backup_directory
            );
            fs::rename(output_directory, backup_directory)?;
        }
        let _ = create_all(output_directory, true);

        tracing::info!("compile ...");
        let d_org = d_org.as_ref();
        let section = self
            .compiler
            .compile_section(d_org)
            .expect("NO document compiled");

        let g = section.build_graph();
        let g_dot = Dot::new(&g.graph);
        tracing::debug!("Basic DOT format:\n{:?}\n", g_dot);
        tracing::debug!("{:#?}", g.graph);

        tracing::info!("build site ...");
        let site = self
            .site_builder
            .build(&section)
            .expect("site_builder.build() failed");

        tracing::info!("render site ...");
        self.renderer.render_site(&site);

        Ok(String::from("todo"))
    }
}
