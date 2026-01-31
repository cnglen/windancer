//! org -> html
// #![feature(test)]
#![allow(warnings)]
use clap::Parser;
use export::ssg::StaticSiteGenerator;
use orgize::config;
use tracing_subscriber::FmtSubscriber;

mod compiler;
mod constants;
mod engine;
mod export;
use std::fs;

use orgize::Org;
use orgize::rowan::ast::AstNode;
use rowan::{GreenNode, GreenToken, NodeOrToken, WalkEvent};

use crate::compiler::Compiler;
use crate::compiler::ast_builder::AstBuilder;
use crate::compiler::parser::OrgParser;
use crate::compiler::parser::config::{OrgParserConfig, OrgUseSubSuperscripts};
use crate::compiler::parser::syntax::SyntaxToken;
use crate::engine::Engine;
use crate::export::ssg::Config;
use crate::export::ssg::html::HtmlRenderer;
use crate::export::ssg::site::SiteBuilder;

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

    let mut ssg = StaticSiteGenerator::default();
    let d_org = args.input.clone();
    ssg.generate(d_org);
}
