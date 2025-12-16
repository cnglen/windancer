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
    object::whitespaces_v2()
        .then(just("-").repeated().at_least(5).collect::<Vec<&str>>()) //todo: collect as String failed
        .then(object::whitespaces_v2())
        .then(object::newline_or_ending())
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(|((((ws1, dashes), ws2), nl), blanklines)| {
            let mut children = vec![];
            if ws1.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }

            let mut _dashes = String::new();
            for s in dashes.into_iter() {
                _dashes.push_str(s);
            }
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &_dashes,
            )));

            if ws2.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws2,
                )));
            }

            match nl {
                Some(newline) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Newline.into(),
                        &newline,
                    )));
                }
                None => {}
            }
            for blankline in blanklines {
                children.push(NodeOrToken::Token(blankline));
            }

            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::HorizontalRule.into(),
                children,
            ))
        })
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
