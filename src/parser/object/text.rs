//! text parser
/// -> other_object parsers (lookahead)
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

// plain text Parser
pub(crate) fn plain_text_parser<'a>(
    non_plain_text_parsers: impl Parser<
        'a,
        &'a str,
        S2,
        extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    any::<_, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>()
        .and_is(non_plain_text_parsers.not())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map_with(|s, e| {
            if let Some(c) = s.chars().last() {
                e.state().prev_char = Some(c);
            }

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Token(
                GreenToken::new(OrgSyntaxKind::Text.into(), &s),
            ))
        })
}
