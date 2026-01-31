//! Line break parser
use chumsky::prelude::*;

use crate::compiler::parser::{MyExtra, NT, OSK, object};

pub(crate) fn line_break_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone
{
    // // todo
    // any()
    //     .map_with(|s, e|)
    //     .rewind()

    // PRE\\SPACE
    object::prev_valid_parser(|c| c.map_or(true, |e| e != '\\'))
        .ignore_then(just(r##"\\"##))
        // just(r##"\\"##)
        .then(object::whitespaces())
        .then_ignore(object::newline().ignored().rewind())
        .map(|(line_break, maybe_ws): (&str, &str)| {
            let mut children = Vec::with_capacity(2);

            children.push(crate::token!(OSK::BackSlash2, line_break));

            if !maybe_ws.is_empty() {
                children.push(crate::token!(OSK::Whitespace, maybe_ws));
            } else {
            }

            crate::node!(OSK::LineBreak, children)
        })
        // .try_map_with(|(line_break, maybe_ws): (&str, &str), e| {
        //     if let Some('\\') = e.state().prev_char {
        //         let error =
        //             Rich::custom(e.span(), format!("PRE is \\ not mathced, NOT line break"));
        //         Err(error)
        //     } else {
        //         let mut children = Vec::with_capacity(2);
        //         children.push(crate::token!(OSK::BackSlash2, line_break));
        //         if !maybe_ws.is_empty() {
        //             children.push(crate::token!(OSK::Whitespace, maybe_ws));
        //             e.state().prev_char = maybe_ws.chars().last();
        //         } else {
        //             e.state().prev_char = line_break.chars().last();
        //         }
        //         Ok(crate::node!(OSK::LineBreak, children))
        //     }
        // })
        .boxed()
}
