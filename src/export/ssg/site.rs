use std::cell::RefCell;
/// Content Model, where Site is composed of Pages and
/// - Site := Page + ... + Page
/// - Section -> SiteBuilder -> Site
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use fs_extra::dir::{CopyOptions, copy};
use walkdir::WalkDir;

use crate::compiler::ast_builder::element::OrgFile;
use crate::compiler::content::{Document, Section};
use crate::compiler::parser::syntax::{OrgSyntaxKind, SyntaxNode};
use crate::export::ssg::toc::{TableOfContents, TocNode};

#[derive(Clone)]
pub struct Page {
    pub id: PageId,

    pub title: String,
    pub url: String,
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

    // is_index?

    // return the html path relatie to root of site, such as "blog/foo/index.html"
    pub html_path: String,
}

impl Page {
    // fn faked(id: PageId, children_ids: Vec<PageId>) -> Self {
    //     Self {
    //         id,
    //         children_ids,

    //         title: String::default(),
    //         url: String::default(),
    //         metadata: PageMetadata {  },
    //         ast: OrgFile {
    //             zeroth_section: None,
    //             heading_subtrees: vec![],
    //             footnote_definitions: vec![],
    //             keywords: BTreeMap::default(),
    //             properties: BTreeMap::default(),
    //             extracted_links: vec![],
    //             roam_nodes: vec![]
    //         },
    //         syntax_tree: crate::node!(OrgSyntaxKind::Root, vec![]),

    //         parent_id: None,

    //         prev_sibling_id: None,
    //         next_sibling_id: None,
    //         prev_flattened_id: None,
    //         next_flattened_id: None,
    //         tags: HashSet::default(),
    //         category: vec![],
    //         html_path: String::default()
    //     }
    // }
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
    // ... 其他配置
}
impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            output_directory: "public".into(),
        }
    }
}

#[derive(Debug)]
pub struct Site {
    pub config: SiteConfig,
    pub pages: HashMap<PageId, Page>,
    pub url_to_page_id: HashMap<String, PageId>,
    pub root_page_id: Option<PageId>,

    // build_tag_index(), get_pages_by_tag，generate_tag_pages
    pub tag_index: HashMap<String, Vec<PageId>>,
    pub flattened_pages: Vec<PageId>,

    // static assets: including css/image/fonts.
    static_assets: Vec<(PathBuf, PathBuf)>,
    // pub knowledge_graph: RoamGraph? // 更好的可视化js?
    // // 🔥 核心图结构
    // // 使用 PageId 作为图的节点，边的类型可以自定义（如：引用、提及、相关）
    // pub graph: petgraph::graph::DiGraph<PageId, LinkType>;

    // // 快速查找：从 org-roam id 到 page id 的映射
    // pub roam_id_to_page_id: HashMap<String, PageId>;

    // // 🔥 为每个页面预计算的相关页面列表（用于渲染，避免实时遍历图）
    // pub related_pages: HashMap<PageId, Vec<RelatedPage>>,
}
impl Site {
    fn get_toc_of_page(&self, page_id: &PageId) -> TocNode {
        let page = self.pages.get(page_id).unwrap();
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
        let root_nodes = if let Some(root) = self.root_page_id.clone() {
            let root_toc = self.get_toc_of_page(&root);
            root_toc.children
        } else {
            let mut children = vec![];
            for id in self
                .pages
                .iter()
                .filter(|(_, page)| page.parent_id.is_none())
                .map(|(id, _)| id)
            {
                children.push(self.get_toc_of_page(&id));
            }
            children
        };

        TableOfContents { root_nodes }
    }
}
impl Default for Site {
    fn default() -> Self {
        Self {
            config: SiteConfig::default(),
            pages: HashMap::new(),
            url_to_page_id: HashMap::new(),
            root_page_id: None,
            tag_index: HashMap::new(),
            flattened_pages: vec![],
            static_assets: vec![],
        }
    }
}

pub struct SiteBuilder {
    // template_engine: Tera,
    // resource_processor: ResourceProcessor, // // 资源处理（图片、CSS等）
    config: SiteConfig,
    // 可能还有插件系统、图关系构建器等

    // state during processing
    // parent_stack during `build()` for get parent page
    parent_stack: Vec<PageId>,
    // pages during `build()' for output and get parent page to set children_ids
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

        let url = document.html_path();
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
            if doc.file_info.maybe_index {
                let id = self.process_document(doc);
                index_page_id = Some(id.clone());
                n_index_page = n_index_page + 1;
                self.parent_stack.push(id);
            } else {
                self.process_document(doc);
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

            copy(&static_directory_from, &static_directory_to, &options).expect("copy failed");
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

    /// Build Site from
    pub fn build(&mut self, root_section: &Section) -> std::io::Result<Site> {
        self.pages.clear();

        tracing::debug!("build page tree");
        let root_page_id = self.process_section(root_section);

        // get prev_sibling/prev_faltten
        let mut pages = self.pages.clone();
        let root = if let Some(root) = root_page_id.clone() {
            root
        } else {
            let faked_root = PageId::from("FAKED_ROOT_ID");
            for (_id, page) in pages.iter_mut() {
                if page.parent_id.is_none() {
                    page.parent_id = Some(faked_root.clone());
                }
            }
            faked_root
        };

        fn preorder(root_page_id: PageId, pages: &HashMap<String, Page>) -> Vec<PageId> {
            let mut result = vec![];
            preorder_helper(&root_page_id, &mut result, pages);
            result
        }

        fn preorder_helper(
            page_id: &PageId,
            result: &mut Vec<PageId>,
            pages: &HashMap<String, Page>,
        ) {
            result.push(page_id.into());
            for child in pages.get(page_id).unwrap().children_ids.iter() {
                preorder_helper(child, result, pages);
            }
        }

        let ans = preorder(root, &pages);
        tracing::info!("ans={:?}", ans);
        // // , pages: HashMap<String, Page>
        // fn preorder(root_page: Option<Rc<RefCell<Page>>>) -> Vec<PageId> {
        //     let mut result = vec![];
        //     preorder_helper(&root_page, &mut result);
        //     result
        // }

        // fn preorder_helper(node: &Option<Rc<RefCell<Page>>>, result: &mut Vec<PageId>) {
        //     if let Some(page) = node {
        //         let page = page.borrow();
        //         result.push(page.id);

        //         for child in page.children_ids {
        //             preorder_helper(&child, result);
        //         }
        //     }
        // }

        let pages = self.pages.clone();
        //         site.establish_sibling_links();
        // build a graph: root is index_page id or faked_root
        // dfs to get flattened_pages? // toc?

        tracing::debug!("build tag-index: tag -> page_id");
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

        let static_assets = self.process_static_assets(root_section)?;

        // todo:
        // 处理所有页面中的 org-roam 链接，构建图
        // site.build_roam_graph();

        // let g = section.build_graph();
        // let g_dot = Dot::new(&g.graph);
        // tracing::debug!("Basic DOT format:\n{:?}\n", g_dot);
        // tracing::debug!("{:#?}", g.graph);

        //         // 3. 最后，基于图关系为每个页面预计算“相关页面”
        //         site.precompute_related_pages();

        let site = Site {
            config: self.config.clone(),
            root_page_id,
            pages,
            static_assets,
            ..Site::default()
        };

        self.pages.clear();

        Ok(site)
    }
}
