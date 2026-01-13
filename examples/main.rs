// #![allow(warnings)]
//! `cargo run --example main -- --show-output`
use orgize::{Org, rowan::ast::AstNode};
use std::fs;
use std::time::Instant;
use tracing;
use tracing_subscriber::FmtSubscriber;
use windancer::ast::builder::AstBuilder;
use windancer::parser::config::OrgUseSubSuperscripts;
use windancer::parser::{OrgParser, config::OrgParserConfig};
use windancer::renderer::html::{HtmlRenderer, RenderConfig};

fn main() -> std::io::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("set global subscripber failed");

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
        tracing::info!("orgize               : {:?}", duration);
    }

    // windancer
    // - parser
    let start = Instant::now();
    let org_config =
        OrgParserConfig::default().with_use_sub_superscripts(OrgUseSubSuperscripts::Brace);
    let mut parser = OrgParser::new(org_config);
    let parser_output = parser.parse(f_org);
    let syntax_tree = parser_output.syntax();
    let _ = fs::write(
        "tests/windancer_red_tree.json",
        format!("{:#?}", syntax_tree),
    );
    let duration = start.elapsed();
    tracing::info!("windancer@parser     : {:?}", duration);

    // // - ast builder
    let start = Instant::now();
    let ast_builder = AstBuilder::new();
    let ast = ast_builder.build(&syntax_tree).unwrap();
    let _ = fs::write("tests/windancer_ast.json", format!("{:#?}", ast));
    let duration = start.elapsed();
    tracing::info!("windancer@AST builder: {:?}", duration);

    // // - html render
    let start = Instant::now();
    let renderer_config = RenderConfig::default();
    let mut html_renderer = HtmlRenderer::new(renderer_config);
    let html = html_renderer.render_document(&ast);
    let _ = fs::write("tests/windancer_output.html", format!("{}", html));
    let duration = start.elapsed();
    tracing::info!("windancer@Html render: {:?}", duration);

    Ok(())
}
