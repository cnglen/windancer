//! element parser: greater? excluding heading/section

pub(crate) mod block;
pub(crate) mod comment;
pub(crate) mod drawer;
pub(crate) mod horizontal_rule;
pub(crate) mod keyword;
pub(crate) mod table;

use crate::parser::{
    ParserState, footnote_definition, latex_environment, object, paragraph, section,
};

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// FIXME:
///   递归的parser不覆盖，如list_parser，更好地办法?
///   - list_parser -> element_parser -> list_parser ...
pub(crate) fn element_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    choice((
        footnote_definition::footnote_definition_parser(),
        block::block_parser(),
        table::table_parser(),
        keyword::keyword_parser(object::standard_set_object_parser()),
        drawer::drawer_parser(),
        horizontal_rule::horizontal_rule_parser(),
        latex_environment::latex_environment_parser(),
        comment::comment_parser(),
        paragraph::paragraph_parser(),
        section::section_unknown_parser(),
    ))
}
