//! org -> html
// #![feature(test)]
#![allow(warnings)]
use clap::Parser;
use orgize::config;
use tracing_subscriber::FmtSubscriber;

mod ast;
mod constants;
mod parser;
mod renderer;
use crate::ast::builder::AstBuilder;
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
    /// Input path of org file
    #[arg(short = 'i', long)]
    f_org: String,

    /// Output path of html file
    #[arg(short = 'o', long)]
    f_html: Option<String>,

    /// Turn debugging information on: `-d` render ast.json; `-dd` syntax_tree.json/ast.json
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

    let f_org = args.f_org.clone();
    let mut parser = OrgParser::new(
        OrgParserConfig::default().with_use_sub_superscripts(OrgUseSubSuperscripts::Brace),
    );
    let parser_output = parser.parse(&f_org);
    let green_tree = parser_output.green();
    let syntax_tree = parser_output.syntax();

    let ast_builder = AstBuilder::new();
    let ast = ast_builder.build(&syntax_tree).unwrap();
    let renderer_config = RenderConfig::default();
    let mut html_renderer = HtmlRenderer::new(renderer_config);
    let html = html_renderer.render_document(&ast);
    let f_html = match args.f_html {
        Some(ref file) => file,
        None => &args.f_org.replace(".org", ".html"),
    };
    fs::write(f_html, format!("{}", html));
    tracing::info!("{:} -> {} done", f_org, f_html);
    match args.debug {
        0 => {}
        1 => {
            let f_ast = f_org.replace(".org", "_ast.json");
            fs::write(&f_ast, format!("{:#?}", ast));
            tracing::info!("  - AST:         {f_ast}");
        }
        _ => {
            let f_green_tree = f_org.replace(".org", "_green_tree.json");
            let f_syntax_tree = f_org.replace(".org", "_syntax_tree.json");
            let f_ast = f_org.replace(".org", "_ast.json");
            fs::write(&f_green_tree, format!("{:#?}", green_tree));
            fs::write(&f_syntax_tree, format!("{:#?}", syntax_tree));
            fs::write(&f_ast, format!("{:#?}", ast));
            tracing::info!("  - Green tree:  {f_green_tree}");
            tracing::info!("  - Syntax tree: {f_syntax_tree}");
            tracing::info!("  - AST:         {f_ast}");
        }
    }
}
