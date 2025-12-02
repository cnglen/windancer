//! AST node definition for element in org-mode
use crate::ast::object::Object;
use std::collections::HashMap;
use std::fmt;

// 文档级结构
#[derive(Clone)]
pub struct Document {
    pub zeroth_section: Option<Section>,
    pub heading_subtrees: Vec<HeadingSubtree>,
    pub footnote_definitions: Vec<FootnoteDefinition>,
    pub k2v: HashMap<String, Vec<Object>>,
}

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r##"Document {{
zeroth_secton: {:#?},
heading_subtrees: {:#?},
footnote_definitions: {:#?}
}}"##,
            self.zeroth_section, self.heading_subtrees, self.footnote_definitions
        )
    }
}

impl Document {
    pub fn new(
        zeroth_section: Option<Section>,
        heading_subtrees: Vec<HeadingSubtree>,
        footnote_definitions: Vec<FootnoteDefinition>,
        k2v: HashMap<String, Vec<Object>>,
    ) -> Self {
        Self {
            zeroth_section,
            heading_subtrees,
            footnote_definitions,
            k2v,
        }
    }

    // // 设置 zeroth_section
    // pub fn with_zeroth_section(mut self, section: Section) -> Self {
    //     self.zeroth_section = Some(section);
    //     self
    // }

    // // 添加顶级标题
    // pub fn add_heading_subtree(mut self, heading_subtree: HeadingSubtree) -> Self {
    //     self.heading_subtrees.push(heading_subtree);
    //     self
    // }
}

#[derive(Clone)]
pub struct HeadingSubtree {
    // heading row info
    pub level: u8,
    pub keyword: Option<String>,
    pub priority: Option<String>,
    pub is_commented: bool,
    pub title: Option<String>,
    pub tags: Vec<String>,

    pub planning: Option<Planning>,
    pub property_drawer: Option<PropertyDrawer>,
    pub section: Option<Section>,
    pub sub_heading_subtrees: Vec<HeadingSubtree>,
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
            self.section,
            self.sub_heading_subtrees
        )
    }
}

#[derive(Clone)]
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
#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct Drawer {
    pub name: String,
    pub contents: Vec<Element>,
    pub affiliated_keywords: Vec<AffiliatedKeyword>,
}

#[derive(Debug, Clone)]
pub struct PropertyDrawer {
    pub contents: Vec<NodeProperty>,
}

#[derive(Debug, Clone)]
pub struct NodeProperty {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Planning {
    pub keyword: String,
    pub timestamp: Object,
}

#[derive(Clone)]
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

#[derive(Clone)]
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
#[derive(Debug, Clone, PartialEq, Eq)]
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
#[derive(Debug, Clone)]
pub struct TableFormula {
    pub data: String,
    // pub target: String,         // 目标单元格/范围
    // pub formula: String,        // 公式内容
    // pub format: Option<String>, // 格式说明
}

#[derive(Debug, Clone)]
pub struct Paragraph {
    pub affiliated_keywords: Vec<AffiliatedKeyword>,
    pub objects: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct List {
    pub list_type: ListType,
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListType {
    Ordered,
    Unordered,
    Descriptive,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub bullet: String,
    pub counter_set: Option<String>,
    pub checkbox: Option<String>,
    pub tag: Option<String>,
    pub contents: Vec<Element>,
}

#[derive(Debug, Clone)]
pub struct CenterBlock {
    pub parameters: Option<String>,
    pub contents: Vec<Element>,
}

#[derive(Debug, Clone)]
pub struct QuoteBlock {
    pub parameters: Option<String>,
    pub contents: Vec<Element>,
}

#[derive(Debug, Clone)]
pub struct SpecialBlock {
    pub name: String,
    pub parameters: Option<String>,
    pub contents: Vec<Element>,
}

// Lesser
#[derive(Debug, Clone)]
pub struct ExampleBlock {
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct CommentBlock {
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct VerseBlock {
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct ExportBlock {
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct SrcBlock {
    pub language: String,
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct FootnoteDefinition {
    pub nid: usize, // one label determines one nid, `nid` used to sort the defintions by the order of first occurrenced reference
    pub label: String, // the actual id of a footnote definition
    pub rids: Vec<usize>, // all rids of footnote references related to current definiton, used to link back to reference.
    pub contents: Vec<Element>,
}

#[derive(Debug, Clone)]
pub struct HorizontalRule {}

#[derive(Debug, Clone)]
pub struct Keyword {
    pub key: String,
    pub value: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct AffiliatedKeyword {
    pub key: String,
    pub optvalue: Option<String>,
    pub value: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct LatexEnvironment {
    pub(crate) text: String,
}

#[derive(Clone, Debug)]
pub struct Comment {
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct FixedWidth {
    pub text: String,
}
