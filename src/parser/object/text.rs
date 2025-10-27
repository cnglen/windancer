//! text parser
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::object::angle_link::angle_link_parser;
use crate::parser::object::entity::entity_parser;
use crate::parser::object::footnote_reference::footnote_reference_parser;
use crate::parser::object::latex_fragment::latex_fragment_parser;
use crate::parser::object::line_break_parser;
use crate::parser::object::r#macro::macro_parser;
use crate::parser::object::regular_link::regular_link_parser;
use crate::parser::object::subscript_superscript::superscript_parser;
use crate::parser::object::target::target_parser;
use crate::parser::object::text_markup::text_markup_parser;
use crate::parser::object::timestamp::timestamp_parser;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// Text Parser
pub(crate) fn text_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    any()
        .and_is(text_markup_parser().not())
        .and_is(entity_parser().not())
        .and_is(regular_link_parser().not())
        .and_is(angle_link_parser().not())
        .and_is(latex_fragment_parser().not())
        .and_is(footnote_reference_parser().not())
        .and_is(line_break_parser().not())
        .and_is(macro_parser().not())
        .and_is(superscript_parser().not())
        .and_is(target_parser().not())
        .and_is(timestamp_parser().not())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map_with(|s, e| {
            // let z: &mut MapExtra<'_, '_, &str, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>> = e;
            if let Some(c) = s.chars().last() {
                e.state().prev_char = Some(c);
            }

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Token(
                GreenToken::new(OrgSyntaxKind::Text.into(), &s),
            ))
        })
}
