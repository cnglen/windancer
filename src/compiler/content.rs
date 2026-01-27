// Model of content, i.e, output of Compiler

// todo:
//   next, prev (for next chatper/ prev chapter)
//   parent
//
// Meta data of org file
use chrono::{DateTime, Local};
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::compiler::ast_builder::element::{HeadingSubtree, OrgFile};
use crate::compiler::org_roam::{EdgeType, NodeType, RawAst, RoamGraph, RoamNode};
use crate::compiler::parser::syntax::SyntaxNode;
use crate::export::ssg::renderer::Renderer; // remove to exporter?

/// A single directory is compiled to `Section`
#[derive(Debug)]
pub struct Section {
    pub file_info: FileInfo,
    pub documents: Vec<Document>,
    pub subsections: Vec<Section>,
    pub metadata: SectionMetadata,
}

impl Section {
    pub fn build_graph(&self) -> RoamGraph {
        let mut graph = DiGraph::<RoamNode, EdgeType>::new();
        let mut id_to_index: HashMap<String, NodeIndex> = HashMap::new();
        let mut refs_to_id: HashMap<String, String> = HashMap::new();

        for document in self.documents.iter() {
            for node in document.ast.roam_nodes.iter() {
                let index = graph.add_node(node.clone());
                id_to_index.insert(node.id.clone(), index);

                for refs in node.refs.iter() {
                    refs_to_id.insert(refs.clone(), node.id.clone());
                }
            }
        }

        for document in self.documents.iter() {
            for extracted_link in document.ast.extracted_links.iter() {
                if extracted_link.link.protocol == "id" {
                    if let Some(source_id) = extracted_link.source_roam_id() {
                        let target_id = extracted_link
                            .link
                            .path
                            .strip_prefix("id:")
                            .expect("must have ID in path")
                            .to_string();

                        if let Some(source_index) = id_to_index.get(source_id.as_str()) {
                            if let Some(target_index) = id_to_index.get(&target_id) {
                                graph.add_edge(
                                    *source_index,
                                    *target_index,
                                    EdgeType::ExplicitReference {
                                        source_path: vec![],
                                    },
                                );
                            }
                        }
                    }
                }
            }
        }

        RoamGraph { id_to_index, graph }
    }
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
    fn get_roam_info(
        properties: &HashMap<String, String>,
    ) -> Option<(String, Vec<String>, Vec<String>, HashMap<String, String>)> {
        if let Some(id) = properties.get("ID") {
            let (mut aliases, mut refs, mut _properties) =
                (vec![], vec![], HashMap::<String, String>::new());
            for (k, v) in properties.iter() {
                match k.as_str() {
                    "ID" => {}
                    "ROAM_ALIASES" => {
                        aliases.extend(v.split_whitespace().map(str::to_string));
                    }
                    "ROAM_REFS" => {
                        refs.extend(v.split_whitespace().map(str::to_string));
                    }
                    other => {
                        _properties.insert(other.to_string(), v.to_string());
                    }
                }
            }

            Some((id.to_string(), aliases, refs, _properties))
        } else {
            None
        }
    }

    // collect roam_node and all links: from ast? or in building process?
    // build graph
    pub fn extract(&self) -> () {
        // // let nodes = vec![];
        // let doc_node = self.extract_file_roam_node();

        // fn traverse(
        //     heading_subtree: &HeadingSubtree,
        //     parent_stack: &mut Vec<RoamNode>, // parent, level
        //     collector: &mut Vec<(RoamNode, Vec<String>, Option<String>)>, // (节点, 引用列表, 显式父ID)
        //     current_level: u8,
        // ) {
        //     if let Some(roam_node) = Document::extract_headline_roam_node(heading_subtree) {
        //         let mut parent_id = None;
        //         while let Some(parent_node) = parent_stack.last() {
        //             if parent_node.level < current_level {
        //                 parent_id = Some(parent_node.id.clone());
        //                 break;
        //             } else {
        //                 parent_stack.pop(); // 同级或更高级，出栈
        //             }
        //         }

        //         // 3. 提取该节点内容中的所有 ID 引用
        //         // first_section OK
        //         // sub roam_node jump
        //         // non roam_node -> into
        //         // let refs = extract_id_refs_from_content(&roam_node.raw_content);

        //         //         // 4. 记录节点及其关系，稍后统一处理
        //         //         collector.push((node, refs, parent_id));

        //         //         // 将当前节点压入栈，用于后续子节点的父子关系判断
        //         //         // 注意：此时尚未分配 NodeIndex，先压入一个占位符
        //         // parent_stack.push(roam_node);
        //     }

        //     //     // 递归处理子节点
        //     //     for child in heading_subtree.sub_heading_subtrees() {
        //     //         let child_level = child.level;
        //     //         traverse(&child, parent_stack, collector, child_level);
        //     //     }

        //     //     // 遍历完当前节点的子节点后，如果当前节点是RoamNode，则出栈
        //     //     if is_roam_node(syntax) { // ??
        //     //         parent_stack.pop();
        //     //     }
        //     // }

        //     // // 开始遍历，收集所有节点和关系
        //     // let mut collector = Vec::new();
        //     // traverse(syntax_root, &mut node_stack, &mut collector, 0);

        //     // // **第二阶段：构建图谱**
        //     // // 1. 将所有节点加入图，建立 ID 到索引的映射
        //     // for (node, _, _) in &collector {
        //     //     let index = graph.add_node(node.clone());
        //     //     id_to_index.insert(node.id.clone(), index);
        //     // }

        //     // // 2. 处理父子关系和引用关系，添加边
        //     // for (node, refs, parent_id) in collector {
        //     //     let source_index = id_to_index[&node.id];

        //     //     // a. 处理父子关系（优先使用显式 :PARENT:）
        //     //     if let Some(pid) = parent_id {
        //     //         if let Some(&parent_index) = id_to_index.get(&pid) {
        //     //             graph.add_edge(parent_index, source_index, EdgeType::Parent);
        //     //         } // 如果父节点不在当前图内，可记录为“悬挂父节点”待后续处理
        //     //     }

        //     //     // b. 处理ID引用关系
        //     //     for ref_id in refs {
        //     //         if let Some(&target_index) = id_to_index.get(&ref_id) {
        //     //             graph.add_edge(source_index, target_index, EdgeType::ExplicitReference);
        //     //         } // 同样，外部引用可记录
        //     //     }
        // }

        // RoamGraph { graph, id_to_index }

        // // iter subtrees and link
        // self.ast.heading_subtrees

        // collect node: root/subtree
        // node -parent-> node: subtree
        // node -idref -> node: id link: in which tree, find link, get target id

        // tracing::debug!(doc_node=?doc_node);
        // RoamGraph
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
