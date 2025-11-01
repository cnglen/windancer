//! Drawer parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn drawer_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let drawer_name_row = object::whitespaces()
        .then(just(":"))
        .then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '-'))
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .then(just(":"))
        .then(object::whitespaces())
        .then(just("\n"))
        .map(|(((((ws1, c1), name), c2), ws2), nl)| {
            // println!(
            //     "drawer begin row: ws1={}, c1={}, name={}, c2={}, ws2={}, nl={}",
            //     ws1, c1, name, c2, ws2, nl
            // );
            let mut tokens = vec![];

            if ws1.len() > 0 {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }
            tokens.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                c1,
            )));

            tokens.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &name,
            )));

            tokens.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                c2,
            )));
            if ws2.len() > 0 {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }
            tokens.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Newline.into(),
                nl,
            )));

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::DrawerBegin.into(),
                tokens,
            ))
        });

    let drawer_end_row = object::whitespaces()
        .then(object::just_case_insensitive(":end:"))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .map(|(((ws1, end), ws2), nl)| {
            let mut tokens = vec![];

            if ws1.len() > 0 {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }
            tokens.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &end,
            )));
            if ws2.len() > 0 {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }
            match nl {
                Some(_nl) => tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    &_nl,
                ))),
                None => {}
            }
            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::DrawerEnd.into(),
                tokens,
            ))
        });

    let drawer_content = any()
        .and_is(drawer_end_row.clone().not())
        .repeated()
        .collect::<String>();

    let blank_lines = object::blank_line_parser().repeated().collect::<Vec<_>>();

    drawer_name_row
        .then(drawer_content)
        .then(drawer_end_row)
        .then(blank_lines)
        .map(|(((begin, content), end), blank_lines)| {
            let mut children = vec![];

            children.push(begin);

            if content.len() > 0 {
                let mut c_children = vec![];

                let token =
                    NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Text.into(), &content));
                let mut content_node_children = vec![];
                content_node_children.push(token);

                c_children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::Paragraph.into(),
                    content_node_children,
                )));

                let node = NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::DrawerContent.into(),
                    c_children,
                ));
                children.push(node);
            }

            children.push(end);
            for bl in blank_lines {
                children.push(NodeOrToken::Token(bl));
            }

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Drawer.into(),
                children,
            ))
        })
}
