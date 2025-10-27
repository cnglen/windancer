//! target parser
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// target parser: <<TARGET>>
pub(crate) fn target_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
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
    let target = target_g2char.or(target_onechar);

    just::<_, _, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>("<<")
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
