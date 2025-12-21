//! Horizontal rule parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn horizontal_rule_parser<'a, C: 'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
    object::whitespaces()
        .then(just("-").repeated().at_least(5).to_slice())
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(|((((ws1, dashes), ws2), nl), blanklines)| {
            let mut children = Vec::with_capacity(3 + blanklines.len());
            if !ws1.is_empty() {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    ws1,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                dashes,
            )));

            if !ws2.is_empty() {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    ws2,
                )));
            }

            if let Some(newline) = nl {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    newline,
                )));
            }

            children.extend(blanklines);

            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::HorizontalRule.into(),
                children,
            ))
        })
        .boxed()
}
#[cfg(test)]
mod tests {
    use crate::parser::common::get_parser_output;

    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_horizontal_rule_01() {
        assert_eq!(
            get_parser_output(horizontal_rule_parser::<()>(), r"-----"),
            r#"HorizontalRule@0..5
  Text@0..5 "-----"
"#
        );
    }

    #[test]
    fn test_horizontal_rule_02() {
        assert_eq!(
            get_parser_output(horizontal_rule_parser::<()>(), r"---------"),
            r#"HorizontalRule@0..9
  Text@0..9 "---------"
"#
        );
    }

    #[test]
    #[should_panic]
    fn test_horizontal_rule_03() {
        get_parser_output(horizontal_rule_parser::<()>(), r"----");
    }
}
