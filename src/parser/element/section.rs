//! Section parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserResult, ParserState, element, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;

use crate::parser::element::{
    block, comment, drawer, footnote_definition, horizontal_rule, keyword, latex_environment,
    paragraph, table,
};

use crate::parser::element::paragraph::simple_heading_row_parser;

/// Section解析器，返回包含`GreenNode`的ParserResult
///
/// 实现要点:
/// - 结尾满足下面条件之一:
///   - \n + HeadingRow: 避免把`This is a * faked_title`部分识别为HeadingRow
///   - end()
///     - \n + end()
///     - end()
/// - 开头不能以`* Text`开头, 否则部分标题会被识别为Section

// block_parser
// blank_line
// other_parser
// S2? 是否合适?
pub(crate) fn section_parser<'a>(
    element_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone
    + 'a,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    Parser::boxed(
        element_parser
            .and_is(simple_heading_row_parser().not()) // Section不能以<* title>开头，避免HeadingSurbtree被识别为Section
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>()
            .labelled("section parse")
            .map_with(|children, e| {
                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Section.into(), children))
            }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use crate::parser::element::element_in_section_parser;
    use pretty_assertions::assert_eq;

    #[test]
    #[should_panic]
    fn test_section_01() {
        let input = "section content
* heading
";
        let parser = section_parser(element_in_section_parser());
        get_parser_output(parser, input);
    }

    #[test]
    fn test_section_02_fakedtitle() {
        let input = "0123456789 * faked_title";
        let parser = section_parser(element_in_section_parser());
        assert_eq!(
            get_parser_output(parser, input),
            r##"Section@0..24
  Paragraph@0..24
    Text@0..18 "0123456789 * faked"
    Subscript@18..24
      Caret@18..19 "_"
      Text@19..24 "title"
"##
        );
    }

    #[test]
    #[should_panic]
    fn test_section_03_vs_heading_subtree() {
        let input = "* title\n asf\n";
        let parser = section_parser(element_in_section_parser());
        get_parser_output(parser, input);
    }

    #[test]
    fn test_section_04_with_end() {
        let input = "0123456789";
        let parser = section_parser(element_in_section_parser());
        assert_eq!(
            get_parser_output(parser, input),
            r##"Section@0..10
  Paragraph@0..10
    Text@0..10 "0123456789"
"##
        );
    }

    #[test]
    fn test_section_05_with_newline_end() {
        let input = "0123456789\n";
        let parser = section_parser(element_in_section_parser());
        assert_eq!(
            get_parser_output(parser, input),
            r##"Section@0..11
  Paragraph@0..11
    Text@0..11 "0123456789\n"
"##
        );
    }

    #[test]
    fn test_section_06_with_newline_end() {
        let input = "0123456789\nfoo\nbar\nhello\nnice\nto meet you\n\n";
        let parser = section_parser(element_in_section_parser());
        assert_eq!(
            get_parser_output(parser, input),
            r##"Section@0..43
  Paragraph@0..43
    Text@0..42 "0123456789\nfoo\nbar\nhe ..."
    BlankLine@42..43 "\n"
"##
        );
    }

    #[test]
    fn test_section_07_with_newline_end() {
        let input = "SCHEDULED: <1999-03-31 Wed>
"; // planning is not allowed to be in section
        let parser = section_parser(element_in_section_parser());
        assert_eq!(
            get_parser_output(parser, input),
            r##"Section@0..28
  Paragraph@0..28
    Text@0..11 "SCHEDULED: "
    Timestamp@11..27
      Text@11..27 "<1999-03-31 Wed>"
    Text@27..28 "\n"
"##
        );
    }
}
