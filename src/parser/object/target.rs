//! target parser
use crate::parser::ParserState;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// target parser: <<TARGET>>
pub(crate) fn target_parser<'a, C: 'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
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

    just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>("<<")
        .then(target)
        .then(just(">>"))
        .map_with(|((lbracket2, target), rbracket2), e| {
            e.state().prev_char = rbracket2.chars().last();

            let mut children = Vec::with_capacity(3);
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

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Target.into(),
                children,
            ))
        })
        .boxed()
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parser_output;
    use pretty_assertions::assert_eq;

    use super::target_parser;

    #[test]
    fn test_target_01() {
        assert_eq!(
            get_parser_output(target_parser::<()>(), "<<target>>"),
            r##"Target@0..10
  LeftAngleBracket2@0..2 "<<"
  Text@2..8 "target"
  RightAngleBracket2@8..10 ">>"
"##
        );
    }

    #[test]
    #[should_panic]
    fn test_target_02() {
        get_parser_output(target_parser::<()>(), "<<tar\nget>>");
    }

    #[test]
    #[should_panic]
    fn test_target_03() {
        get_parser_output(target_parser::<()>(), "<< target>>");
    }

    #[test]
    #[should_panic]
    fn test_target_04() {
        get_parser_output(target_parser::<()>(), "<<target >>");
    }

    #[test]
    fn test_target_05() {
        assert_eq!(
            get_parser_output(target_parser::<()>(), "<<t>>"),
            r##"Target@0..5
  LeftAngleBracket2@0..2 "<<"
  Text@2..3 "t"
  RightAngleBracket2@3..5 ">>"
"##,
            r"TARGET with one char"
        );
    }
}
