//! Document parser
use crate::parser::ParserResult;
use crate::parser::ParserState;
use crate::parser::syntax::OrgSyntaxKind;
// use crate::parser::SyntaxNode;

use crate::parser::heading;
use crate::parser::section;
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, NodeOrToken};
use std::ops::Range;

/// Document parser: [section]? + heading+
/// - Document
///   - Zeroth Section
///   - HeadingSubtree
///   - ...
///   - HeadingSubtree

pub(crate) fn document_parser<'a>()
-> impl Parser<'a, &'a str, ParserResult, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>
{
    section::section_parser()
        .repeated()
        .at_most(1)
        .collect::<Vec<ParserResult>>()
        .then(
            heading::heading_subtree_parser()
                .repeated()
                .collect::<Vec<ParserResult>>(),
        )
        .map_with(|(section, _children), e| {
            let span: SimpleSpan = e.span();

            let mut children = vec![];
            let mut text = String::new();

            for c in section.iter() {
                children.push(c.green.clone());
            }

            for c in _children.iter() {
                children.push(c.green.clone());
                text.push_str(&c.text);
            }

            let radio_targets = e.state().radio_targets.clone();

            // println!("zeroth section={:#?}", section);
            let node = GreenNode::new(OrgSyntaxKind::Document.into(), children);
            // println!("{:#?}", SyntaxNode::new_root(node.clone()));
            ParserResult {
                green: NodeOrToken::Node(node),
                text: format!("{}", text),
                span: Range {
                    start: span.start,
                    end: span.end,
                },
            }
        })
}
