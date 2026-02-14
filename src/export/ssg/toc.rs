//! Table of contents for SSG

use serde;

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
    root_nodes: Vec<TocNode>, // not flatten
}

impl TableOfContents {
    pub fn new(root_nodes: Vec<TocNode>) -> Self {
        Self { root_nodes }
    }
}

impl Default for TableOfContents {
    fn default() -> Self {
        Self { root_nodes: vec![] }
    }
}

impl TableOfContents {
    pub fn to_html_nav(&self, active_slug: Option<&str>) -> String {
        fn node_to_html(
            node: &TocNode,
            active_slug: Option<&str>,
            html: &mut String,
            max_depth: usize,
        ) {
            let is_active =
                active_slug.map_or(false, |slug| slug == node.path.trim_start_matches('#'));
            let active_class = if is_active { r#" class="active""# } else { "" };
            if node.level <= max_depth {
                html.push_str(&format!(
                    r#"<li{}><a href="{}">{}</a>"#,
                    active_class, node.path, node.title
                ));
                if !node.children.is_empty() && node.level < max_depth {
                    html.push_str("\n<ul>\n");
                    for child in &node.children {
                        node_to_html(child, active_slug, html, max_depth);
                    }
                    html.push_str("</ul>\n");
                }
                html.push_str("</li>\n");
            }
        }

        let max_depth = 5;
        let mut html = String::from(r#"<nav class="toc"> <ul>"#);
        for node in &self.root_nodes {
            node_to_html(node, active_slug, &mut html, max_depth);
        }
        html.push_str(r#"</ul></nav>"#);

        // beatufity html
        use tidier::{Doc, FormatOptions};
        let opts = FormatOptions::new()
            .wrap(80)
            .tabs(false)
            .strip_comments(false);

        let doc = Doc::new(html, false).expect("todo");
        doc.format(&opts).expect("todo")
    }
}
