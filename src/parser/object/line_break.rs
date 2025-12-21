//! Line break parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn line_break_parser<'a, C: 'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
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

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::BackSlash2.into(),
                    line_break,
                )));

                if !maybe_ws.is_empty() {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        maybe_ws,
                    )));
                    e.state().prev_char = maybe_ws.chars().last();
                } else {
                    e.state().prev_char = line_break.chars().last();
                }

                Ok(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::LineBreak.into(),
                    children,
                )))
            }
        })
        .boxed()
}
