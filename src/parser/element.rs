//! element parser: greater? excluding heading/section
pub(crate) mod block;
pub(crate) mod comment;
pub(crate) mod drawer;
pub(crate) mod fixed_width;
pub(crate) mod footnote_definition;
pub(crate) mod heading;
pub(crate) mod horizontal_rule;
pub(crate) mod item;
pub(crate) mod keyword;
pub(crate) mod latex_environment;
pub(crate) mod list;
pub(crate) mod paragraph;
pub(crate) mod planning;
pub(crate) mod section;
pub(crate) mod table;

use crate::parser::{MyExtra, NT};
use chumsky::prelude::*;

// heading should not be in here, since it's parsed by document!
pub(crate) fn get_element_parser<'a, C: 'a + std::default::Default>() -> (
    impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
    impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone,
    impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
    impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
) {
    // let mut element_in_pagraph = Recursive::declare();
    let mut element_without_tablerow_and_item = Recursive::declare();
    let mut element_in_section = Recursive::declare();
    let mut element_in_drawer = Recursive::declare();

    // item only in list; table_row only in table;
    // section only in heading_subtree or before first heading
    // planning only in heading

    // elements_in_item: non_item; table_row;
    // elements_in_paragraph: non_item; non_table_row; non_paragraph;
    // elements_in_drawer: non drawer;
    // elements_in_section:

    // checked
    let horizontal_rule = horizontal_rule::horizontal_rule_parser();
    let latex_environment = latex_environment::latex_environment_parser();
    let src_block = block::src_block_parser();
    let export_block = block::export_block_parser();
    let verse_block = block::verse_block_parser();
    let example_block = block::example_block_parser();
    let comment_block = block::comment_block_parser();
    // let planning = planning::planning_parser();
    let comment = comment::comment_parser();
    let table = table::table_parser();
    let fixed_width = fixed_width::fixed_width_parser();

    // to check
    let keyword = keyword::keyword_parser();
    let footnote_definition =
        footnote_definition::footnote_definition_parser(element_without_tablerow_and_item.clone());
    let center_block = block::center_block_parser(element_without_tablerow_and_item.clone());
    let quote_block = block::quote_block_parser(element_without_tablerow_and_item.clone());
    let special_block = block::special_block_parser(element_without_tablerow_and_item.clone());
    let drawer = drawer::drawer_parser(element_in_drawer.clone());
    let plain_list =
        list::plain_list_parser(item::item_parser(element_without_tablerow_and_item.clone()));

    // ONLY used for lookhead
    // IN paragraph_parser(), simple_heading/simple_table/simple_footnote_definition is used for lookahead, no need here for performance:
    // - we don't include heading subtree to avoid stackoverflow
    // - we don't include table/drawer/center_block/quote_block/special_block/verse_block/fixed_width/latex_environment_parser
    let non_paragraph_element_parser_used_in_lookahead =
        choice((keyword::simple_keyword_parser(),));
    let paragraph_parser =
        paragraph::paragraph_parser(non_paragraph_element_parser_used_in_lookahead.clone()); // non_paragraph_element_parser used to negative lookehead

    element_without_tablerow_and_item.define(choice((
        footnote_definition.clone(),
        drawer.clone(),
        plain_list.clone(),
        horizontal_rule.clone(),
        latex_environment.clone(),
        center_block.clone(),
        quote_block.clone(),
        special_block.clone(),
        src_block.clone(),
        export_block.clone(),
        verse_block.clone(),
        example_block.clone(),
        comment_block.clone(),
        comment.clone(),
        table.clone(),
        fixed_width.clone(),
        keyword.clone(),
        paragraph_parser.clone(),
    )));

    element_in_section.define(choice((
        footnote_definition.clone(),
        drawer.clone(),
        plain_list.clone(),
        horizontal_rule.clone(),
        latex_environment.clone(),
        center_block.clone(),
        quote_block.clone(),
        special_block.clone(),
        src_block.clone(),
        export_block.clone(),
        verse_block.clone(),
        example_block.clone(),
        comment_block.clone(),
        comment.clone(),
        table.clone(),
        fixed_width.clone(),
        keyword.clone(),
        paragraph_parser.clone(),
    )));

    element_in_drawer.define(choice((
        footnote_definition.clone(),
        plain_list.clone(),
        horizontal_rule.clone(),
        latex_environment.clone(),
        center_block.clone(),
        quote_block.clone(),
        special_block.clone(),
        src_block.clone(),
        export_block.clone(),
        verse_block.clone(),
        example_block.clone(),
        comment_block.clone(),
        comment.clone(),
        table.clone(),
        fixed_width.clone(),
        keyword.clone(),
        paragraph_parser.clone(),
    )));

    (
        Parser::boxed(element_without_tablerow_and_item),
        Parser::boxed(non_paragraph_element_parser_used_in_lookahead),
        Parser::boxed(element_in_section),
        Parser::boxed(element_in_drawer),
    )
}

pub(crate) fn element_parser<'a, C: 'a + std::default::Default>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    get_element_parser::<C>().0
}

#[allow(unused)]
pub(crate) fn elements_parser<'a, C: 'a + std::default::Default>()
-> impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone {
    element_parser::<C>()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
}

#[allow(unused)]
pub(crate) fn element_in_paragraph_parser<'a, C: 'a + std::default::Default>()
-> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    get_element_parser::<C>().1
}

pub(crate) fn element_in_section_parser<'a, C: 'a + std::default::Default>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    get_element_parser::<C>().2
}

#[allow(unused)]
pub(crate) fn element_in_item_parser<'a, C: 'a + std::default::Default>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    get_element_parser::<C>().2
}

#[allow(unused)]
pub(crate) fn element_in_drawer_parser<'a, C: 'a + std::default::Default>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    get_element_parser::<C>().3
}
