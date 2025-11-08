//! Line break parser

use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::object::whitespaces;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn line_break_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    // // todo
    // any()
    //     .map_with(|s, e|)
    //     .rewind()

    // PRE\\SPACE
    just(r##"\\"##)
        .then(whitespaces())
        .then_ignore(
            one_of("\r")
                .repeated()
                .at_most(1)
                .collect::<String>()
                .then(just("\n"))
                .rewind(),
        )
        .try_map_with(|(line_break, maybe_ws), e| {
            if let Some('\\') = e.state().prev_char {
                let error =
                    Rich::custom(e.span(), format!("PRE is \\ not mathced, NOT line break"));
                Err(error)
            } else {
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::BackSlash2.into(),
                    line_break,
                )));

                if maybe_ws.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &maybe_ws,
                    )));
                    e.state().prev_char = maybe_ws.chars().last();
                } else {
                    e.state().prev_char = line_break.chars().last();
                }

                Ok(S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(
                    GreenNode::new(OrgSyntaxKind::LineBreak.into(), children),
                )))
            }
        })
}
