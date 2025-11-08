//! element parser: greater? excluding heading/section

use crate::parser::{
    ParserState, block, comment, drawer, footnote_definition, horizontal_rule, keyword,
    latex_environment, paragraph, section, table,
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
        keyword::keyword_parser(),
        drawer::drawer_parser(),
        horizontal_rule::horizontal_rule_parser(),
        latex_environment::latex_environment_parser(),
        comment::comment_parser(),
        paragraph::paragraph_parser(),
        section::section_unknown_parser(),
    ))
}
