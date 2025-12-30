//! Document parser
use crate::parser::ParserState;
use crate::parser::{NT, OSK};
use crate::parser::{element, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;

/// document <- zeroth_section? heading_subtree*
/// zeroth_sectoin <- blank_line* comment? property_drawer? section?
pub(crate) fn document_parser<'a>()
-> impl Parser<'a, &'a str, NT, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> {
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
                let mut children = Vec::new();
                if let Some(blank_lines) = maybe_blank_lines {
                    children.extend(blank_lines);
                }

                let estimated = maybe_comment.as_ref().map(|_| 1).unwrap_or(0)
                    + maybe_property_drawer.as_ref().map(|_| 1).unwrap_or(0)
                    + maybe_section
                        .as_ref()
                        .map(|s| s.as_node().unwrap().children().count())
                        .unwrap_or(0);

                let mut children_in_section = Vec::with_capacity(estimated);
                children_in_section.extend(maybe_comment.into_iter());
                children_in_section.extend(maybe_property_drawer.into_iter());
                if let Some(section) = maybe_section {
                    children_in_section
                        .extend(section.as_node().unwrap().children().map(|e| e.to_owned()));
                }

                if !children_in_section.is_empty() {
                    let zeroth_section = crate::node!(OSK::Section, children_in_section);
                    children.push(zeroth_section);
                }

                children.extend(headings);

                crate::node!(OSK::Document, children)
            },
        );

    Parser::boxed(parser)
}
