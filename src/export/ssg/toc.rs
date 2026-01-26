use crate::compiler::ast_builder::element::HeadingSubtree;
use crate::compiler::content::{Document, Section};
use std::collections::{HashMap, HashSet};

// Site:
// Page

pub struct PageMetadata {}

pub struct PageId(pub usize); // or hash?

pub struct Page {
    pub id: PageId,

    pub title: String,
    pub url: String,
    pub metadata: PageMetadata,

    // html
    pub content: String,

    // tree: directory/section tree
    // 层级导航，树形结构，生成侧边栏目录、面包屑
    pub parent_id: Option<PageId>,
    pub children_ids: Vec<PageId>,

    // 兄弟导航，父节点下的线性链表，章节内“上一节/下一节”
    pub prev_id: Option<PageId>,
    pub next_id: Option<PageId>,

    pub tags: HashSet<String>,
    pub category: Option<String>,

    // flat global navigation
    // 全局扁平导航,全站深度优先序列,博客式“上一篇/下一篇”，跨章节连续阅读
    pub next_flattened_id: Option<PageId>,
    pub prev_flattened_id: Option<PageId>,
}

pub struct Site {
    pub pages: HashMap<PageId, Page>,
    pub url_to_page_id: HashMap<String, PageId>,
    pub root_page_id: Option<PageId>,

    // build_tag_index(), get_pages_by_tag，generate_tag_pages
    pub tag_index: HashMap<String, Vec<PageId>>,
    pub flattened_pages: Vec<PageId>,
    // // 🔥 核心图结构
    // // 使用 PageId 作为图的节点，边的类型可以自定义（如：引用、提及、相关）
    // pub graph: petgraph::graph::DiGraph<PageId, LinkType>;

    // // 快速查找：从 org-roam id 到 page id 的映射
    // pub roam_id_to_page_id: HashMap<String, PageId>;

    // // 🔥 为每个页面预计算的相关页面列表（用于渲染，避免实时遍历图）
    // pub related_pages: HashMap<PageId, Vec<RelatedPage>>,
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
            if subsection.documents.len() > 0 {
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
    pub flatten_nodes: Vec<TocNode>,
    pub root_nodes: Vec<TocNode>, // not flatten
}

impl TableOfContents {
    pub fn new(root_nodes: Vec<TocNode>) -> Self {
        fn flatten(node: &TocNode) -> Vec<TocNode> {
            let mut ans = vec![];
            ans.push(node.clone());

            for child in &node.children {
                ans.extend(flatten(&child));
            }
            ans
        }

        let mut flatten_nodes = vec![];
        for node in &root_nodes {
            flatten_nodes.extend(flatten(node));
        }

        Self {
            root_nodes,
            flatten_nodes,
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
                    html.push_str("\n<ul>\n");
                    for child in &node.children {
                        node_to_html(child, active_slug, html, max_depth);
                    }
                    html.push_str("</ul>\n");
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
