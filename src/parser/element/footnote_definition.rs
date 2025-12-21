//! Footnote definition parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, element, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn footnote_definition_parser<'a, C: 'a>(
    element_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
    > + Clone
    + 'a,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
    let affiliated_keywords = element::keyword::affiliated_keyword_parser()
        .repeated()
        .collect::<Vec<_>>();

    let label = any()
        .filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '-'))
        .repeated()
        .at_least(1)
        .to_slice();

    let content_begin = just("[fn:").then(label.clone()).then(just("]"));

    let content_inner = object::line_parser()
        .or(object::blank_line_str_parser())
        .and_is(just("*").ignored().not()) // ends at the next heading
        .and_is(
            object::blank_line_parser()
                .repeated()
                .at_least(2)
                .ignored()
                .not(),
        ) // two consecutive blank lines
        .and_is(content_begin.ignored().not()) // ends at the next footnote definition
        .repeated()
        .to_slice();

    let content = element_parser
        .repeated()
        .collect::<Vec<_>>()
        .nested_in(content_inner);

    affiliated_keywords
        .then(just("[fn:"))
        .then(label)
        .then(just("]"))
        .then(object::whitespaces_g1())
        .then(content.clone())
        .map(|(((((keywords, _lfnc), label), rbracket), ws1), content)| {
            let mut children = Vec::with_capacity(8 + content.len());
            children.extend(keywords);

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
                label,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Whitespace.into(),
                &ws1,
            )));

            children.extend(content);

            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteDefinition.into(),
                children,
            ))
        })
        .boxed()
}

// only used in lookahead
pub(crate) fn simple_footnote_definition_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, &'a str, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>> + Clone
{
    let affiliated_keywords = element::keyword::affiliated_keyword_parser().repeated();

    let label = any()
        .filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '-'))
        .repeated()
        .at_least(1);

    let content_begin = just("[fn:").then(label.clone()).then(just("]"));

    let content_inner = object::line_parser()
        .or(object::blank_line_str_parser())
        .and_is(just("*").ignored().not()) // ends at the next heading
        .and_is(
            object::blank_line_parser()
                .repeated()
                .at_least(2)
                .ignored()
                .not(),
        ) // two consecutive blank lines
        .and_is(content_begin.ignored().not()) // ends at the next footnote definition
        .repeated();

    affiliated_keywords
        .then(just("[fn:"))
        .then(label)
        .then(just("]"))
        .then(object::whitespaces_g1())
        .then(content_inner)
        .to_slice()
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use crate::parser::element;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_footnote_defintion_01() {
        let input = "[fn:1] A short footnote.";
        assert_eq!(
            get_parser_output(
                footnote_definition_parser(element::element_parser::<()>()),
                input
            ),
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
    fn test_footnote_defintion_02_blankline() {
        let input = "[fn:2] This is a longer footnote.

    It even contains a single blank line.
";
        let expected_output = r##"FootnoteDefinition@0..77
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
            get_parser_output(
                footnote_definition_parser(element::element_parser::<()>()),
                input
            ),
            expected_output
        );
    }

    #[test]
    #[should_panic]
    fn test_footnote_defintion_03() {
        let input = "[fn:2] This is a longer footnote.
[fn:3] This is a longer footnote.
";
        get_parser_output(
            footnote_definition_parser(element::element_parser::<()>()),
            input,
        );
    }

    #[test]
    fn test_footnote_defintion_04_keywords() {
        let input = r##"#+caption: affiliated keywords in footnote defintion
[fn:2] This is a longer footnote.
"##;
        let expected_output = r##"FootnoteDefinition@0..87
  AffiliatedKeyword@0..53
    HashPlus@0..2 "#+"
    KeywordKey@2..9
      Text@2..9 "caption"
    Colon@9..10 ":"
    Whitespace@10..11 " "
    KeywordValue@11..52
      Text@11..52 "affiliated keywords i ..."
    Newline@52..53 "\n"
  LeftSquareBracket@53..54 "["
  Text@54..56 "fn"
  Colon@56..57 ":"
  Text@57..58 "2"
  RightSquareBracket@58..59 "]"
  Whitespace@59..60 " "
  Paragraph@60..87
    Text@60..87 "This is a longer foot ..."
"##;
        assert_eq!(
            get_parser_output(
                footnote_definition_parser(element::element_parser::<()>()),
                input
            ),
            expected_output
        );
    }
}
