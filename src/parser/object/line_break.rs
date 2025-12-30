//! Line break parser
use crate::parser::object;
use crate::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;

pub(crate) fn line_break_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone
{
    // // todo
    // any()
    //     .map_with(|s, e|)
    //     .rewind()

    // PRE\\SPACE
    just(r##"\\"##)
        .then(object::whitespaces())
        .then_ignore(object::newline().ignored().rewind())
        .try_map_with(|(line_break, maybe_ws): (&str, &str), e| {
            if let Some('\\') = e.state().prev_char {
                let error =
                    Rich::custom(e.span(), format!("PRE is \\ not mathced, NOT line break"));
                Err(error)
            } else {
                let mut children = Vec::with_capacity(2);

                children.push(crate::token!(OSK::BackSlash2, line_break));

                if !maybe_ws.is_empty() {
                    children.push(crate::token!(OSK::Whitespace, maybe_ws));
                    e.state().prev_char = maybe_ws.chars().last();
                } else {
                    e.state().prev_char = line_break.chars().last();
                }

                Ok(crate::node!(OSK::LineBreak, children))
            }
        })
        .boxed()
}
