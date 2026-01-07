// #![allow(warnings)]
//! `cargo run --example main -- --show-output`
use orgize::{Org, rowan::ast::AstNode};
use std::fs;
use std::time::Instant;
use windancer::ast::builder::AstBuilder;
use windancer::parser::{OrgParser, config::OrgParserConfig};
use windancer::renderer::html::{HtmlRenderer, RenderConfig};

fn main() -> std::io::Result<()> {
    let f_org = "tests/test.org";
    let input = &fs::read_to_string(f_org).unwrap_or(String::new());

    let use_orgize = true;
    if use_orgize {
        // orgize
        let start = Instant::now();
        let orgize_green_tree = Org::parse(input);
        let _ = fs::write(
            "tests/orgize_red_tree.json",
            format!("{:#?}", orgize_green_tree.document().syntax()),
        );
        let _ = fs::write("tests/orgize_output.html", orgize_green_tree.to_html());
        let duration = start.elapsed();
        println!("orgize               : {:?}", duration);
    }

    // windancer
    // - parser
    let start = Instant::now();
    let org_config = OrgParserConfig::default();
    let mut parser = OrgParser::new(org_config);
    let parser_output = parser.parse(input);
    let syntax_tree = parser_output.syntax();
    let _ = fs::write(
        "tests/windancer_red_tree.json",
        format!("{:#?}", syntax_tree),
    );
    let duration = start.elapsed();
    println!("windancer@parser     : {:?}", duration);

    // // - ast builder
    let start = Instant::now();
    let ast_builder = AstBuilder::new();
    let ast = ast_builder.build(&syntax_tree).unwrap();
    let _ = fs::write("tests/windancer_ast.json", format!("{:#?}", ast));
    let duration = start.elapsed();
    println!("windancer@AST builder: {:?}", duration);

    // // - html render
    let start = Instant::now();
    let renderer_config = RenderConfig::default();
    let mut html_renderer = HtmlRenderer::new(renderer_config);
    let html = html_renderer.render_document(&ast);
    let _ = fs::write("tests/windancer_output.html", format!("{}", html));
    let duration = start.elapsed();
    println!("windancer@Html render: {:?}", duration);

    Ok(())
}
