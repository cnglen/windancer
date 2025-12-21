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
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    ws1,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Hash.into(),
                hash,
            )));

            if let Some((ws2, content)) = maybe_ws2_content {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    ws2,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    content,
                )));
            }

            if let Some(nl) = maybe_nl {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    nl,
                )));
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

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Comment.into(), children))
        })
        .boxed()
}
