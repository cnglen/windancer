use crate::compiler::ast_builder::element::HeadingSubtree;
use crate::compiler::content::{Document, Section};
use std::collections::{HashMap, HashSet};

// Site:
// Page

/// Node of TableOfContents
#[derive(Debug, Clone)]
pub struct TocNode {
    /// title to display in Toc, i.e, <a href={path}>title</a>
    pub title: String,
    /// href in <a> html
    pub path: String,

    // /// number of  in path.split("/"), note: path/to/index.html -> path/to
    // /// - / <- /index.html :: level=0
    // /// - /blog <- /blog/index.html :: level=1
    // /// - /blog/bar.html :: level=2
    // /// - /blog/note/rust.html :: level=3
    // pub level: usize,
    /// children nodes, only path ends with index.html has non-empty children
    pub children: Vec<TocNode>,
    // /// true if filename of path is "index.html"
    // pub is_index: bool,
}

impl TocNode {
    // // Toc node for page's content only
    // // todo: heading using id(property_id > hash): https://yoursite.com/foo/#d061c832dd9cdb14f32148b81a1ac02416ce76d1
    // fn from_document(document: &Document) -> Self {
    //     let ast = &document.ast;

    //     fn from_subtree(h: &HeadingSubtree) -> TocNode {
    //         // let title = h
    //         //     .title
    //         //     .iter()
    //         //     .map(|e| self.render_object(e))
    //         //     .collect::<String>();

    //         // let path;           // get id, from html? or use same hash? or ast add id?maybe
    //         // s.sub_heading_subtrees

    //         // TocNode {
    //         // }
    //     }

    //     let mut children = vec![];
    //     for subtree in ast.heading_subtrees {
    //         children.push(from_subtree(subtree));
    //     }

    //     TocNode{
    //         title: document.metadata.title.clone(),
    //         path: document.html_path(),
    //         children
    //     }

    // }

    // index.html as root_node
    pub fn from_section(section: &Section) -> Self {
        fn from_document(document: &Document) -> TocNode {
            let path = document.html_path();
            let title = document
                .metadata
                .title
                .clone()
                .unwrap_or("no title found".to_string());

            TocNode {
                title,
                path,
                children: vec![],
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

            TocNode {
                title: String::from("faked index node"),
                path: path,
                children: vec![],
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
        let p = std::path::Path::new(&self.path);

        let is_index = p.file_name().expect("must has file name") == "index.html";

        if is_index {
            p.components().count() - 1
        } else {
            p.components().count()
        }
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

            if node.level() <= max_depth {
                html.push_str(&format!(
                    r#"<li{}><a href="/{}">{}</a>"#,
                    active_class, node.path, node.title
                ));

                if !node.children.is_empty() && node.level() < max_depth {
                    html.push_str("\n<ol>\n");
                    for child in &node.children {
                        node_to_html(child, active_slug, html, max_depth);
                    }
                    html.push_str("</ol>\n");
                }

                html.push_str("</li>\n");
            }
        }

        let mut html = String::from("<nav class=\"toc\">\n  <ul>\n");
        for node in &self.root_nodes {
            node_to_html(node, active_slug, &mut html, 5);
        }
        html.push_str("  </ul>\n</nav>\n");

        html
    }
}
