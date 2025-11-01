//! timestamp parser
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

use super::whitespaces_g1;

/// timestamp parser: <<TIMESTAMP>>
pub(crate) fn timestamp_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let yyyymmdd = one_of("0123456789")
        .repeated()
        .at_least(4)
        .at_most(4)
        .then(just("-"))
        .then(one_of("0123456789").repeated().at_least(2).at_most(2))
        .then(just("-"))
        .then(one_of("0123456789").repeated().at_least(2).at_most(2));
    let daytime = none_of(" \t+-]>0123456789\n").repeated().at_least(1);

    let date = yyyymmdd.then(whitespaces_g1().then(daytime).or_not());
    let time = one_of("0123456789")
        .repeated()
        .at_least(1)
        .at_most(2)
        .then(just(":"))
        .then(one_of("0123456789").repeated().at_least(2).at_most(2));
    let repeater_or_day = just("++")
        .or(just(".+"))
        .or(just("+"))
        .or(just("--"))
        .or(just("+"))
        .then(one_of("0123456789").repeated().at_least(1))
        .then(one_of("hdwmy"));

    let p1a = just("<")
        .then(date.clone())
        .then(whitespaces_g1().then(time).or_not())
        .then(whitespaces_g1().then(repeater_or_day).or_not())
        .then(just(">"))
        .to_slice()
        .map_with(|s, e| {
            e.state().prev_char = s.chars().last();

            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                s,
            )));

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Timestamp.into(),
                children,
            )))
        });

    let p1b = just("[")
        .then(date.clone())
        .then(whitespaces_g1().then(time).or_not())
        .then(whitespaces_g1().then(repeater_or_day).or_not())
        .then(just("]"))
        .to_slice()
        .map_with(|s, e| {
            e.state().prev_char = s.chars().last();

            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                s,
            )));

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Timestamp.into(),
                children,
            )))
        });

    let p2a = p1a
        .clone()
        .then(just("--"))
        .then(p1a.clone())
        .to_slice()
        .map_with(|s, e| {
            e.state().prev_char = s.chars().last();

            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                s,
            )));

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Timestamp.into(),
                children,
            )))
        });

    let p2b = p1b
        .clone()
        .then(just("--"))
        .then(p1b.clone())
        .to_slice()
        .map_with(|s, e| {
            e.state().prev_char = s.chars().last();

            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                s,
            )));

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Timestamp.into(),
                children,
            )))
        });

    let p3a = just("<")
        .then(date.clone())
        .then(whitespaces_g1().then(time).then(just("-").then(time)))
        .then(whitespaces_g1().then(repeater_or_day).or_not())
        .then(just(">"))
        .to_slice()
        .map_with(|s, e| {
            e.state().prev_char = s.chars().last();

            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                s,
            )));

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Timestamp.into(),
                children,
            )))
        });

    let p3b = just("[")
        .then(date.clone())
        .then(whitespaces_g1().then(time).then(just("-").then(time)))
        .then(whitespaces_g1().then(repeater_or_day).or_not())
        .then(just("]"))
        .to_slice()
        .map_with(|s, e| {
            e.state().prev_char = s.chars().last();

            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                s,
            )));

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Timestamp.into(),
                children,
            )))
        });

    p2a.or(p2b).or(p3a).or(p3b).or(p1a).or(p1b)
}
