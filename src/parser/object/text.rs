//! text parser
/// -> other_object parsers (lookahead)
use crate::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;

// plain text Parser
pub(crate) fn plain_text_parser<'a, C: 'a>(
    non_plain_text_parsers: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    any()
        .and_is(non_plain_text_parsers.ignored().not())
        .repeated()
        .at_least(1)
        .to_slice()
        .map(|s| crate::token!(OSK::Text, s))
        .boxed()
}
