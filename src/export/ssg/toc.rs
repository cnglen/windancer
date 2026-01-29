use crate::compiler::ast_builder::element::HeadingSubtree;
use crate::compiler::content::{Document, Section};
use serde;
use std::collections::{HashMap, HashSet};

// Site:
// Page

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

impl TocNode {
    pub fn from_section(section: &Section) -> Self {
        fn from_document(document: &Document) -> TocNode {
            let path = document.html_path();
            let title = document
                .metadata
                .title
                .clone()
                .unwrap_or("no title found".to_string());

            let level = if document.file_info.maybe_index {
                std::path::Path::new(&path).components().count() - 1
            } else {
                std::path::Path::new(&path).components().count()
            };

            TocNode {
                title,
                path,
                children: vec![],
                level,
            }
        }

        let mut children = vec![];
        let mut maybe_root = None;
        for doc in &section.documents {
            let node = from_document(doc);
            if doc.file_info.maybe_index {
                maybe_root = Some(node);
            } else {
                children.push(node);
            }
        }

        let mut root = if let Some(root) = maybe_root {
            root
        } else {
            let path = if let Some(relative_path) = &section.file_info.relative_path {
                std::path::Path::new(&relative_path).join("index.html")
            } else {
                std::path::Path::new("index.html").to_path_buf()
            };
            let path = path.to_string_lossy().to_string();
            let level = path.split("/").count() - 1;
            TocNode {
                title: String::from("faked index node"),
                path: path,
                children: vec![],
                level,
            }
        };

        root.children.extend(children);
        for subsection in &section.subsections {
            if subsection.documents.len() > 0 || subsection.subsections.len() > 0 {
                let toc_node = Self::from_section(subsection);
                root.children.push(toc_node);
            }
        }
        root
    }

    fn level(&self) -> usize {
        self.level
    }
}

#[derive(Debug, Clone)]
pub struct TableOfContents {
    // pub flatten_nodes: Vec<TocNode>,
    pub root_nodes: Vec<TocNode>, // not flatten
}

impl TableOfContents {
    pub fn new(root_nodes: Vec<TocNode>) -> Self {
        // fn flatten(node: &TocNode) -> Vec<TocNode> {
        //     let mut ans = vec![];
        //     ans.push(node.clone());

        //     for child in &node.children {
        //         ans.extend(flatten(&child));
        //     }
        //     ans
        // }

        // let mut flatten_nodes = vec![];
        // for node in &root_nodes {
        //     flatten_nodes.extend(flatten(node));
        // }

        Self {
            root_nodes,
            // flatten_nodes,
        }
    }
}

use tera::{Context, Tera};

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
