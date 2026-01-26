// // SSG
// use std::fs;
// use crate::compiler::parser::syntax::SyntaxNode;
// use std::path::PathBuf;
// use crate::ast::builder::AstBuilder;
// use crate::compiler::parser::{OrgParser, config::OrgParserConfig, config::OrgUseSubSuperscripts};
// use crate::ast::element::Document;
// use crate::renderer::html::{HtmlRenderer, RenderConfig};
// use walkdir::WalkDir;

// // page {path, ast}: page.parse().build_ast().render_html()
// // site: pages

// pub struct Page {
//     pub config: OrgPaserConfig,
//     pub path: PathBuf,
//     pub ast: Document,
//     parser: OrgParser,
//     ast_builder: AstBuilder,
//     html_renderer: HtmlRenderer
// }

// impl Page {
//     fn parse(&self) -> Document {

//     }

// }

// pub struct Engine {
//     parser: OrgParser,
//     ast_builder: AstBuilder,
//     renderer: HtmlRenderer,
// }

// impl SyntaxNode {
//     pub fn build_ast(&self) -> Option<Document> {
//         builder.build(&self)
//     }
// }

// impl Engine {
//     pub fn new() -> Self {
//         let config = OrgParserConfig::default().with_use_sub_superscripts(OrgUseSubSuperscripts::Brace);
//         let parser = OrgParser::new(config);
//         let ast_builder = AstBuilder::new();
//         let renderer = HtmlRenderer::new(RenderConfig::default());
//         Self {
//             parser,
//             ast_builder,
//             renderer,
//         }
//     }

//     // engine.parse(x.org)?.build_ast()?.render_html()?;
//     // enginer.parse_and_build_ast()

//     /// Build the AST, f_org -> SyntaxNode -> Document
//     pub fn build(&self, f_org: &PathBuf) -> Option<Document> {
//         let syntax_tree = {
//             let parser_output = self.parser.parse(f_org);
//             parser_output.syntax()
//         };
//         tracing::trace!("syntax_tree:{:#?}", syntax_tree);

//         tracing::trace!("syntax_tree:{:#?}", syntax_tree);
//         let ast = self.ast_builder.build(&syntax_tree).expect("build");

//         ast
//     }

//     pub fn org2html(
//         &mut self,
//         f_org: &str,
//         f_html: &str,
//         debug: u8,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         let syntax_tree = {
//             let parser_output = self.parser.parse(f_org);
//             parser_output.syntax()
//         };
//         tracing::trace!("syntax_tree:{:#?}", syntax_tree);
//         let ast = self.ast_builder.build(&syntax_tree).expect("build");
//         let html = self.renderer.render_document(&ast);

//         let html_dir = std::path::Path::new(f_html).parent().unwrap();
//         std::fs::create_dir_all(html_dir)?;
//         fs::write(f_html, html)?;
//         tracing::info!("{} -> {} done", f_org, f_html);

//         match debug {
//             0 => {}
//             1 => {
//                 let f_ast = f_html.replace(".html", "_ast.json");
//                 fs::write(&f_ast, format!("{:#?}", ast))?;
//                 tracing::info!("  - AST:         {f_ast}");
//             }
//             _ => {
//                 let f_syntax_tree = f_html.replace(".html", "_syntax_tree.json");
//                 let f_ast = f_html.replace(".html", "_ast.json");
//                 fs::write(&f_syntax_tree, format!("{:#?}", syntax_tree))?;
//                 fs::write(&f_ast, format!("{:#?}", ast))?;
//                 tracing::info!("  - Syntax tree: {f_syntax_tree}");
//                 tracing::info!("  - AST:         {f_ast}");
//             }
//         }

//         Ok(())
//     }

//     pub fn dir2html(
//         &mut self,
//         d_org: &str,
//         d_html: &str,
//         debug: u8,
//     ) -> Result<(), Box<dyn std::error::Error>> {
//         std::fs::create_dir_all(d_html)?;

//         for entry in WalkDir::new(d_org).into_iter().filter_map(|s| s.ok()) {
//             tracing::debug!("entry: {entry:?}");
//             if entry
//                 .file_name()
//                 .to_str()
//                 .expect("to str")
//                 .ends_with(".org")
//                 && !entry.file_name().to_str().expect("to str").starts_with(".")
//             {
//                 // mode? index/samename
//                 let f_org = entry.path().to_str().expect("to str");
//                 let f_html = entry
//                     .path()
//                     .parent()
//                     .unwrap()
//                     .join("index.html")
//                     .into_os_string()
//                     .into_string()
//                     .expect("Path contains invalid Unicode")
//                     .replace(d_org, d_html);
//                 // let f_html = f_org.replacen(d_org, d_html, 1).replace(".org", ".html");
//                 tracing::debug!("{:?} -> {:?}", entry.path(), &f_html);
//                 self.org2html(f_org, &f_html, debug)?;
//             }
//         }

//         Ok(())
//     }
// }
