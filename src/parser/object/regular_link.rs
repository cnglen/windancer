//! regular link parser
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::input::MapExtra;
use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// regular link parser
pub(crate) fn regular_link_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    let pathreg = just("[")
        .then(none_of("]").repeated().collect::<String>())
        .then(just("]"))
        .map(|((lbracket, path), rbracket)| {
            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                lbracket,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &path,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));
            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::LinkPath.into(),
                children,
            ))
        });

    just("[")
        .then(pathreg)
        .then(
            just("[")
                .then(none_of("]").repeated().collect::<String>())
                .then(just("]"))
                .or_not()
                .map(|description| match description {
                    None => None,

                    Some(((lbracket, content), rbracket)) => {
                        let mut children = vec![];
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::LeftSquareBracket.into(),
                            lbracket,
                        )));

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Text.into(),
                            &content,
                        )));

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::RightSquareBracket.into(),
                            rbracket,
                        )));

                        Some(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                            OrgSyntaxKind::LinkDescription.into(),
                            children,
                        )))
                    }
                }),
        )
        .then(just("]"))
        .map_with(
            |(((lbracket, path), maybe_desc), rbracket),
             e: &mut MapExtra<
                '_,
                '_,
                &str,
                extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>,
            >| {
                // Q: ^ why type annotation needed?  for e:
                e.state().prev_char = rbracket.chars().last();

                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftSquareBracket.into(),
                    lbracket,
                )));

                children.push(path);

                match maybe_desc {
                    None => {}
                    Some(desc) => {
                        children.push(desc);
                    }
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightSquareBracket.into(),
                    rbracket,
                )));

                let link = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::Link.into(),
                    children,
                ));

                S2::Single(link)
            },
        )
}
