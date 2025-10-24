//! AST node definition for element in org-mode
use crate::ast::object::{Object, TableCell};
use crate::parser::syntax::{OrgLanguage, OrgSyntaxKind, SyntaxElement, SyntaxNode};
use std::collections::HashMap;
use std::fmt;

// 文档级结构
#[derive(Clone)]
pub struct Document {
    pub(crate) syntax: SyntaxNode,
    pub zeroth_section: Option<Section>,
    pub heading_subtrees: Vec<HeadingSubtree>,
    pub footnote_definitions: Vec<FootnoteDefinition>,
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
        syntax: SyntaxNode,
        zeroth_section: Option<Section>,
        heading_subtrees: Vec<HeadingSubtree>,
        footnote_definitions: Vec<FootnoteDefinition>,
    ) -> Self {
        Self {
            syntax,
            zeroth_section,
            heading_subtrees,
            footnote_definitions,
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
    pub(crate) syntax: SyntaxNode,

    // heading row info
    pub level: u8,
    pub keyword: Option<String>,
    pub priority: Option<String>,
    pub is_commented: bool,
    pub title: Option<String>,
    pub tags: Vec<String>,

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

    section: {:#?},
    sub_heading_subtrees: {:#?}
}}"##,
            self.level,
            self.keyword,
            self.priority,
            self.is_commented,
            self.title,
            self.tags,
            self.section,
            self.sub_heading_subtrees
        )
    }
}

#[derive(Clone)]
pub struct Section {
    pub(crate) syntax: SyntaxNode,

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
    // PropertyDrawer(PropertyDrawer),
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
    TableRow(TableRow),
    // BabelCall(BabelCall),
}

#[derive(Debug, Clone)]
pub struct Drawer {
    pub(crate) syntax: SyntaxNode,
    pub name: String,
    pub contents: Vec<Element>,
}

#[derive(Clone)]
pub struct Table {
    pub(crate) syntax: SyntaxNode,
    pub name: Option<String>,    // 表格名称 (#+NAME:)
    pub caption: Option<String>, // 表格标题 (#+CAPTION:)
    // pub attributes: TableAttributes,    // 表格属性
    pub header: Option<TableRow>,    // 表头行（可选）
    pub separator: Option<TableRow>, // 分隔线行（可选）
    pub rows: Vec<TableRow>,         // 数据行
                                     // pub formulas: Vec<TableFormula>,    // 表格公式
}

impl fmt::Debug for Table {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self.rows)
    }
}

#[derive(Clone)]
pub struct TableRow {
    pub(crate) syntax: SyntaxNode,
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
    pub target: String,         // 目标单元格/范围
    pub formula: String,        // 公式内容
    pub format: Option<String>, // 格式说明
}

#[derive(Clone)]
pub struct Paragraph {
    pub(crate) syntax: SyntaxNode,
    pub objects: Vec<Object>,
}

impl fmt::Debug for Paragraph {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            r##"Section {{
    objects: {:#?}
}}"##,
            self.objects
        )
    }
}

#[derive(Debug, Clone)]
pub struct List {
    pub(crate) syntax: SyntaxNode,
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
    pub(crate) syntax: SyntaxNode,
    pub bullet: String,
    pub counter_set: Option<String>,
    pub checkbox: Option<String>,
    pub tag: Option<String>,
    pub contents: Vec<Element>,
}

#[derive(Debug, Clone)]
pub struct CenterBlock {
    pub(crate) syntax: SyntaxNode,
    pub parameters: Option<String>,
    pub contents: Vec<Element>,
}

#[derive(Debug, Clone)]
pub struct QuoteBlock {
    pub(crate) syntax: SyntaxNode,
    pub parameters: Option<String>,
    pub contents: Vec<Element>,
}

#[derive(Debug, Clone)]
pub struct SpecialBlock {
    pub(crate) syntax: SyntaxNode,
    pub name: String,
    pub parameters: Option<String>,
    pub contents: Vec<Element>,
}

// Lesser
#[derive(Debug, Clone)]
pub struct ExampleBlock {
    pub(crate) syntax: SyntaxNode,
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct CommentBlock {
    pub(crate) syntax: SyntaxNode,
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct VerseBlock {
    pub(crate) syntax: SyntaxNode,
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct ExportBlock {
    pub(crate) syntax: SyntaxNode,
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct SrcBlock {
    pub(crate) syntax: SyntaxNode,
    pub data: Option<String>,
    pub contents: Vec<Object>,
}

#[derive(Debug, Clone)]
pub struct FootnoteDefinition {
    pub nid: usize, // one label determines one nid, `nid` used to sort the defintions by the order of first occurrenced reference
    pub label: String, // the actual id of a footnote definition
    pub rids: Vec<usize>, // all rids of footnote references related to current definiton, used to link back to reference.
    pub contents: Vec<Element>,
    pub(crate) syntax: SyntaxNode,
}

#[derive(Debug, Clone)]
pub struct HorizontalRule {
    pub(crate) syntax: SyntaxNode,
}

#[derive(Debug, Clone)]
pub struct Keyword {
    pub(crate) syntax: SyntaxNode,
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct LatexEnvironment {
    pub(crate) syntax: SyntaxNode,
}

impl LatexEnvironment {
    pub fn syntax(&self) -> &SyntaxNode {
        &self.syntax
    }
}
