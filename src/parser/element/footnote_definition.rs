//! Footnote definition parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use chumsky::prelude::*;
use chumsky::{inspector::RollbackState, text::Char};
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn footnote_definition_parser<'a>(
    element_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let label = any()
        .filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '-'))
        .repeated()
        .at_least(1)
        .collect::<String>();

    let content_begin = just("[fn:").then(label.clone()).then(just("]"));

    let content_inner = object::line_parser()
        .or(object::blank_line_str_parser())
        .and_is(just("*").not()) // ends at the next heading
        .and_is(object::blank_line_parser().repeated().at_least(2).not()) // two consecutive blank lines
        .and_is(content_begin.not()) // ends at the next footnote definition
        .repeated()
        .to_slice();

    let content = element_parser
        .repeated()
        .collect::<Vec<_>>()
        .nested_in(content_inner);

    just("[fn:")
        .then(label)
        .then(just("]"))
        .then(object::whitespaces_g1())
        .then(content.clone())
        .map(|((((_lfnc, label), rbracket), ws1), content)| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                "[",
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                "fn",
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                ":",
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &label,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Whitespace.into(),
                &ws1,
            )));

            for e in content {
                children.push(e);
            }

            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteDefinition.into(),
                children,
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use crate::parser::element;
    use crate::parser::{ParserState, SyntaxNode};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_footnote_01() {
        let input = "[fn:1] A short footnote.";
        assert_eq!(
            get_parser_output(footnote_definition_parser(element::element_parser()), input),
            r##"FootnoteDefinition@0..24
  LeftSquareBracket@0..1 "["
  Text@1..3 "fn"
  Colon@3..4 ":"
  Text@4..5 "1"
  RightSquareBracket@5..6 "]"
  Whitespace@6..7 " "
  Paragraph@7..24
    Text@7..24 "A short footnote."
"##
        );
    }

    #[test]
    fn test_footnote_02_blankline() {
        let input = "[fn:2] This is a longer footnote.

    It even contains a single blank line.
";
        let expected_outoput = r##"FootnoteDefinition@0..77
  LeftSquareBracket@0..1 "["
  Text@1..3 "fn"
  Colon@3..4 ":"
  Text@4..5 "2"
  RightSquareBracket@5..6 "]"
  Whitespace@6..7 " "
  Paragraph@7..35
    Text@7..34 "This is a longer foot ..."
    BlankLine@34..35 "\n"
  Paragraph@35..77
    Text@35..77 "    It even contains  ..."
"##;
        assert_eq!(
            get_parser_output(footnote_definition_parser(element::element_parser()), input),
            expected_outoput
        );
    }

    #[test]
    #[should_panic]
    fn test_footnote_03() {
        let input = "[fn:2] This is a longer footnote.
[fn:3] This is a longer footnote.
";
        get_parser_output(footnote_definition_parser(element::element_parser()), input);
    }
}
