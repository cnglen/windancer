//! text parser
/// -> other_object parsers (lookahead)
use crate::parser::ParserState;
use crate::parser::{MyExtra, NT, OSK};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::GreenToken;

// plain text Parser
pub(crate) fn plain_text_parser<'a, C: 'a>(
    non_plain_text_parsers: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    any()
        .and_is(non_plain_text_parsers.ignored().not())
        // we MUST update state here: if negation lookahead successes，update state to let the object_parser work
        // input: fox_bar
        //   - f: negation lookahead OK, update state -> f
        //   - o:                                     -> o
        //   - x:                                     -> x
        //   - _: negation lookahead BAD(subscript_parser OK, update state -> r; then rollback -> x), then Text(fox) OK, update state -> x
        // we must use RollbackState: if negation lookahead failed, rollback
        .map_with(|c, e| {
            e.state().prev_char = Some(c);
            c
        })
        .repeated()
        .at_least(1)
        // .to_slice()
        .collect::<String>() // to_slice? test failed
        .map_with(|s, e| {
            if let Some(c) = s.chars().last() {
                (e.state() as &mut RollbackState<ParserState>).prev_char = Some(c);
            }

            NT::Token(GreenToken::new(OSK::Text.into(), &s))
        })
        .boxed()
}
