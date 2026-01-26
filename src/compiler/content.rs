// Model of content, i.e, output of Compiler

// todo:
//   next, prev (for next chatper/ prev chapter)
//   parent
//
// Meta data of org file
use chrono::{DateTime, Local};
use orgize::ast::PropertyDrawer;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::compiler::ast_builder::element::{self, OrgFile};
use crate::compiler::org_roam::{NodeType, RoamNode};
use crate::compiler::parser::syntax::{OrgSyntaxKind, SyntaxNode};
use crate::export::ssg::renderer::Renderer; // remove to exporter?

/// A single directory is compiled to `Section`
#[derive(Debug)]
pub struct Section {
    pub file_info: FileInfo,
    pub documents: Vec<Document>,
    pub subsections: Vec<Section>,
    pub metadata: SectionMetadata,
}

/// A single org file is compiled to `Document` by compiler
pub struct Document {
    pub file_info: FileInfo,
    pub metadata: DocumentMetadata,
    pub ast: OrgFile,
    pub syntax_tree: SyntaxNode,
}

impl std::fmt::Debug for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Documet: file_info={:?} metadata={:?}",
            self.file_info, self.metadata
        )
    }
}

impl Document {
    pub fn extract(&self) -> () {
        // let nodes = vec![];

        let (mut maybe_id, mut aliases, mut refs, mut properties) =
            (None, vec![], vec![], HashMap::new());
        if let Some(zeroth_section) = &self.ast.zeroth_section {
            for element in zeroth_section.elements.iter().filter_map(|e| {
                if let element::Element::PropertyDrawer(property_drawer) = e {
                    Some(property_drawer)
                } else {
                    None
                }
            }) {
                for node_property in &element.contents {
                    let (name, value) = (&node_property.name, &node_property.value);

                    if let Some(value) = value {
                        match name.as_str() {
                            "ID" => {
                                maybe_id = Some(value);
                            }
                            "ROAM_ALIASES" => {
                                aliases.extend(value.split_whitespace().map(str::to_string));
                            }
                            "ROAM_REFS" => {
                                refs.extend(value.split_whitespace().map(str::to_string));
                            }
                            other => {
                                properties.insert(other.to_string(), value.to_string());
                            }
                        }
                    }
                }
            }
        }
        let doc_node = if let Some(id) = maybe_id {
            let node = RoamNode {
                id: id.to_string(),
                aliases,
                refs,
                properties,
                title: self
                    .metadata
                    .title
                    .clone()
                    .unwrap_or("empty title".to_string()),
                node_type: NodeType::File,
                file_info: self.file_info.clone(),
                tags: self.metadata.filetags.clone(),
                level: 0,
            };
            Some(node)
        } else {
            None
        };

        // // iter subtrees and link
        // self.ast.heading_subtrees

        tracing::debug!(doc_node=?doc_node);
    }

    /// Return the html path relative to root of site
    pub fn html_path(&self) -> String {
        let directory = if let Some(relative_directories) = &self.file_info.relative_directories {
            relative_directories.join("/")
        } else {
            tracing::warn!(
                "no 'content' found in {}, use '' as relative_drectory",
                self.file_info.full_path.display(),
            );
            "".to_string()
        };

        let html_file_name = if self.file_info.maybe_index {
            "index.html".to_string()
        } else {
            Renderer::slugify(self.file_info.file_name.replace(".org", ".html"))
        };

        std::path::Path::new(&directory)
            .join(&html_file_name)
            .to_string_lossy()
            .to_string()
    }
}

/// File info for a file(directory is a speical case of file), for example
/// - filename: bar.org
/// - full_path: /foo/content/blog/bar/bar.org
/// - relative_path: blog/bar/bar.org
/// - relative_directories: [blog, bar]
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// full path in file system
    pub full_path: PathBuf,

    /// file name, e.g, "bar.org"
    pub file_name: String,

    // without extension
    pub name: String,

    /// relative path staring from "content"
    pub relative_path: Option<String>,

    pub(crate) maybe_index: bool,

    /// relative directories
    // used in SSG export for directory structure
    // Staring from "content" directory, without starting /, used in SSG export to generate ${OUTPUT_DIRECTORY}/relative_path/index.html
    pub relative_directories: Option<Vec<String>>,
}

impl FileInfo {
    // f: ~ not supported, you have to expand HOME your self if needed
    pub fn from<P: AsRef<Path>>(f: P) -> Self {
        let path = f.as_ref();
        let file_name = path
            .file_name()
            .expect("no file name")
            .to_string_lossy()
            .to_string();

        let name = path.file_stem().unwrap().to_string_lossy().to_string();

        let full_path = fs::canonicalize(path).expect("no full path");

        let mut is_in_content = false;
        let mut relative_directories_vec = vec![];
        for section in path.parent().unwrap().components() {
            let component = section.as_os_str().to_string_lossy();
            if is_in_content {
                relative_directories_vec.push(component.to_string());
            } else if component == "content" {
                is_in_content = true;
            }
        }

        let n = relative_directories_vec.len();
        let maybe_index = if n > 0 && relative_directories_vec[n - 1] == name {
            true
        } else {
            false
        };

        let (relative_path, relative_directories) = if is_in_content {
            (
                if !relative_directories_vec.is_empty() {
                    Some(format!(
                        "{}/{}",
                        relative_directories_vec.join("/"),
                        file_name
                    ))
                } else {
                    Some(file_name.clone())
                },
                Some(relative_directories_vec),
            )
        } else if file_name == "content" && path.is_dir() {
            (Some("".to_string()), Some(relative_directories_vec))
        } else {
            (None, None)
        };

        Self {
            full_path,
            file_name,
            name,
            maybe_index,
            relative_path,
            relative_directories,
        }
    }
}

#[derive(Debug)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub created_ts: Option<DateTime<Local>>,
    pub last_modified_ts: Option<DateTime<Local>>,
    pub filetags: Vec<String>,
    pub category: Vec<String>,
    pub weight: Option<usize>,
    pub language: Option<String>,

    pub is_draft: bool,
    pub enable_render: bool, // only work for exporter
    pub in_search_index: bool,

    // // // 🔥 Org-roam 核心属性
    // pub roam_node_id: Option<String>, // ID property of zeroth section
    // pub roam_alias: Vec<String>, // ROAM_ALIAS property of zeroth section
    // // 🔥 链接数据（编译时从AST中提取出的原始id链接目标）
    // pub roam_links: Vec<RawLink>,

    // roam_nodes_in_heading: Vec<>
    pub extra: HashMap<String, Vec<String>>,
}

// // id <-> url?
// // 原始链接表示
// pub enum RawLink {
//     RoamId { id: String },      // [[id:xxxxx][描述]]
//     RoamFile { file: String },   // [[file:...][描述]]
//     WebUrl { url: String },      // [[https://...][描述]]
//     // ... 其他类型
// }

impl Section {
    pub fn extract(&self) {}
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            title: None,
            authors: Vec::new(),
            created_ts: None,
            last_modified_ts: None,
            filetags: Vec::new(),
            category: Vec::new(),
            weight: None,
            language: None,
            is_draft: false,
            enable_render: true,
            in_search_index: true,
            extra: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct SectionMetadata {
    pub title: String,
    pub weight: Option<f64>,
    pub extra: HashMap<String, String>,
}

impl Default for SectionMetadata {
    fn default() -> Self {
        Self {
            title: "todo".to_string(),
            weight: None,
            extra: HashMap::new(),
        }
    }
}
