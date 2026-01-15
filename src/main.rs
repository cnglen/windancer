//! org -> html
// #![feature(test)]
#![allow(warnings)]
use clap::Parser;
use orgize::config;
use tracing_subscriber::FmtSubscriber;

mod ast;
mod constants;
mod engine;
mod parser;
mod renderer;
use crate::ast::builder::AstBuilder;
use crate::engine::Engine;
use crate::parser::syntax::SyntaxToken;
use crate::parser::{OrgParser, config::OrgParserConfig, config::OrgUseSubSuperscripts};
use crate::renderer::Render;
use crate::renderer::html::{HtmlRenderer, RenderConfig};
use orgize::{Org, rowan::ast::AstNode};
use rowan::{GreenNode, GreenToken, NodeOrToken, WalkEvent};
use std::fs;

#[derive(Parser)]
#[command(name = "winancer")]
#[command(version = "0.1")]
#[command(about = "Render a org file to html", long_about = None)]
struct Cli {
    /// Input path of org file or input directory
    #[arg(short = 'i', long)]
    input: String,

    /// Output path of html file or input directory
    #[arg(short = 'o', long)]
    output: Option<String>,

    /// Turn debugging information on: `-d -dd -ddd -dddd`
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

fn main() {
    let args = Cli::parse();
    let max_level = match args.debug {
        0 => tracing::Level::ERROR,
        1 => tracing::Level::WARN,
        2 => tracing::Level::INFO,
        3 => tracing::Level::DEBUG,
        _ => tracing::Level::TRACE,
    };
    let subscriber = FmtSubscriber::builder().with_max_level(max_level).finish();
    tracing::subscriber::set_global_default(subscriber).expect("set global subscripber failed");

    let mut engine = Engine::new();

    let input = std::path::Path::new(&args.input);
    if input.is_file() {
        tracing::debug!("single file mode");
        let f_org = &args.input.clone();
        let f_html = match args.output {
            Some(ref file) => file,
            None => &args.input.replace(".org", ".html"),
        };
        engine.org2html(f_org, f_html, args.debug);
    } else if input.is_dir() {
        tracing::debug!("batch mode in directory");

        let d_org = if !args.input.clone().ends_with('/') {
            &format!("{}/", args.input)
        } else {
            &args.input
        };

        let d_html = match args.output {
            Some(ref dir) if dir.ends_with('/') => dir,
            Some(ref dir) => &format!("{}/", dir),
            None => "./dist/",
        };

        tracing::debug!("d_org={}, d_html={}", d_org, d_html);

        engine.dir2html(d_org, d_html, args.debug);
    }

    // engine.dir2html(f_org, f_html, args.debug);

    // let mut parser = OrgParser::new(
    //     OrgParserConfig::default().with_use_sub_superscripts(OrgUseSubSuperscripts::Brace),
    // );
    // let parser_output = parser.parse(&f_org);
    // let green_tree = parser_output.green();
    // let syntax_tree = parser_output.syntax();

    // let ast_builder = AstBuilder::new();
    // let ast = ast_builder.build(&syntax_tree).unwrap();
    // let renderer_config = RenderConfig::default();
    // let mut html_renderer = HtmlRenderer::new(renderer_config);
    // let html = html_renderer.render_document(&ast);
    // let f_html = match args.f_html {
    //     Some(ref file) => file,
    //     None => &args.f_org.replace(".org", ".html"),
    // };
    // fs::write(f_html, format!("{}", html));
    // tracing::info!("{:} -> {} done", f_org, f_html);
    // match args.debug {
    //     0 => {}
    //     1 => {
    //         let f_ast = f_org.replace(".org", "_ast.json");
    //         fs::write(&f_ast, format!("{:#?}", ast));
    //         tracing::info!("  - AST:         {f_ast}");
    //     }
    //     _ => {
    //         let f_green_tree = f_org.replace(".org", "_green_tree.json");
    //         let f_syntax_tree = f_org.replace(".org", "_syntax_tree.json");
    //         let f_ast = f_org.replace(".org", "_ast.json");
    //         fs::write(&f_green_tree, format!("{:#?}", green_tree));
    //         fs::write(&f_syntax_tree, format!("{:#?}", syntax_tree));
    //         fs::write(&f_ast, format!("{:#?}", ast));
    //         tracing::info!("  - Green tree:  {f_green_tree}");
    //         tracing::info!("  - Syntax tree: {f_syntax_tree}");
    //         tracing::info!("  - AST:         {f_ast}");
    //     }
    // }
}
