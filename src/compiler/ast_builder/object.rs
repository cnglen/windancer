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

    RadioLink(Vec<Object>),
    RadioTarget(Vec<Object>),

    // external link: has protocol
    // internal link:
    //   #my-custom-id ->
    //   *some section ->
    //   my_tartet -> <<my_target>> / keyword #+NAME: my_target
    // including regular/plain/angle link, without radio link
    // radio link?
    GeneralLink {
        protocol: String, // protocol or type, http/file/#
        path: String,     //
        description: Vec<Object>,
        is_image: bool,
    },

    Superscript(Vec<Object>),
    Subscript(Vec<Object>),

    Target(String),

    Timestamp(String),

    // if definition if found (such as inline or anonymous footnote), a FootnoteDefinition object is auto generated in addition to the FootnoteReference object
    FootnoteReference {
        // <label + label_rid> identify a unique reference id
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

    LineBreak,

    Macro {
        name: String,
        arguments: Vec<String>,
    },

    InlineSourceBlock {
        lang: String,
        headers: Option<String>,
        body: String,
    },

    InlineBabelCall {
        name: String,
        header1: Option<String>,
        arguments: String,
        header2: Option<String>,
    },

    StatisticsCookie(String),

    CitationReference(CitationReference),

    Citation {
        global_prefix: Vec<Object>,
        citestyle: Option<String>,
        references: Vec<CitationReference>,
        global_suffix: Vec<Object>,
    },

    ExportSnippet {
        backend: String,
        value: String,
    },

    // other
    Whitespace(String),
}

#[derive(Debug, Clone)]
pub struct TableCell {
    pub contents: Vec<Object>,
    // pub alignment: CellAlignment, // 对齐方式
    // pub span: CellSpan,           // 跨行/跨列信息
    pub cell_type: TableCellType,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableCellType {
    Header,
    Data,
}

#[derive(Debug, Clone)]
pub struct CitationReference {
    pub key_prefix: Vec<Object>,
    pub key: String,
    pub key_suffix: Vec<Object>,
}
