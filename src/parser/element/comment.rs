//! Comment parser
use crate::parser::object;
use crate::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;

pub(crate) fn comment_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let comment_line = object::whitespaces()
        .then(just("#"))
        .then(
            object::whitespaces_g1()
                .then(none_of(object::CRLF).repeated().to_slice())
                .or_not(),
        )
        .then(object::newline_or_ending())
        .map(|(((ws1, hash), maybe_ws2_content), maybe_nl)| {
            let mut children = Vec::with_capacity(5);

            if !ws1.is_empty() {
                children.push(crate::token!(OSK::Whitespace, ws1));
            }

            children.push(crate::token!(OSK::Hash, hash));

            if let Some((ws2, content)) = maybe_ws2_content {
                children.push(crate::token!(OSK::Whitespace, ws2));
                children.push(crate::token!(OSK::Text, content));
            }

            if let Some(nl) = maybe_nl {
                children.push(crate::token!(OSK::Newline, nl));
            }
            children
        });

    comment_line
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(|(vn, blanklines)| {
            let mut children =
                Vec::with_capacity(vn.iter().map(|e| e.len()).sum::<usize>() + blanklines.len());
            children.extend(vn.into_iter().flatten());
            children.extend(blanklines);

            crate::node!(OSK::Comment, children)
        })
        .boxed()
}
