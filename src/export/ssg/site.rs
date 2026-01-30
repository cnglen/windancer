/// Content Model, incluing Site and Page
use std::collections::{HashMap, HashSet};

use std::fmt;

use crate::compiler::content::{Document, Section};

#[derive(Debug, Clone)]
pub struct PageMetadata {}

type PageId = String;

use crate::compiler::ast_builder::element::OrgFile;

/// Content Model
#[derive(Clone)]
pub struct Page {
    pub id: PageId,

    pub title: String,
    pub url: String,
    pub metadata: PageMetadata,
    pub ast: OrgFile,

    // tree: directory/section tree
    // 层级导航，树形结构，生成侧边栏目录、面包屑
    pub parent_id: Option<PageId>,
    pub children_ids: Vec<PageId>,
    // 兄弟导航，父节点下的线性链表，章节内“上一节/下一节”
    pub prev_id: Option<PageId>,
    pub next_id: Option<PageId>,

    pub tags: HashSet<String>,
    pub category: Vec<String>,

    // flat global navigation
    // 全局扁平导航,全站深度优先序列,博客式“上一篇/下一篇”，跨章节连续阅读
    pub next_flattened_id: Option<PageId>,
    pub prev_flattened_id: Option<PageId>,
    // is_index?
}

// impl Site {
//     pub fn build_from_section(root_section: &Section, config: &ExportConfig) -> Self {
//         let mut site = Site::new();
//         // 1. 首先，像之前一样构建页面树和基本导航
//         let root_page_id = site.process_section(root_section, None, config);
//         site.root_page_id = Some(root_page_id);
//         site.establish_sibling_links();

//         // 2. 然后，处理所有页面中的 org-roam 链接，构建图
//         site.build_roam_graph();

//         // 3. 最后，基于图关系为每个页面预计算“相关页面”
//         site.precompute_related_pages();

//         site
//     }

//     fn build_roam_graph(&mut self) {
//         // 第一遍：建立 roam_id 到 page_id 的映射
//         for (page_id, page) in &self.pages {
//             if let Some(ref roam_id) = page.metadata.roam_id {
//                 self.roam_id_to_page_id.insert(roam_id.clone(), *page_id);
//             }
//         }

//         // 第二遍：解析链接，在图中添加边
//         for (source_page_id, source_page) in &self.pages {
//             for raw_link in &source_page.metadata.raw_links {
//                 if let RawLink::RoamId { id } = raw_link {
//                     if let Some(&target_page_id) = self.roam_id_to_page_id.get(id) {
//                         // 添加一条从源页面指向目标页面的边
//                         self.graph.add_edge(*source_page_id, target_page_id, LinkType::DirectLink);
//                         // 可选：同时添加一条反向边，或将反向链接单独存储为 Backlink 类型
//                     }
//                 }
//             }
//         }
//     }
// }
// // 链接类型，可用于在图中区分不同关系
// #[derive(Debug, Clone)]
// pub enum LinkType {
//     DirectLink,    // 明确的双向链接
//     Backlink,      // 反向链接（可自动推导）
//     Mention,       // 提及（可能通过文本分析得到）
// }

// // 预计算的相关页面信息
// pub struct RelatedPage {
//     pub page_id: PageId,
//     pub link_type: LinkType,
//     pub snippet: Option<String>, // 可选的上下文摘要
// }

// impl Site {
//     /// 在构建 Site 后，调用此方法建立标签索引
//     pub fn build_tag_index(&mut self) {
//         self.tag_index.clear();
//         for (page_id, page) in &self.pages {
//             for tag in &page.tags {
//                 self.tag_index
//                     .entry(tag.clone())
//                     .or_insert_with(Vec::new)
//                     .push(*page_id);
//             }
//         }
//         // 对每个标签下的页面列表进行排序（例如按日期）
//         for page_ids in self.tag_index.values_mut() {
//             page_ids.sort_by_key(|&id| {
//                 self.pages.get(&id).and_then(|p| p.metadata.date).unwrap_or_default()
//             });
//         }
//     }

//     /// 根据标签获取相关页面
//     pub fn get_pages_by_tag(&self, tag: &str) -> Option<Vec<&Page>> {
//         self.tag_index.get(tag).map(|ids| {
//             ids.iter().filter_map(|id| self.pages.get(id)).collect()
//         })
//     }

//     /// 生成所有标签的聚合页（可在导出阶段调用）
//     pub fn generate_tag_pages(&self) -> HashMap<String, Page> {
//         let mut tag_pages = HashMap::new();
//         for (tag, page_ids) in &self.tag_index {
//             // 为每个标签创建一个虚拟的“聚合页”
//             let tag_page = Page {
//                 id: PageId(usize::MAX), // 使用特殊ID或专门生成
//                 title: format!("Tag: {}", tag),
//                 relative_url: format!("/tags/{}/", tag),
//                 content: self.render_tag_page(tag, page_ids), // 渲染逻辑
//                 tags: HashSet::new(),
//                 // ... 其他字段
//             };
//             tag_pages.insert(tag.clone(), tag_page);
//         }
//         tag_pages
//     }
// }

// impl Site {
//     /// 建立扁平化导航顺序（例如，深度优先）
//     pub fn build_flattened_order(&mut self) {
//         self.flattened_order.clear();
//         if let Some(root_id) = self.root_page_id {
//             self.dfs_traverse(root_id);
//             // 基于遍历结果，为每个页面设置 prev_flattened_id 和 next_flattened_id
//             self.set_flattened_navigation();
//         }
//     }

//     fn dfs_traverse(&mut self, current_page_id: PageId) {
//         if let Some(page) = self.pages.get(&current_page_id) {
//             // 1. 首先访问当前页面
//             self.flattened_order.push(current_page_id);
//             // 2. 然后递归访问所有子页面（按 children_ids 顺序）
//             for &child_id in &page.children_ids {
//                 self.dfs_traverse(child_id);
//             }
//             // (如果是后序遍历，则将 `push` 操作移到递归之后)
//         }
//     }

//     fn set_flattened_navigation(&mut self) {
//         // 清空现有关系
//         for page in self.pages.values_mut() {
//             page.next_flattened_id = None;
//             page.prev_flattened_id = None;
//         }
//         // 根据顺序列表设置关系
//         for (i, &page_id) in self.flattened_order.iter().enumerate() {
//             if let Some(page) = self.pages.get_mut(&page_id) {
//                 if i > 0 {
//                     page.prev_flattened_id = Some(self.flattened_order[i - 1]);
//                 }
//                 if i + 1 < self.flattened_order.len() {
//                     page.next_flattened_id = Some(self.flattened_order[i + 1]);
//                 }
//             }
//         }
//     }

//     /// 获取当前页面的“下一篇”（扁平化顺序）
//     pub fn get_next_flattened(&self, page_id: PageId) -> Option<&Page> {
//         self.pages.get(&page_id)
//             .and_then(|p| p.next_flattened_id)
//             .and_then(|id| self.pages.get(&id))
//     }
// }

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

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SiteConfig {
    pub output_dir: PathBuf,
    // pub base_url: String,
    // pub theme: String,
    // pub generate_search_index: bool,
    // ... 其他配置
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            output_dir: "public".into(),
        }
    }
}

/// Content model
#[derive(Debug)]
pub struct Site {
    pub config: SiteConfig,
    pub pages: HashMap<PageId, Page>,
    pub url_to_page_id: HashMap<String, PageId>,
    pub root_page_id: Option<PageId>,

    // build_tag_index(), get_pages_by_tag，generate_tag_pages
    pub tag_index: HashMap<String, Vec<PageId>>,
    pub flattened_pages: Vec<PageId>,
    // pub knowledge_graph: RoamGraph? // 更好的可视化js?
    // // 🔥 核心图结构
    // // 使用 PageId 作为图的节点，边的类型可以自定义（如：引用、提及、相关）
    // pub graph: petgraph::graph::DiGraph<PageId, LinkType>;

    // // 快速查找：从 org-roam id 到 page id 的映射
    // pub roam_id_to_page_id: HashMap<String, PageId>;

    // // 🔥 为每个页面预计算的相关页面列表（用于渲染，避免实时遍历图）
    // pub related_pages: HashMap<PageId, Vec<RelatedPage>>,
}

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

use crate::export::ssg::toc::TocNode;

use super::toc::TableOfContents;

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
        }
    }
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
        tracing::info!(
            "parent_stack={:?}, doc title={:?} path={:?}",
            self.parent_stack,
            document.metadata.title,
            document.html_path()
        );
        let ast = document.ast.clone();
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

        let prev_id = None;
        let next_id = None;

        let tags = document
            .metadata
            .filetags
            .clone()
            .into_iter()
            .collect::<HashSet<String>>();
        let category = document.metadata.category.clone();

        let next_flattened_id = None;
        let prev_flattened_id = None;

        self.pages.insert(
            id.clone(),
            Page {
                id: id.clone(),
                title,
                url,
                metadata,
                ast,
                parent_id,
                children_ids,
                prev_id,
                next_id,
                tags,
                category,
                next_flattened_id,
                prev_flattened_id,
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

    pub fn build(&mut self, root_section: &Section) -> std::io::Result<Site> {
        self.pages.clear();

        let root_page_id = self.process_section(root_section);

        let pages = self.pages.clone();

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

        // build a graph: root is index_page id or faked_root
        // dfs to get flattened_pages? // toc?

        let site = Site {
            config: self.config.clone(),
            root_page_id,
            pages,
            ..Site::default()
        };

        self.pages.clear();

        Ok(site)
    }
}
