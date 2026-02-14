//! AST definition for element in org-mode
use std::collections::BTreeMap;
use std::fmt;

use bincode;
use serde::{Deserialize, Serialize};

use crate::compiler::ast_builder::ExtractedLink;
use crate::compiler::ast_builder::object::Object;
use crate::compiler::org_roam::RoamNode;

pub(crate) trait Id {
    fn id(&self) -> String
    where
        Self: Serialize,
    {
        let bytes = bincode::serialize(self).expect("serialize failed");
        let mut hasher = blake3::Hasher::new();
        hasher.update(&bytes);
        hasher.finalize().to_hex().to_string()
    }
}

#[derive(Clone, Serialize)]
pub struct OrgFile {
    pub zeroth_section: Option<Section>,
    pub heading_subtrees: Vec<HeadingSubtree>,

    // metadata collected durbing build AST

    // render Footnotes HeadingSubtree
    pub footnote_definitions: Vec<FootnoteDefinition>,
    /// keywords from org file to extract title, etc.
    pub keywords: BTreeMap<String, Vec<Object>>,
    /// properties from zeroth section to extract ID, etc.
    pub properties: BTreeMap<String, String>,
    /// Extracted links including full path for roam based knowledge graph
    pub extracted_links: Vec<ExtractedLink>,
    /// Roam Nodes
    pub roam_nodes: Vec<RoamNode>,
}

impl Id for OrgFile {}

impl fmt::Debug for OrgFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r##"OrgFile {{
zeroth_secton: {:#?},
heading_subtrees: {:#?},
footnote_definitions: {:#?},
properties: {:#?}
keywords: {:#?},
extracted_links: {:#?},
roam_nodes: {:#?},
}}"##,
            self.zeroth_section,
            self.heading_subtrees,
            self.footnote_definitions,
            self.properties,
            self.keywords,
            self.extracted_links,
            self.roam_nodes
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HeadingSubtree {
    // heading row info
    pub level: u8,
    pub keyword: Option<String>,
    pub priority: Option<String>,
    pub is_commented: bool,
    pub title: Vec<Object>,
    pub tags: Vec<String>,

    pub planning: Option<Planning>,
    pub property_drawer: Option<PropertyDrawer>,
    pub section: Option<Section>,
    pub sub_heading_subtrees: Vec<HeadingSubtree>,
    pub properties: BTreeMap<String, String>,
}

impl Id for HeadingSubtree {}

// todo: if key duplicated?
pub(crate) fn get_properties(property_drawer: &Option<PropertyDrawer>) -> BTreeMap<String, String> {
    let mut properties: BTreeMap<String, String> = BTreeMap::new();
    if let Some(drawer) = property_drawer {
        for property in drawer.contents.iter() {
            if let Some(value) = property.value.clone() {
                properties.insert(property.name.to_ascii_uppercase(), value);
            }
        }
    }
    properties
}

impl fmt::Debug for HeadingSubtree {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r##"HeadingSubtree {{
    level: {:#?},
    keyword: {:#?},
    priority: {:#?},
    is_commented: {:#?},
    title: {:#?},
    tags: {:#?},
    planning: {:#?},
    property_drawer: {:#?},
    properties: {:#?},

    section: {:#?},
    sub_heading_subtrees: {:#?}
}}"##,
            self.level,
            self.keyword,
            self.priority,
            self.is_commented,
            self.title,
            self.tags,
            self.planning,
            self.property_drawer,
            self.properties,
            self.section,
            self.sub_heading_subtrees
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Section {
    pub elements: Vec<Element>,
}
impl fmt::Debug for Section {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r##"Section {{
    elements: {:#?}
}}"##,
            self.elements
        )
    }
}

// 块级元素（Block-level elements）： Greater Or Lesser Element
// 第一个Table := Element:Table表示枚举，第二个Table表示该枚举所带的数据的类型(结构体)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Element {
    // Greater
    Table(Table),
    Drawer(Drawer),
    PropertyDrawer(PropertyDrawer),
    CenterBlock(CenterBlock),
    QuoteBlock(QuoteBlock),
    SpecialBlock(SpecialBlock),
    // DynamicBlock(DynamicBlock),
    List(List),
    Item(Item),
    FootnoteDefinition(FootnoteDefinition),

    ZerothSectionPreamble(ZerothSectionPreamble), // todo

    // Lesser Element
    Paragraph(Paragraph),
    SrcBlock(SrcBlock),
    CommentBlock(CommentBlock),
    VerseBlock(VerseBlock),
    ExampleBlock(ExampleBlock),
    ExportBlock(ExportBlock),
    HorizontalRule(HorizontalRule),
    LatexEnvironment(LatexEnvironment),
    Keyword(Keyword),
    AffiliatedKeyword(AffiliatedKeyword),
    FixedWidth(FixedWidth),
    NodeProperty(NodeProperty),
    Planning(Planning),

    TableRow(TableRow),
    // BabelCall(BabelCall),
    Comment(Comment),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZerothSectionPreamble {
    pub comment: Option<Comment>,
    pub property_drawer: Option<PropertyDrawer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drawer {
    pub name: String,
    pub contents: Vec<Element>,
    pub affiliated_keywords: Vec<AffiliatedKeyword>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDrawer {
    pub contents: Vec<NodeProperty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeProperty {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Planning {
    pub keyword: String,
    pub timestamp: Object,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: Option<String>, // 表格名称 (#+NAME:)
    pub caption: Vec<Object>, // 表格标题 (#+CAPTION:)
    // pub attributes: TableAttributes,    // 表格属性
    pub header: Vec<TableRow>,       // 表头行（>=0）
    pub separator: Option<TableRow>, // 分隔线行（可选）
    pub rows: Vec<TableRow>,         // 数据行
    pub formulas: Vec<TableFormula>, // 表格公式
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "caption={:?}\nname={:?}\nheader={:?}\nrows={:?}\nformulas={:?}",
            self.caption, self.name, self.header, self.rows, self.formulas
        )
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TableRow {
    pub cells: Vec<Object>,
    pub row_type: TableRowType,
}

impl fmt::Debug for TableRow {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}: {:?}", self.row_type, self.cells)
    }
}

// 行类型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableRowType {
    Header,  // 表头行
    Rule,    // 分隔线行: rule
    Data,    // 数据行
    Formula, // 公式行
}

// // 单元格对齐方式
// #[derive(Debug, Clone, PartialEq, Eq)]
// pub enum CellAlignment {
//     Left,
//     Center,
//     Right,
//     Default,
// }

// // 单元格跨度
// #[derive(Debug, Clone, PartialEq, Eq, Default)]
// pub struct CellSpan {
//     pub rowspan: u32,   // 跨行数
//     pub colspan: u32,   // 跨列数
// }

// 表格公式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableFormula {
    pub data: String,
    // pub target: String,         // 目标单元格/范围
    // pub formula: String,        // 公式内容
    // pub format: Option<String>, // 格式说明
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paragraph {
    pub affiliated_keywords: Vec<AffiliatedKeyword>,
    pub objects: Vec<Object>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct List {
    pub list_type: ListType,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListType {
    Ordered,
    Unordered,
    Descriptive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub bullet: String,
    pub counter_set: Option<String>,
    pub checkbox: Option<String>,
    pub tag: Vec<Object>,
    pub contents: Vec<Element>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CenterBlock {
    pub parameters: Option<String>,
    pub contents: Vec<Element>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteBlock {
    pub parameters: Option<String>,
    pub contents: Vec<Element>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecialBlock {
    pub name: String,
    pub parameters: Option<String>,
    pub contents: Vec<Element>,
}

// Lesser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExampleBlock {
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentBlock {
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerseBlock {
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportBlock {
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrcBlock {
    pub language: String,

    pub switches: Vec<String>,

    pub results: Option<String>,
    pub exports: Option<String>,
    pub vars: BTreeMap<String, String>,
    pub other_args: BTreeMap<String, String>,

    pub contents: Vec<Object>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FootnoteDefinition {
    pub nid: usize, // one label determines one nid, `nid` used to sort the defintions by the order of first occurrenced reference
    pub label: String, // the actual id of a footnote definition
    pub rids: Vec<usize>, // all rids of footnote references related to current definiton, used to link back to reference.
    pub contents: Vec<Element>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizontalRule {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyword {
    pub key: String,
    pub value: Vec<Object>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffiliatedKeyword {
    pub key: String,
    pub optvalue: Option<String>,
    pub value: Vec<Object>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatexEnvironment {
    pub(crate) text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comment {
    pub text: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FixedWidth {
    pub text: String,
}
