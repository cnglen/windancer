//! org -> html
// #![feature(test)]
#![allow(warnings)]
use clap::Parser;
use orgize::config;
use tracing_subscriber::FmtSubscriber;

mod compiler;
mod constants;
mod engine;
mod export;
use crate::compiler::ast_builder::AstBuilder;
use crate::compiler::parser::syntax::SyntaxToken;
use crate::compiler::parser::{OrgParser, config::OrgParserConfig, config::OrgUseSubSuperscripts};
use crate::engine::Engine;
use crate::export::ssg::html::{HtmlRenderer};
use orgize::{Org, rowan::ast::AstNode};
use rowan::{GreenNode, GreenToken, NodeOrToken, WalkEvent};
use std::fs;
use windancer::export::ssg::renderer::{Renderer, RendererConfig};

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

    
    let mut renderer = Renderer::new(RendererConfig::default());
    
    let input = std::path::Path::new(&args.input);
    if input.is_file() {
        tracing::debug!("single file mode");
        let f_org = &args.input.clone();
        let f_html = match args.output {
            Some(ref file) => file,
            None => &args.input.replace(".org", ".html"),
        };
        renderer.build_file(f_org);
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

        renderer.build(d_org);
    }
}
