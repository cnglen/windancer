//! Keyword parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn keyword_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    just("#+")
        .then(none_of(" \t:").repeated().at_least(1).collect::<String>())
        .then(just(":"))
        .then(object::whitespaces())
        .then(none_of("\n").repeated().collect::<String>())
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(
            |(((((((hash_plus, key), colon), ws1), value), ws2), nl), blanklines)| {
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::HashPlus.into(),
                    hash_plus,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &key,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    colon,
                )));

                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws1,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &value,
                )));

                if ws2.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws2,
                    )));
                }

                match nl {
                    Some(newline) => {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Newline.into(),
                            &newline,
                        )));
                    }
                    None => {}
                }
                for blankline in blanklines {
                    children.push(NodeOrToken::Token(blankline));
                }

                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Keyword.into(), children))
            },
        )
}
