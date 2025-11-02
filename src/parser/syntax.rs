//! Syntax: GreenTree/SyntaxTree(RedTree) definition
use rowan::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u16)]
pub enum OrgSyntaxKind {
    // Structure Node
    Root,
    Document,

    // Element :: Structure Element
    HeadingSubtree, // 从某个 Heading 开始，到下一个同级或更高级别的 Heading 之前的所有内容 (subtree)
    Section,

    // Element :: Greater Element
    Table,
    Drawer,
    CenterBlock,
    QuoteBlock,
    SpecialBlock,
    DynamicBlock,
    List,
    Item,
    PlainList,
    FootnoteDefinition,

    // Element :: Lesser Element
    Paragraph,
    SrcBlock,
    CommentBlock,
    VerseBlock,
    ExampleBlock,
    ExportBlock,
    HorizontalRule,
    LatexEnvironment,
    Keyword,
    BabelCall,
    TableStandardRow,
    TableRuleRow,

    // Object
    Entity,
    LatexFragment,
    Bold,
    Italic,
    Underline,
    Code,
    Verbatim,
    Strikethrough,
    Macro,
    MacroName,
    MacroArgs,
    Subscript,
    Superscript,
    Caret, // ^
    Target,
    Timestamp,

    FootnoteReference,
    FootnoteReferenceLabel, // simplify ast
    FootnoteReferenceDefintion,

    // Token
    EntityName,

    // FIXME:
    Content,

    // Elements
    // 官方文档中, heading, headline 有歧义 (1) 含section (2) 仅指第一行, subtree指整体。用HeadingSubtree和HeadingRow区分。
    HeadingRow, // 标题行，只是标题本身，指一行
    HeadingRowStars,
    HeadingRowKeywordTodo, // optional
    HeadingRowKeywordDone, // optional
    HeadingRowPriority,    // optional
    HeadingRowComment,     // optional
    HeadingRowTitle,       // optional
    HeadingRowTags,        // optional: Node
    HeadingRowTag,         // Token

    TableCell, // Node
    HashPlus,
    DrawerBegin,
    DrawerContent,
    DrawerEnd,
    PropertyDrawer,

    BlockBegin,
    BlockContent,
    BlockEnd,
    SrcBlockLanguage,

    //
    SectionUnknown,

    ListItem,
    ListItemIndent,
    ListItemBullet,
    ListItemCounter,
    ListItemCheckbox,
    ListItemTag,
    ListItemContent,

    DiarySexp,

    NodeProperty,
    FixedWhidth,
    Comment,
    Planning,
    Clock,

    LatexEnvironmentBegin,
    LatexEnvironmentEnd,

    //

    // 行内节点
    Text,

    Link,
    LinkPath,
    LinkDescription,

    AngleLink,
    PlainLink,

    LineBreak,

    // 令牌类型
    HeadingMarker, // *, **, *** 等
    TextContent,
    Whitespace,
    Newline,
    Asterisk,   // *
    Star,       // *
    Slash,      // /
    Underscore, // _
    Plus,       // +
    Equals,     // =
    Tilde,      // ~
    Colon,      // Token :
    Colon2,     // Token ::
    Dollar,     // $
    Dollar2,    // $$
    Pipe,       // Token |
    Dash,       // Token -
    At,         // at
    BackSlash,  // \
    BackSlash2, // \\

    LinkStart,
    LinkEnd,

    BlankLine, // Token
    LeftSquareBracket,
    RightSquareBracket,
    LeftCurlyBracket,
    RightCurlyBracket,
    LeftAngleBracket,
    RightAngleBracket,
    LeftAngleBracket2,
    RightAngleBracket2,

    LeftCurlyBracket3,
    RightCurlyBracket3,

    LeftRoundBracket,
    RightRoundBracket,
    Hash,

    Spaces,

    // 错误节点
    Error,
}

// 为方便使用，实现 From<OrgSyntaxKind> for rowan::SyntaxKind
impl From<OrgSyntaxKind> for rowan::SyntaxKind {
    fn from(kind: OrgSyntaxKind) -> Self {
        Self(kind as u16)
    }
}

pub type SyntaxNode = rowan::SyntaxNode<OrgLanguage>;
pub(crate) type SyntaxToken = rowan::SyntaxToken<OrgLanguage>;

// NodeOrToken<SyntaxNode<OrgLanguage>, SyntaxToken<OrgLanguage>>
pub type SyntaxElement = rowan::SyntaxElement<OrgLanguage>;

pub type SyntaxNodeChildren = rowan::SyntaxNodeChildren<OrgLanguage>;
pub type SyntaxElementChildren = rowan::SyntaxElementChildren<OrgLanguage>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OrgLanguage {}
impl Language for OrgLanguage {
    type Kind = OrgSyntaxKind;

    fn kind_from_raw(raw: rowan::SyntaxKind) -> Self::Kind {
        unsafe { std::mem::transmute::<u16, OrgSyntaxKind>(raw.0) }
    }
    fn kind_to_raw(kind: Self::Kind) -> rowan::SyntaxKind {
        rowan::SyntaxKind(kind as u16)
    }
}
