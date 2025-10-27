//! footnote reference parser
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// Footnote refrence
// - [fn:LABEL]
// - [fn:LABEL:DEFINITION]
// - [fn::DEFINITION]
pub(crate) fn footnote_reference_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    let label = any()
        .filter(|c: &char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        .repeated()
        .at_least(1)
        .collect::<String>();

    // let definition = object_parser(); // make object_parser: recursive
    // FIXME: simplified version
    let definition = any().and_is(just("]").not()).repeated().collect::<String>();

    // [fn:LABEL]
    let t1 = just::<_, _, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>("[fn:")
        .then(label)
        .then(just("]"))
        .map_with(|((_left_fn_c, label), rbracket), e| {
            e.state().prev_char = rbracket.chars().last();
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                "[",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                "fn",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                ":",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &label,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteReference.into(),
                children,
            )))
        });

    // [fn:LABEL:DEFINITION]
    let t2 = just("[fn:")
        .then(label)
        .then(just(":"))
        .then(definition)
        .then(just("]"))
        .map_with(
            |((((_left_fn_c, label), colon), definition), rbracket), e| {
                e.state().prev_char = rbracket.chars().last();
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftSquareBracket.into(),
                    "[",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    "fn",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    colon,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &label,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    ":",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &definition,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightSquareBracket.into(),
                    rbracket,
                )));

                S2::Single(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::FootnoteReference.into(),
                    children,
                )))
            },
        );

    // [fn::DEFINITION]
    let t3 = just("[fn::").then(definition).then(just("]")).map_with(
        |((_left_fn_c_c, definition), rbracket), e| {
            e.state().prev_char = rbracket.chars().last();
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                "[",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                "fn",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon2.into(),
                "::",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &definition,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteReference.into(),
                children,
            )))
        },
    );

    t1.or(t2).or(t3)
}
