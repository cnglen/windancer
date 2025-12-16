//! Comment parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn comment_parser<'a, C: 'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
    let comment_line1 = object::whitespaces()
        .then(just("#"))
        .then(object::whitespaces_g1())
        .then(none_of("\n").repeated().collect::<String>())
        .then(object::newline_or_ending())
        .map(|((((ws1, hash), ws2), content), maybe_nl)| {
            let mut children = vec![];

            if ws1.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Hash.into(),
                hash,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Whitespace.into(),
                &ws2,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &content,
            )));

            match maybe_nl {
                Some(nl) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Newline.into(),
                        &nl,
                    )));
                }
                None => {}
            }

            children
        });

    let comment_line2 = object::whitespaces()
        .then(just("#"))
        .then(object::newline_or_ending())
        .map(|((ws, hash), maybe_nl)| {
            let mut children = vec![];

            if ws.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Hash.into(),
                hash,
            )));

            match maybe_nl {
                Some(nl) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Newline.into(),
                        &nl,
                    )));
                }
                None => {}
            }

            children
        });

    comment_line1
        .or(comment_line2)
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(|(vn, blanklines)| {
            let mut children = vec![];
            for e in vn {
                for ee in e {
                    children.push(ee);
                }
            }
            for blankline in blanklines {
                children.push(NodeOrToken::Token(blankline));
            }

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Comment.into(), children))
        })
}
