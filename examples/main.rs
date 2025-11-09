// #![allow(warnings)]
//! cargo run --example main -- --show-output
use orgize::{Org, rowan::ast::AstNode};
use std::fs;
use windancer::ast::builder::AstBuilder;
use windancer::parser::{OrgConfig, OrgParser};
use windancer::renderer::html::{HtmlRenderer, RenderConfig};

fn main() -> std::io::Result<()> {
    // let f_org = "/home/touch/note/src/git-20250802184314.org";
    let f_org = "tests/test.org";
    let input = &fs::read_to_string(f_org).unwrap_or(String::new());

    let orgize_green_tree = Org::parse(input);
    let _ = fs::write(
        "tests/orgize_red_tree.json",
        format!("{:#?}", orgize_green_tree.document().syntax()),
    );

    let _ = fs::write("tests/orgize_output.html", orgize_green_tree.to_html());

    let org_config = OrgConfig::default();
    let mut parser = OrgParser::new(org_config);
    let parser_output = parser.parse(input);
    let _green_tree = parser_output.green();
    let syntax_tree = parser_output.syntax();
    let _ = fs::write(
        "tests/windancer_red_tree.json",
        format!("{:#?}", syntax_tree),
    );

    let ast_builder = AstBuilder::new();
    let ast = ast_builder.build(&syntax_tree).unwrap();
    let _ = fs::write("tests/windancer_ast.json", format!("{:#?}", ast));

    let renderer_config = RenderConfig {
        include_css: false,
        class_prefix: String::from(""),
        highlight_code_blocks: false,
    };

    let mut html_renderer = HtmlRenderer::new(renderer_config);
    let html = html_renderer.render_document(&ast);
    let _ = fs::write("tests/windancer_output.html", format!("{}", html));

    Ok(())
}
