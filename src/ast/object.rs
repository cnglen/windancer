//! AST node definition for object in org-mode

// 内联元素（Inline-level elements）
#[derive(Debug, Clone)]
pub enum Object {
    Text(String),

    // Markup
    Bold(Vec<Object>),
    Italic(Vec<Object>),
    Underline(Vec<Object>),
    Strikethrough(Vec<Object>),
    Code(Vec<Object>),
    Verbatim(Vec<Object>),

    TableCell(TableCell),

    Link {
        url: String,
        text: Option<String>,
    },

    // todo: inline/anonymous/standard footnote
    FootnoteReference {
        label: Option<String>,
        definition: Option<String>,
    },

    Entity {
        name: String,
    },

    // other
    Whitespace(String),
}

// 表格单元格
#[derive(Debug, Clone)]
pub struct TableCell {
    pub contents: Vec<Object>, // 单元格内容（可以包含内联元素）
                               // pub alignment: CellAlignment, // 对齐方式
                               // pub span: CellSpan,           // 跨行/跨列信息
}
