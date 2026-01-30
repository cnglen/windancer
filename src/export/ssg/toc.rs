//! Table of contents for SSG

use serde;
use tera::{Context, Tera};

/// Node of TableOfContents
#[derive(Debug, Clone, serde::Serialize)]
pub struct TocNode {
    /// title to display in Toc, i.e, <a href={path}>title</a>
    pub title: String,

    /// href in <a> html
    pub path: String,

    /// level in TOC
    pub level: usize,

    /// children nodes, only path ends with index.html has non-empty children
    pub children: Vec<TocNode>,
}

#[derive(Debug, Clone)]
pub struct TableOfContents {
    pub root_nodes: Vec<TocNode>, // not flatten
}

impl TableOfContents {
    pub fn new(root_nodes: Vec<TocNode>) -> Self {
        Self { root_nodes }
    }
}

impl TableOfContents {
    pub fn to_html_nav(&self, active_slug: Option<&str>) -> String {
        let tera = match Tera::new("src/export/ssg/templates/**/*.html") {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("template error: {}", e);
                ::std::process::exit(1);
            }
        };
        let mut context = Context::new();
        context.insert("root_nodes", &self.root_nodes);
        context.insert("active_slug", &active_slug);
        context.insert("max_depth", &5);
        let html = tera
            .render("toc_nav.html", &context)
            .unwrap_or_else(|err| format!("Template rendering failed: {}", err));

        // beatufity html
        use tidier::{Doc, FormatOptions};
        let opts = FormatOptions::new()
            .wrap(80)
            .tabs(false)
            .strip_comments(false);

        let doc = Doc::new(html, false).expect("todo"); // 第二个参数 `false` 表示输入非XHTML
        doc.format(&opts).expect("todo")
    }
}
