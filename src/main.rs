//! org -> html
#![allow(warnings)]
use clap::Parser;

mod ast;
mod parser;
mod renderer;

use crate::ast::builder::AstBuilder;
use crate::parser::syntax::SyntaxToken;
use crate::parser::{OrgConfig, OrgParser};
use crate::renderer::Render;
use crate::renderer::html::{HtmlRenderer, RenderConfig};
use orgize::{Org, rowan::ast::AstNode};
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

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

fn main() {
    let args = Cli::parse();

    let f_org = args.f_org.clone();
    let input = &fs::read_to_string(&f_org).expect(&format!("can't read from {}", args.f_org));

    let org_config = OrgConfig::default();
    let mut parser = OrgParser::new(org_config);
    let parser_output = parser.parse(input);
    let syntax_tree = parser_output.syntax();
    let ast_builder = AstBuilder::new();
    let ast = ast_builder.build(&syntax_tree).unwrap();
    let renderer_config = RenderConfig {
        include_css: false,
        class_prefix: String::from(""),
        highlight_code_blocks: false,
    };
    let html_renderer = HtmlRenderer::new(renderer_config);
    let html = html_renderer.render_document(&ast);
    let f_html = match args.f_html {
        Some(ref file) => file,
        None => &args.f_org.replace(".org", ".html"),
    };
    fs::write(f_html, format!("{}", html));
    println!("{:} -> {} done", f_org, f_html);
    match args.debug {
        0 => {}
        1 => {
            let f_ast = f_org.replace(".org", "_ast.json");
            fs::write(&f_ast, format!("{:#?}", ast));
            println!("  - AST: {f_ast}");
        }
        _ => {
            let f_syntax_tree = f_org.replace(".org", "_syntax_tree.json");
            let f_ast = f_org.replace(".org", "_ast.json");
            fs::write(&f_syntax_tree, format!("{:#?}", syntax_tree));
            fs::write(&f_ast, format!("{:#?}", ast));
            println!("  - Syntax tree: {f_syntax_tree}");
            println!("  - AST: {f_ast}");
        }
    }
}
