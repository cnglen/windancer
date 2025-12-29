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
        .then(choice((
            object::newline()
                .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
                .map(|(newline, blanklines)| {
                    let mut children = Vec::with_capacity(1 + blanklines.len());
                    children.push(crate::token!(OrgSyntaxKind::Newline, newline));
                    children.extend(blanklines);

                    children
                }),
            end().to(vec![]),
        )))
        .map(|(((ws1, dashes), ws2), others)| {
            let mut children = Vec::with_capacity(3 + others.len());
            if !ws1.is_empty() {
                children.push(crate::token!(OrgSyntaxKind::Whitespace, ws1));
            }
            children.push(crate::token!(OrgSyntaxKind::Text, dashes));
            if !ws2.is_empty() {
                children.push(crate::token!(OrgSyntaxKind::Whitespace, ws2));
            }
            children.extend(others);

            crate::node!(OrgSyntaxKind::HorizontalRule, children)
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
