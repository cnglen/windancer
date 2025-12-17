//! Document parser
use crate::parser::ParserState;
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{element, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// document <- zeroth_section? heading_subtree*
/// zeroth_sectoin <- blank_line* comment? property_drawer? section?
pub(crate) fn document_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> {
    let parser = object::blank_line_parser()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .or_not()
        .then(element::comment::comment_parser().or_not())
        .then(element::drawer::property_drawer_parser().or_not())
        .then(element::section::section_parser(element::element_in_section_parser()).or_not())
        .then(
            element::heading::heading_subtree_parser(element::element_parser(), 0)
                .repeated()
                .collect::<Vec<_>>(),
        )
        .map(
            |(
                (((maybe_blank_lines, maybe_comment), maybe_property_drawer), maybe_section),
                headings,
            )| {
                let mut children = vec![];
                if let Some(blank_lines) = maybe_blank_lines {
                    for blank_line in blank_lines {
                        children.push(NodeOrToken::Token(blank_line));
                    }
                }

                let mut children_in_section = vec![];
                if let Some(comment) = maybe_comment {
                    children_in_section.push(comment);
                }
                if let Some(property_drawer) = maybe_property_drawer {
                    children_in_section.push(property_drawer);
                }
                if let Some(section) = maybe_section {
                    for e in section.as_node().unwrap().children() {
                        children_in_section.push(e.to_owned());
                    }
                }

                if children_in_section.len() > 0 {
                    let zeroth_section = NodeOrToken::Node(GreenNode::new(
                        OrgSyntaxKind::Section.into(),
                        children_in_section,
                    ));
                    children.push(zeroth_section);
                }

                for c in headings {
                    children.push(c);
                }
                let node = GreenNode::new(OrgSyntaxKind::Document.into(), children);

                NodeOrToken::Node(node)
            },
        );

    Parser::boxed(parser)
}
