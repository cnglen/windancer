use crate::compiler::parser::syntax::OrgSyntaxKind;
use std::fmt;
use thiserror::Error;

/// AST转换过程中可能出现的错误类型
#[derive(Debug, Error)]
pub enum AstError {
    /// 未知的节点类型
    #[error("未知的节点类型: {kind:?}")]
    UnknownNodeType {
        kind: OrgSyntaxKind,
        position: Option<TextRange>,
    },

    /// 缺少必需的子节点
    #[error("节点 {parent_kind:?} 缺少必需的子节点 {child_kind:?}")]
    MissingRequiredChild {
        parent_kind: OrgSyntaxKind,
        child_kind: OrgSyntaxKind,
        position: Option<TextRange>,
    },

    /// 文本内容格式错误
    #[error("文本内容格式错误: {message} (在 {text:?} 中)")]
    TextFormatError {
        message: String,
        text: String,
        position: Option<TextRange>,
    },

    /// 无效的标题级别
    #[error("无效的标题级别: {level} (应为 1-6)")]
    InvalidHeadingLevel {
        level: u8,
        position: Option<TextRange>,
    },

    /// 链接格式错误
    #[error("链接格式错误: {url}")]
    InvalidLinkFormat {
        url: String,
        position: Option<TextRange>,
    },

    /// 嵌套结构错误
    #[error("嵌套结构错误: {message}")]
    NestingError {
        message: String,
        position: Option<TextRange>,
    },

    /// 多个错误集合
    #[error("发现多个错误:\n{}", format_errors(.0))]
    MultipleErrors(Vec<AstError>),

    /// 内部错误（通常用于包装其他错误）
    #[error("内部转换错误: {source}")]
    InternalError {
        #[from]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// 文本范围位置信息
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub start: usize,
    pub end: usize,
}

impl TextRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

impl fmt::Display for TextRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

// 为MultipleErrors提供格式化辅助函数
fn format_errors(errors: &[AstError]) -> String {
    errors
        .iter()
        .map(|err| format!("  - {}", err))
        .collect::<Vec<_>>()
        .join("\n")
}
