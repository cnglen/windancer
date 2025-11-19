//! element parser: greater? excluding heading/section

pub(crate) mod block;
pub(crate) mod comment;
pub(crate) mod drawer;
pub(crate) mod horizontal_rule;
pub(crate) mod item;
pub(crate) mod keyword;
pub(crate) mod list;
pub(crate) mod paragraph;
pub(crate) mod planning;
pub(crate) mod table;
pub(crate) mod section;
pub(crate) mod heading;

use crate::parser::{ParserState, footnote_definition, latex_environment, object};

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use drawer::node_property_parser;
use paragraph::paragraph_parser;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// FIXME:
///   递归的parser不覆盖，如list_parser，更好地办法?
///   - list_parser -> element_parser -> list_parser ...
// pub(crate) fn element_parser<'a>() -> impl Parser<
//     'a,
//     &'a str,
//     NodeOrToken<GreenNode, GreenToken>,
//     extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
// > + Clone {
//     choice((
//         footnote_definition::footnote_definition_parser(),
//         block::block_parser(),
//         drawer::drawer_parser(),
//         table::table_parser(),
//         keyword::keyword_parser(),
//         horizontal_rule::horizontal_rule_parser(),
//         latex_environment::latex_environment_parser(),
//         comment::comment_parser(),
//         // paragraph::paragraph_parser(),
//         section::section_unknown_parser(),
//     ))
// }


// pub(crate) fn element_parser_in_list<'a>() -> impl Parser<
//         'a,
//     &'a str,
//     NodeOrToken<GreenNode, GreenToken>,
//     extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
//     > + Clone {
//     get_element_parser().1
// }

pub(crate) fn get_element_parser<'a>() -> (
    impl Parser<
            'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
    impl Parser<
            'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
        > + Clone,
    impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) {

    // let mut element_in_pagraph = Recursive::declare();
    let mut element_without_tablerow_and_item = Recursive::declare();
    let mut element_in_section = Recursive::declare();    

    // item only in list; table_row only in table;
    // section only in heading_subtree or before first heading

    // elements_in_item: non_item; table_row;
    // elements_in_paragraph: non_item; non_table_row; non_paragraph;
    // elements_in_drawer: non drawer;
    // elements_in_section: 

    // checked
    let horizontal_rule = horizontal_rule::horizontal_rule_parser();
    let latex_environment = latex_environment::latex_environment_parser();
    let keyword = keyword::keyword_parser();
    let src_block = block::src_block_parser();
    let export_block = block::export_block_parser();
    let verse_block = block::verse_block_parser();
    let example_block = block::example_block_parser();
    let comment_block = block::comment_block_parser();
    let planning = planning::planning_parser();
    let comment = comment::comment_parser();
    let table = table::table_parser();

    // to check
    let footnote_definition = footnote_definition::footnote_definition_parser();
    let block = block::block_parser();
    let drawer = drawer::drawer_parser();
    // let section_unknown = section::section_unknown_parser();
    let plain_list = list::plain_list_parser(item::item_parser(
        element_without_tablerow_and_item.clone(),
    ));

    // fixme: overflow!!
    // let section = section::section_parser(element_without_tablerow_and_item.clone());
    let heading_subtree = heading::heading_subtree_parser(section::section_parser(element_without_tablerow_and_item.clone()));
    
    let non_paragraph_element_parser = Parser::boxed(
        choice((
            heading_subtree.clone(),
            footnote_definition.clone(),
            block.clone(),
            drawer.clone(),
            plain_list.clone(),
            horizontal_rule.clone(),
            latex_environment.clone(),
            keyword.clone(),
            src_block.clone(),
            export_block.clone(),
            verse_block.clone(),
            example_block.clone(),
            comment_block.clone(),
            planning.clone(),
            comment.clone(),
            table.clone(),
        )));
    let paragraph_parser = paragraph::paragraph_parser(non_paragraph_element_parser.clone());

    element_without_tablerow_and_item.define(choice((
        non_paragraph_element_parser.clone(),
        paragraph_parser.clone(),
    )));


    // element in section: without heading
    let non_paragraph_element_parser_in_section = Parser::boxed(
        choice((
            footnote_definition.clone(),
            block.clone(),
            drawer.clone(),
            plain_list.clone(),
            horizontal_rule.clone(),
            latex_environment.clone(),
            keyword.clone(),
            src_block.clone(),
            export_block.clone(),
            verse_block.clone(),
            example_block.clone(),
            comment_block.clone(),
            planning.clone(),
            comment.clone(),
            table.clone(),
        )));

    element_in_section.define(choice((
        non_paragraph_element_parser_in_section.clone(),
        paragraph_parser.clone(),
    )));
    
    (element_without_tablerow_and_item, non_paragraph_element_parser, element_in_section)
}

pub(crate) fn element_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone {
    get_element_parser().0
}

pub(crate) fn element_in_paragraph_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone {
    get_element_parser().1
}

pub(crate) fn element_in_section_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone {
    get_element_parser().2
}
