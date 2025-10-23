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

    // if definition if found (such as inline or anonymous footnote), a FootnoteDefinition object is auto generated in addition to the FootnoteReference object
    FootnoteReference { // <label + label_rid> identify a unique reference id
        label: String,    // label, the actual id of footnote DEFINITION
        label_rid: usize, // reference id of the same label, started from 1.
        nid: usize,       // auto generated numeric id from label: label <-> nid, started from 1
    },

    Entity {
        name: String,
    },

    LatexFragment {
        display_mode: Option<bool>,
        content: String,
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
