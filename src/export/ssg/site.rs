/// Content Model, where Site is composed of Pages
/// - Site := Page + ... + Page
/// - Section -> SiteBuilder -> Site
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use fs_extra::dir::{CopyOptions, copy};
use petgraph::graph::{DiGraph, NodeIndex};
use rowan::GreenNode;
use walkdir::WalkDir;

use crate::compiler::ast_builder::element::OrgFile;
use crate::compiler::content::{Document, Section};
use crate::compiler::parser::syntax::{OrgSyntaxKind, SyntaxNode};
use crate::export::ssg::toc::{TableOfContents, TocNode};

#[derive(Clone)]
pub struct Page {
    pub id: PageId,

    pub title: String,
    // the url relative to root of site, such as "/blog/foo/index.html"
    pub url: String,
    // the html path relatie to root of site, such as "blog/foo/index.html"
    pub html_path: String,
    pub metadata: PageMetadata,
    pub ast: OrgFile,
    pub syntax_tree: SyntaxNode,

    pub parent_id: Option<PageId>,
    pub children_ids: Vec<PageId>,
    pub prev_sibling_id: Option<PageId>,
    pub next_sibling_id: Option<PageId>,
    pub prev_flattened_id: Option<PageId>,
    pub next_flattened_id: Option<PageId>,

    pub tags: HashSet<String>,
    pub category: Vec<String>,

    pub created_ts: Option<DateTime<Local>>,
    pub last_modified_ts: Option<DateTime<Local>>,
}

impl Page {
    fn faked(id: PageId, children_ids: Vec<PageId>) -> Self {
        Self {
            id,
            children_ids,

            title: String::default(),
            url: String::default(),
            metadata: PageMetadata {},
            ast: OrgFile {
                zeroth_section: None,
                heading_subtrees: vec![],
                footnote_definitions: vec![],
                keywords: BTreeMap::default(),
                properties: BTreeMap::default(),
                extracted_links: vec![],
                roam_nodes: vec![],
            },
            syntax_tree: SyntaxNode::new_root(GreenNode::new(OrgSyntaxKind::Root.into(), vec![])),

            parent_id: None,

            prev_sibling_id: None,
            next_sibling_id: None,
            prev_flattened_id: None,
            next_flattened_id: None,
            tags: HashSet::default(),
            category: vec![],
            html_path: String::default(),
            last_modified_ts: None,
            created_ts: None,
        }
    }
}
#[derive(Debug, Clone)]
pub struct PageMetadata {}
pub type PageId = String;
impl fmt::Debug for Page {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r##"Page {{
    id: {:#?},
    title: {:#?},
    url: {:#?},
    parent_id: {:#?},
    children_ids: {:#?},
}}"##,
            self.id, self.title, self.url, self.parent_id, self.children_ids
        )
    }
}

#[derive(Debug, Clone)]
pub struct SiteConfig {
    pub output_directory: PathBuf,
    // pub base_url: String,
    // pub theme: String,
    // pub generate_search_index: bool,
}
impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            output_directory: "public".into(),
        }
    }
}

// roam from site

#[derive(Debug)]
pub struct Site {
    pub config: SiteConfig,
    pub pages: HashMap<PageId, Page>,
    pub pageid_to_url: HashMap<PageId, String>,
    pub root_page_id: PageId,

    // build_tag_index(), get_pages_by_tag，generate_tag_pages
    pub tag_index: HashMap<String, Vec<PageId>>,
    pub flattened_pages: Vec<PageId>,

    // static assets: including css/image/fonts.
    _static_assets: Vec<(PathBuf, PathBuf)>,

    pub knowledge_graph: KnowledgeGraph,
    // roam_id, roamd_node, page_id
    // pub roam_nodes: Vec<String, >
    // todo: <roam_id> with page_id
    // roam_id <-> with page(url)

    // // 使用 PageId 作为图的节点，边的类型可以自定义（如：引用、提及、相关）
    // pub graph: petgraph::graph::DiGraph<PageId, LinkType>;
    // // 快速查找：从 org-roam id 到 page id 的映射
    // pub roam_id_to_page_id: HashMap<String, PageId>;

    // // 为每个页面预计算的相关页面列表（用于渲染，避免实时遍历图）
    // pub related_pages: HashMap<PageId, Vec<RelatedPage>>,
}
impl Site {
    fn is_faked_root(page_id: &PageId) -> bool {
        page_id == "FAKED_ROOT_PAGE_ID"
    }

    fn get_toc_of_page(&self, root_page_id: &PageId) -> TocNode {
        let page = self.pages.get(root_page_id).unwrap();
        let mut children = vec![];
        for child_page_id in page.children_ids.iter() {
            children.push(self.get_toc_of_page(child_page_id));
        }

        let level = page.url.split("/").count();

        TocNode {
            title: page.title.clone(),
            path: page.url.clone(),
            children,
            level,
        }
    }

    /// Get the toc
    pub fn toc(&self) -> TableOfContents {
        let root_nodes = self.get_toc_of_page(&self.root_page_id).children;

        // let root_nodes = if let Some(root) = self.root_page_id.clone() {
        //     let root_toc = self.get_toc_of_page(&root);
        //     root_toc.children
        // } else {
        //     let mut children = vec![];
        //     for id in self
        //         .pages
        //         .iter()
        //         .filter(|(_, page)| page.parent_id.is_none())
        //         .map(|(id, _)| id)
        //     {
        //         children.push(self.get_toc_of_page(&id));
        //     }
        //     children
        // };

        TableOfContents { root_nodes }
    }
}
impl Default for Site {
    fn default() -> Self {
        Self {
            config: SiteConfig::default(),
            pages: HashMap::new(),
            pageid_to_url: HashMap::new(),
            root_page_id: PageId::new(),
            tag_index: HashMap::new(),
            flattened_pages: vec![],
            _static_assets: vec![],
            knowledge_graph: KnowledgeGraph::default(),
        }
    }
}

use crate::compiler::ast_builder::object::Object;
use crate::compiler::org_roam::{EdgeType, NodeType, RoamNode};

#[derive(Debug, Clone)]
pub struct GraphNode {
    pub id: String,
    pub title: Vec<Object>,
    pub node_type: NodeType,
    pub aliases: Vec<String>,
    pub refs: Vec<String>,
    pub properties: BTreeMap<String, String>,
    pub tags: Vec<String>,
    pub level: u8,
    pub parent_id: Option<String>,

    pub url: String,
}

impl GraphNode {
    fn from(roam_node: &RoamNode, url: String) -> Self {
        Self {
            id: roam_node.id.clone(),
            title: roam_node.title.clone(),
            node_type: roam_node.node_type.clone(),
            aliases: roam_node.aliases.clone(),
            refs: roam_node.refs.clone(),
            properties: roam_node.properties.clone(),
            tags: roam_node.tags.clone(),
            level: roam_node.level.clone(),
            parent_id: roam_node.parent_id.clone(),
            url: url,
        }
    }
}

#[derive(Debug)]
pub struct KnowledgeGraph {
    pub graph: DiGraph<GraphNode, EdgeType>,
    pub id_to_index: HashMap<String, NodeIndex>,
    pub id_to_url: HashMap<String, String>,
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self {
            graph: DiGraph::default(),
            id_to_index: HashMap::default(),
            id_to_url: HashMap::default(),
        }
    }
}

pub struct SiteBuilder {
    config: SiteConfig,
    // plugin? search?

    // state during processing： parent_stack during `build()` to get parent page
    parent_stack: Vec<PageId>,
    // state during processing: pages during `build()' for output and get parent page to set children_ids
    pages: HashMap<PageId, Page>,
}
impl Default for SiteBuilder {
    fn default() -> Self {
        Self {
            config: SiteConfig::default(),
            parent_stack: vec![],
            pages: HashMap::new(),
        }
    }
}
impl SiteBuilder {
    pub fn new(config: SiteConfig) -> Self {
        Self {
            config,
            parent_stack: vec![],
            pages: HashMap::new(),
        }
    }

    fn process_document(&mut self, document: &Document) -> PageId {
        tracing::trace!(
            "parent_stack={:?}, doc title={:?} path={:?}",
            self.parent_stack,
            document.metadata.title,
            document.html_path()
        );

        let ast = document.ast.clone();
        let syntax_tree = document.syntax_tree.clone();
        let mut hasher = blake3::Hasher::new();
        hasher.update(format!("{:?}", ast).as_bytes());
        let id = format!("{}", hasher.finalize().to_hex());
        let title = document
            .metadata
            .title
            .clone()
            .unwrap_or("no title found".to_string());
        let last_modified_ts = document.metadata.last_modified_ts.clone();
        let created_ts = document.metadata.created_ts.clone();

        let url = format!("/{}", document.html_path());
        let metadata = PageMetadata {};

        let parent_id = self.parent_stack.last().cloned();
        if let Some(ref parent_id_) = parent_id {
            // at the same time update children_ids for the parent page
            self.pages
                .get_mut(parent_id_)
                .unwrap()
                .children_ids
                .push(id.clone());
        }
        let children_ids = vec![];

        let prev_sibling_id = None;
        let next_sibling_id = None;
        let next_flattened_id = None;
        let prev_flattened_id = None;

        let tags = document
            .metadata
            .filetags
            .clone()
            .into_iter()
            .collect::<HashSet<String>>();
        let category = document.metadata.category.clone();

        self.pages.insert(
            id.clone(),
            Page {
                id: id.clone(),
                title,
                url,
                metadata,
                ast,
                syntax_tree,
                tags,
                category,
                parent_id,
                children_ids,
                prev_sibling_id,
                next_sibling_id,
                next_flattened_id,
                prev_flattened_id,
                html_path: document.html_path(),
                created_ts,
                last_modified_ts,
            },
        );

        id
    }

    fn process_section(&mut self, section: &Section) -> Option<PageId> {
        // index page -> other pages
        // documents should be placed in above order!
        let mut index_page_id = None;
        let mut n_index_page: usize = 0;
        for doc in section.documents.iter() {
            if doc.metadata.enable_render {
                if doc.file_info.maybe_index {
                    let id = self.process_document(doc);
                    index_page_id = Some(id.clone());
                    n_index_page = n_index_page + 1;
                    self.parent_stack.push(id);
                } else {
                    self.process_document(doc);
                }
            }
        }
        if n_index_page != 1 {
            tracing::warn!(
                "{} index pages found in section {:?} (should be 1, maybe 0)",
                n_index_page,
                section.file_info.relative_path
            );
        }

        for subsection in section.subsections.iter() {
            self.process_section(&subsection);
        }

        for _ in 0..n_index_page {
            self.parent_stack.pop();
        }

        index_page_id
    }

    // copy and process static assets
    fn process_static_assets(
        &mut self,
        root_section: &Section,
    ) -> std::io::Result<Vec<(PathBuf, PathBuf)>> {
        let mut static_assets = vec![];

        // static
        let static_directory_from = root_section
            .file_info
            .full_path
            .parent()
            .expect("must have parent directory")
            .join("static");
        let static_directory_to = Path::new(&self.config.output_directory);
        if static_directory_from.is_dir() {
            tracing::debug!(from=?static_directory_from.display(), to=?static_directory_to.display());

            let mut options = CopyOptions::new();
            options.overwrite = false; // Overwrite existing files
            options.copy_inside = false;
            options.content_only = true;

            copy(&static_directory_from, &static_directory_to, &options)
                .expect(format!("copy failed from {}", static_directory_from.display()).as_str());
            static_assets.push((static_directory_from, static_directory_to.to_path_buf()));
        }
        std::fs::copy(
            "src/export/ssg/static/default.css",
            static_directory_to.join("default.css"),
        )?;

        // sass

        // non-org file in content
        let directory = &root_section.file_info.full_path;
        for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
            if entry.metadata().unwrap().is_file() {
                let from = entry.path();

                let mut is_in_content = false;
                let mut relative_directories = vec![];
                for section in from.parent().unwrap().components() {
                    let component = section.as_os_str().to_string_lossy();
                    if is_in_content {
                        relative_directories.push(component.to_string());
                    } else if component == "content" {
                        is_in_content = true;
                    }
                }

                let from_filename = from.file_name().expect("xx").to_string_lossy().to_string();
                if from.is_file()
                    && from.extension() != Some(std::ffi::OsStr::new("org"))
                    && (!from_filename.starts_with(&['.', '#']))
                    && (!from_filename.ends_with("_ast.json"))
                    && (!from_filename.ends_with("_syntax.json"))
                {
                    let to_directory = Path::new(&self.config.output_directory)
                        .join(relative_directories.join("/"));
                    if !to_directory.is_dir() {
                        std::fs::create_dir_all(&to_directory)?;
                    }

                    let to = to_directory.join(from.file_name().unwrap());
                    tracing::trace!(from=?from, to=?to, "copy");
                    std::fs::copy(from, &to)?;
                    static_assets.push((from.to_path_buf(), to.to_path_buf()));
                }
            }
        }

        Ok(static_assets)
    }

    fn preorder_dfs_traverse(&self, root_page_id: &PageId) -> Vec<PageId> {
        let mut result = vec![];
        self.preorder_helper(root_page_id, &mut result);
        result
    }

    fn preorder_helper(&self, page_id: &PageId, result: &mut Vec<PageId>) {
        result.push(page_id.into());
        for child in self.pages.get(page_id).unwrap().children_ids.iter() {
            self.preorder_helper(child, result);
        }
    }

    // next_flattened_id/prev_flattened_id
    // next_sibling_id/prev_sibling_id
    fn establish_sibling_flatten_links(&mut self, root_page_id: &PageId) {
        // update next_flattened_id and prev_flattened_id
        let flatten_page_ids = self.preorder_dfs_traverse(root_page_id);
        tracing::debug!("faltten_page_ids={:?}", flatten_page_ids);
        let idx_start = if Site::is_faked_root(root_page_id) {
            1
        } else {
            0
        };
        for idx in (idx_start + 1)..flatten_page_ids.len() {
            self.pages
                .get_mut(&flatten_page_ids[idx])
                .expect("todo")
                .prev_flattened_id = Some(
                self.pages
                    .get(&flatten_page_ids[idx - 1])
                    .expect("todo")
                    .id
                    .clone(),
            );
        }
        for idx in (idx_start)..flatten_page_ids.len() - 1 {
            self.pages
                .get_mut(&flatten_page_ids[idx])
                .expect("todo")
                .next_flattened_id = Some(
                self.pages
                    .get(&flatten_page_ids[idx + 1])
                    .expect("todo")
                    .id
                    .clone(),
            );
        }

        // update next_sibling_id and prev_sibling_id
        let mut siblings = vec![];
        for (_id, page) in self.pages.iter() {
            if page.children_ids.len() >= 2 {
                siblings.push(page.children_ids.clone());
            }
        }
        for sibling in siblings.into_iter() {
            for idx in 1..sibling.len() {
                self.pages
                    .get_mut(&sibling[idx])
                    .expect("todo")
                    .prev_sibling_id =
                    Some(self.pages.get(&sibling[idx - 1]).expect("todo").id.clone());
            }
            for idx in 0..(sibling.len() - 1) {
                self.pages
                    .get_mut(&sibling[idx])
                    .expect("todo")
                    .next_sibling_id =
                    Some(self.pages.get(&sibling[idx + 1]).expect("todo").id.clone());
            }
        }
    }

    fn build_knowledge_graph(&self, root_section: &Section) -> KnowledgeGraph {
        let mut graph = DiGraph::<GraphNode, EdgeType>::new();
        let mut id_to_index: HashMap<String, NodeIndex> = HashMap::new();
        let mut refs_to_id: HashMap<String, String> = HashMap::new();

        fn build_section(
            section: &Section,
            mut graph: &mut DiGraph<GraphNode, EdgeType>,
            mut id_to_index: &mut HashMap<String, NodeIndex>,
            mut refs_to_id: &mut HashMap<String, String>,
        ) {
            for document in section.documents.iter() {
                for node in document.ast.roam_nodes.iter() {
                    let url = format!("/{}#{}", document.html_path(), node.id);
                    let graph_node = GraphNode::from(node, url);
                    let index = graph.add_node(graph_node.clone());
                    id_to_index.insert(graph_node.id.clone(), index);
                    for refs in graph_node.refs.iter() {
                        refs_to_id.insert(refs.clone(), graph_node.id.clone());
                    }
                }

                for node in document.ast.roam_nodes.iter() {
                    if let Some(parent_id) = &node.parent_id {
                        if let Some(current_index) = id_to_index.get(node.id.as_str()) {
                            if let Some(parent_index) = id_to_index.get(parent_id.as_str()) {
                                graph.add_edge(*parent_index, *current_index, EdgeType::Parent {});
                            }
                        }
                    }

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
                                        if !graph.contains_edge(*source_index, *target_index) {
                                            graph.add_edge(
                                                *source_index,
                                                *target_index,
                                                EdgeType::ExplicitReference {
                                                    source_path: extracted_link.source_path.clone(),
                                                },
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            for subsection in &section.subsections {
                build_section(subsection, &mut graph, &mut id_to_index, &mut refs_to_id);
            }
        }

        build_section(root_section, &mut graph, &mut id_to_index, &mut refs_to_id);

        let mut id_to_url = HashMap::<String, String>::new();
        for (id, index) in id_to_index.iter() {
            let node = &graph[*index];
            id_to_url.insert(id.clone(), node.url.clone());
        }

        KnowledgeGraph {
            id_to_index,
            graph,
            id_to_url,
        }
    }

    /// Build Site from
    pub fn build(&mut self, root_section: &Section) -> std::io::Result<Site> {
        self.pages.clear();

        tracing::debug!("  build page tree ...");

        // render page: general link, needs to lookup roam_id -> url
        // render knowlege graph: document html_path

        // todo:
        let knowledge_graph = self.build_knowledge_graph(root_section);
        // 处理所有页面中的 org-roam 链接，构建图
        // site.build_roam_graph();
        // let g = section.build_graph();
        // let g_dot = Dot::new(&g.graph);
        // tracing::debug!("Basic DOT format:\n{:?}\n", g_dot);
        // tracing::debug!("{:#?}", g.graph);
        // 基于图关系为每个页面预计算“相关页面”
        // site.precompute_related_pages();

        let maybe_root_page_id = self.process_section(root_section);
        let root_page_id = if let Some(root_page_id) = maybe_root_page_id.clone() {
            root_page_id
        } else {
            // fake a root page
            let children_ids = self
                .pages
                .iter()
                .filter(|(_id, page)| page.parent_id.is_none())
                .map(|(id, _)| id.to_string())
                .collect::<Vec<_>>();
            let faked_root_page_id = PageId::from("FAKED_ROOT_ID");
            let faked_root = Page::faked(faked_root_page_id.clone(), children_ids); // root_page -children-> children
            for (_id, page) in self.pages.iter_mut() {
                if page.parent_id.is_none() {
                    page.parent_id = Some(faked_root_page_id.clone()); // root_page <-parent- children
                }
            }
            // insert the faked root page to self.pages
            self.pages.insert(faked_root_page_id.clone(), faked_root);

            faked_root_page_id
        };
        self.establish_sibling_flatten_links(&root_page_id);

        // build a graph: root is index_page id or faked_root
        // dfs to get flattened_pages? // toc?

        tracing::debug!("  build tag-index: tag -> page_id ...");
        let mut tag_index: HashMap<String, Vec<PageId>> = HashMap::new();
        for (page_id, page) in self.pages.iter() {
            for tag in page.tags.iter() {
                if tag_index.contains_key(tag) {
                    tag_index.get_mut(tag).unwrap().push(page_id.to_string());
                } else {
                    tag_index.insert(tag.to_string(), vec![page_id.to_string()]);
                }
            }
        }
        tracing::trace!("tag_index: {:?}", tag_index);

        tracing::debug!("  process static assets ...");
        let static_assets = self.process_static_assets(root_section)?;

        let mut pageid_to_url: HashMap<PageId, String> = HashMap::new();
        for (id, page) in self.pages.iter() {
            pageid_to_url.insert(id.clone(), page.url.clone());
        }

        let site = Site {
            config: self.config.clone(),
            pages: self.pages.clone(),
            root_page_id,
            pageid_to_url,
            knowledge_graph,
            tag_index,
            _static_assets: static_assets,
            ..Site::default()
        };

        self.pages.clear();

        Ok(site)
    }
}
