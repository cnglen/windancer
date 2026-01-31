// 构建RoamNode的Graph
// id -> page
// id -> page#link
// 每个node可视化, 存储html? 从page中获取html

// compile增加roamnode的解析: ast + roam_graph
// file node <-> page map: page_id, anchor
// RoamNode和Document的关系?
// - File: 通过FileInfo
// - Headline: SubTree with ID

use std::collections::{BTreeMap, HashMap};

use petgraph::graph::{DiGraph, NodeIndex};

use crate::compiler::ast_builder::object::Object;
use crate::compiler::ast_builder::{SourcePathSegment, element};
use crate::compiler::content::FileInfo;
use crate::export::ssg::renderer_vold::{Renderer, RendererConfig};

#[derive(Debug, Clone)]
pub enum NodeType {
    File,
    Headline,
}

#[derive(Clone)]
pub enum EdgeType {
    Parent,                                                    // a --> b: a is parent of b
    ExplicitReference { source_path: Vec<SourcePathSegment> }, // [[id:...][]]: a refers b
}

impl fmt::Debug for EdgeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EdgeType::Parent => {
                write!(f, r##""##,)
            }

            EdgeType::ExplicitReference { source_path } => {
                write!(f, r##"{:?}"##, source_path,)
            }
        }
    }
}

pub struct RoamGraph {
    pub graph: DiGraph<RoamNode, EdgeType>,
    pub id_to_index: HashMap<String, NodeIndex>,
}

// 结构关系
//   *父子： 文件节点 → 其下的标题节点。
// 显式链接关系
//   *ID引用关系： 通过 [[id:xxx][描述]] 语法主动创建的链接。这是最核心、最富语义的关系。(遍历Node)
//     A -> a_son  -> a_grandson -> link to b
//   文件链接关系: 通过 [[file:...][描述]] 链接到另一个文件（或锚点）。
// 隐式/派生关系
//   标签共现关系	两个节点共享了相同的标签（#+FILETAGS: 或 #+ROAM_TAGS:）。
//   提及/文本关系	在正文中提及了另一个节点的标题或别名，但未用 [[id:...]] 显式链接。
// 顺序关系
//   同级顺序关系	在同一父节点下，兄弟节点（如多个标题、多个文件）之间的前后顺序。
// 元数据关系           别名指向关系	节点A的别名 (ROAM_ALIAS) 恰好是节点B的标题。这可以视为一种弱引用。
// node -- [id:] -> node1
// node -- links with oram_refs -> node2 ()
// node -- has children  -->
#[derive(Clone)]
pub struct RoamNode {
    pub id: String,
    pub title: Vec<Object>,
    pub node_type: NodeType,
    // pub file_info: FileInfo,    // file info where current node is born
    /// ROAM_ALIASES property
    pub aliases: Vec<String>,

    /// 引用键列表 (ROAM_REFS: citation keys, URLs, DOIs)
    pub refs: Vec<String>,

    /// 属性映射 (Org PROPERTIES)
    pub properties: BTreeMap<String, String>,

    /// filetags for doc; head tags for heading
    pub tags: Vec<String>,

    // note_type: literature, concept
    // has refs -> literature
    /// 0 for NodeType::File, headline level for NodeType::HeadLine
    pub level: u8,

    pub parent_id: Option<String>,
    // /// 节点原始内容 (Org-mode 格式)
    // pub raw_ast: RawAst, // OrgFile Or HeadingSubtree

    // /// 节点渲染后内容 (HTML/Markdown)?
    // pub rendered_content: String,

    // /// 创建时间戳
    // pub created: DateTime<Utc>,

    // /// 最后修改时间戳
    // pub modified: DateTime<Utc>,

    // /// 元数据哈希 (用于增量更新检测)
    // pub content_hash: String,

    // pub page: Page,
    // pub anchor: Option<String>, // page.link + #xxxxx; page.link
    // pub link: String,           // url link
}

use std::fmt;

impl fmt::Debug for RoamNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r##"{:?}:{:?}"##,
            self.node_type,
            self.title
                .iter()
                .map(|o| Renderer::new(RendererConfig::default()).render_object(o))
                .collect::<Vec<_>>()
                .join(""),
        )
    }
}
