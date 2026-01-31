// remove this file: compiler.compile() -> Vec<AST> -> Exporter -> Target
use std::fs;

use walkdir::WalkDir;

use crate::compiler::ast_builder::AstBuilder;
use crate::compiler::parser::OrgParser;
use crate::compiler::parser::config::{OrgParserConfig, OrgUseSubSuperscripts};
use crate::export::ssg::html::{HtmlRenderer, RenderConfig};

pub struct Engine {
    parser: OrgParser,
    ast_builder: AstBuilder,
    renderer: HtmlRenderer,
}

impl Engine {
    pub fn new() -> Self {
        let config =
            OrgParserConfig::default().with_use_sub_superscripts(OrgUseSubSuperscripts::Brace);
        let parser = OrgParser::new(config);
        let ast_builder = AstBuilder::new();
        let renderer = HtmlRenderer::new(RenderConfig::default());
        Self {
            parser,
            ast_builder,
            renderer,
        }
    }

    pub fn org2html(
        &mut self,
        f_org: &str,
        f_html: &str,
        debug: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let syntax_tree = self.parser.parse(f_org);
        tracing::trace!("syntax_tree:{:#?}", syntax_tree);

        let ast = self.ast_builder.build(&syntax_tree, f_org).expect("build");
        let html = self.renderer.render_org_file(&ast);

        fs::write(f_html, html)?;
        tracing::info!("{} -> {} done", f_org, f_html);

        match debug {
            0 => {}
            1 => {
                let f_ast = f_html.replace(".html", "_ast.json");
                fs::write(&f_ast, format!("{:#?}", ast))?;
                tracing::info!("  - AST:         {f_ast}");
            }
            _ => {
                let f_syntax_tree = f_html.replace(".html", "_syntax_tree.json");
                let f_ast = f_html.replace(".html", "_ast.json");
                fs::write(&f_syntax_tree, format!("{:#?}", syntax_tree))?;
                fs::write(&f_ast, format!("{:#?}", ast))?;
                tracing::info!("  - Syntax tree: {f_syntax_tree}");
                tracing::info!("  - AST:         {f_ast}");
            }
        }

        Ok(())
    }

    pub fn dir2html(
        &mut self,
        d_org: &str,
        d_html: &str,
        debug: u8,
    ) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(d_html)?;

        for entry in WalkDir::new(d_org).into_iter().filter_map(|s| s.ok()) {
            if entry
                .file_name()
                .to_str()
                .expect("to str")
                .ends_with(".org")
                && !entry.file_name().to_str().expect("to str").starts_with(".")
            {
                let f_org = entry.path().to_str().expect("to str");
                let f_html = f_org.replacen(d_org, d_html, 1).replace(".org", ".html");
                tracing::debug!("{:?} -> {:?}", entry.path(), &f_html);
                self.org2html(f_org, &f_html, debug)?;
            }
        }

        Ok(())
    }
}
