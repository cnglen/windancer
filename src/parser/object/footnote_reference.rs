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
pub(crate) fn footnote_reference_parser<'a>(
    object_parser: impl Parser<
        'a,
        &'a str,
        S2,
        extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    let label = any()
        .filter(|c: &char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        .repeated()
        .at_least(1)
        .collect::<String>();

    // defintion mus in oneline
    let var =
        none_of::<&str, &str, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>("[]\r\n")
            .repeated()
            .at_least(1)
            .to_slice();
    let mut single_expression = Recursive::declare(); // foo / (foo) / (((foo)))
    single_expression.define(
        var.or(just("[")
            .then(single_expression.clone().repeated())
            .then(just("]"))
            .to_slice()),
    );
    let standard_objects_parser = object_parser.repeated().at_least(1).collect::<Vec<S2>>();
    let definition =
        standard_objects_parser.nested_in(single_expression.clone().repeated().to_slice());

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
        .map(|s| {
            println!("s0={s:?};");
            s
        })
        .then(just(":"))
        .map(|s| {
            println!("s1={s:?};");
            s
        })
        .then(definition.clone())
        .map(|s| {
            println!("s2={s:?};");
            s
        })
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

                for node in definition {
                    match node {
                        S2::Single(e) => {
                            children.push(e);
                        }
                        S2::Double(e1, e2) => {
                            children.push(e1);
                            children.push(e2);
                        }
                        _ => {}
                    }
                }

                // children.push(NodeOrToken::Token(GreenToken::new(
                //     OrgSyntaxKind::Text.into(),
                //     &definition,
                // )));

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

            for node in definition {
                match node {
                    S2::Single(e) => {
                        children.push(e);
                    }
                    S2::Double(e1, e2) => {
                        children.push(e1);
                        children.push(e2);
                    }
                    _ => {}
                }
            }

            // children.push(NodeOrToken::Token(GreenToken::new(
            //     OrgSyntaxKind::Text.into(),
            //     &definition,
            // )));

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
