//! text parser
/// -> other_object parsers (lookahead)
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

// plain text Parser
pub(crate) fn plain_text_parser<'a>(
    non_plain_text_parsers: impl Parser<
        'a,
        &'a str,
        S2,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{

    // fox_bar
    // f lookahead negotion OK, update state -> f
    // o                                     -> o
    // x -> x
    // _ lookahead negotion bad, subscript_parser -> r => Text(fox), update to x
    // 
    // fixme: id lookahead negation failed, how to resume state back?
    any::<_, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>()
        .and_is(non_plain_text_parsers.not())
        .map_with(|c, e|{       // update state!! this is important: if lookahead negation successes，update state
            e.state().prev_char= Some(c);
            // println!("plain_texx_paser: map_with update -> {:?}", e.state().prev_char);
            c})
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map_with(|s, e| {
            println!("plain_text_parser: s={s:?}, prev_char={:?}", e.state().prev_char);
            if let Some(c) = s.chars().last() {
                e.state().prev_char = Some(c);
            }

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Token(
                GreenToken::new(OrgSyntaxKind::Text.into(), &s),
            ))
        })
}
