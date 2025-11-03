//! target parser
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// target parser: <<TARGET>>
pub(crate) fn target_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let target_onechar = none_of("<>\n \t").map(|c| format!("{c}"));
    let target_g2char = none_of("<>\n \t")
        .then(none_of("<>\n").repeated().at_least(1).collect::<String>())
        .try_map_with(|(a, b), e| {
            if b.chars().last().expect("at least 1").is_whitespace() {
                Err(Rich::custom(
                    e.span(),
                    format!("the last char of '{}' can't be whitespace", b),
                ))
            } else {
                Ok(format!("{a}{b}"))
            }
        });

    let target = choice((target_g2char, target_onechar)); // target_g2char > target_onechar

    just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>("<<")
        .then(target)
        .then(just(">>"))
        .map_with(|((lbracket2, target), rbracket2), e| {
            e.state().prev_char = rbracket2.chars().last();

            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftAngleBracket2.into(),
                lbracket2,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &target,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightAngleBracket2.into(),
                rbracket2,
            )));

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Target.into(),
                children,
            )))
        })
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parser_output;
    use pretty_assertions::assert_eq;

    use super::target_parser;

    #[test]
    fn test_target_01() {
        assert_eq!(
            get_parser_output(target_parser(), "<<target>>"),
            r##"Target@0..10
  LeftAngleBracket2@0..2 "<<"
  Text@2..8 "target"
  RightAngleBracket2@8..10 ">>"
"##
        );
    }

    #[test]
    fn test_target_02() {
        assert_eq!(
            get_parser_output(target_parser(), "<<tar\nget>>"),
            r##"errors:
found ''\n'' at 5..6 expected something else, or ''>''"##,
            r"TARGET is a string containing any characters but `<>\n`"
        );
    }

    #[test]
    fn test_target_03() {
        assert_eq!(
            get_parser_output(target_parser(), "<< target>>"),
            r##"errors:
found '' '' at 2..3 expected something else"##,
            r"TARGET It cannot start or end with a whitespace character."
        );
    }

    #[test]
    fn test_target_04() {
        assert_eq!(
            get_parser_output(target_parser(), "<<target >>"),
            r##"errors:
found ''a'' at 3..4 expected ''>''"##,
            r"TARGET It cannot start or end with a whitespace character."
        );
    }

    #[test]
    fn test_target_05() {
        assert_eq!(
            get_parser_output(target_parser(), "<<t>>"),
            r##"Target@0..5
  LeftAngleBracket2@0..2 "<<"
  Text@2..3 "t"
  RightAngleBracket2@3..5 ">>"
"##,
            r"TARGET with one char"
        );
    }
}
