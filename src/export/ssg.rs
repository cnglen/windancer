pub mod css_generator;
mod engine;
pub mod renderer;
pub mod site;
pub mod toc;
pub mod view_model;
use std::fs;
use std::path::Path;
use std::time::Instant;

use fs_extra::dir::create_all;
use petgraph::dot::Dot;
use serde::Deserialize;

use crate::compiler::{Compiler, CompilerConfig};
use crate::export::ssg::renderer::{Renderer, RendererConfig};
use crate::export::ssg::site::{SiteBuilder, SiteConfig};

pub struct StaticSiteGenerator {
    pub compiler: Compiler,
    pub site_builder: SiteBuilder,
    pub renderer: Renderer,
}

impl Default for StaticSiteGenerator {
    fn default() -> Self {
        Self {
            compiler: Compiler::default(),
            site_builder: SiteBuilder::default(),
            renderer: Renderer::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SsgConfig {
    pub site: SiteConfig,
    pub renderer: RendererConfig,
}

impl StaticSiteGenerator {
    pub fn generate<P: AsRef<Path>>(&mut self, d_org: P) -> std::io::Result<String> {
        tracing::info!("prepare output directory ...");
        let d_org = fs::canonicalize(d_org.as_ref())?;
        let output_directory = &self.site_builder.output_directory;
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
        let f_css = output_directory.join("encre.css");

        tracing::info!("compile ...");
        // let d_org = d_org.as_ref();
        let section = self
            .compiler
            .compile_section(d_org.as_path(), d_org.as_path())
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

        tracing::info!("  generate css ...");
        let _ = css_generator::generate(&f_css);

        tracing::info!(
            "done, please visit `{}` to check the html files",
            self.renderer.output_directory.display()
        );
        tracing::info!(
            r#"You may start http server, e.g, using python or static-web-server
- python -m http.server -d {} 8889
- static-web-server -d {} -p 8889
Then visit http://127.0.0.1:8889
"#,
            self.renderer.output_directory.display(),
            self.renderer.output_directory.display()
        );
        Ok(String::from("todo"))
    }

    #[allow(dead_code)]
    pub fn generate_html<P: AsRef<Path>>(&mut self, f_org: P) -> String {
        let start = Instant::now();
        let doc = self
            .compiler
            .compile_file(f_org.as_ref(), f_org.as_ref().parent().unwrap())
            .expect("compile org to Document(AST)");
        let duration = start.elapsed();
        tracing::info!("windancer@parser           : {:?}", duration);

        let start = Instant::now();
        let page = self.site_builder.build_document(&doc);
        let duration = start.elapsed();
        tracing::info!("windancer@site_builder     : {:?}", duration);

        let start = Instant::now();
        let html = self.renderer.render_page_inner(&page);
        let duration = start.elapsed();
        tracing::info!("windancer@renderer         : {:?}", duration);
        html
    }

    pub fn new<'a>(
        compiler_config: CompilerConfig,
        ssg_config: SsgConfig,
        input_directory: &'a str,
        output_directory: &'a str,
    ) -> Self {
        let site_config = ssg_config.site;
        let renderer_config = ssg_config.renderer;
        let compiler = Compiler::new(compiler_config);
        let site_builder = SiteBuilder::new(site_config, output_directory);
        let renderer = Renderer::new(renderer_config, input_directory, output_directory);
        Self {
            compiler,
            site_builder,
            renderer,
        }
    }
}
