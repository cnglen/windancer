//! element parser: greater? excluding heading/section

pub(crate) mod block;
pub(crate) mod comment;
pub(crate) mod drawer;
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
use crate::parser::syntax::OrgSyntaxKind;

use crate::parser::{ParserState, object};

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
    let mut element_in_drawer = Recursive::declare();
    let mut heading_subtree = Recursive::declare();

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
    let footnote_definition =
        footnote_definition::footnote_definition_parser(element_without_tablerow_and_item.clone());
    let center_block = block::center_block_parser(element_without_tablerow_and_item.clone());
    let quote_block = block::quote_block_parser(element_without_tablerow_and_item.clone());
    let special_block = block::special_block_parser(element_without_tablerow_and_item.clone());
    let drawer = drawer::drawer_parser(element_in_drawer.clone());
    let plain_list =
        list::plain_list_parser(item::item_parser(element_without_tablerow_and_item.clone()));
    heading_subtree.define(
        heading::heading_row_parser()
            .then(
                section::section_parser(element_without_tablerow_and_item.clone())
                    .repeated()
                    .at_most(1)
                    .collect::<Vec<_>>(),
            )
            .then(heading_subtree.clone().repeated().collect::<Vec<_>>())
            .map_with(|((headline_title, section), children), e| {
                // println!(
                //     "headline_title={:?}\nsection={:?}\nchildren={:?}",
                //     headline_title, section, children
                // );
                let mut children_ = vec![];
                children_.push(headline_title.green);
                for e in section {
                    children_.push(e);
                }
                for c in children {
                    children_.push(c);
                }
                let span: SimpleSpan = e.span();
                e.state().0.level_stack.pop();
                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::HeadingSubtree.into(),
                    children_,
                ))
            }),
    );

    let non_paragraph_element_parser = Parser::boxed(choice((
        heading_subtree.clone(),
        footnote_definition.clone(),
        drawer.clone(),
        plain_list.clone(),
        horizontal_rule.clone(),
        latex_environment.clone(),
        keyword.clone(),
        center_block.clone(),
        quote_block.clone(),
        special_block.clone(),
        src_block.clone(),
        export_block.clone(),
        verse_block.clone(),
        example_block.clone(),
        comment_block.clone(),
        planning.clone(),
        comment.clone(),
        table.clone(),
    )));
    let paragraph_parser = paragraph::paragraph_parser(non_paragraph_element_parser.clone()); // non_paragraph_element_parser used to negative lookehead

    element_without_tablerow_and_item.define(choice((
        non_paragraph_element_parser.clone(),
        paragraph_parser.clone(),
    )));

    // element in section: without heading
    let non_paragraph_element_parser_in_section = Parser::boxed(choice((
        footnote_definition.clone(),
        special_block.clone(),
        drawer.clone(),
        plain_list.clone(),
        horizontal_rule.clone(),
        latex_environment.clone(),
        keyword.clone(),
        center_block.clone(),
        quote_block.clone(),
        special_block.clone(),
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

    let non_paragraph_element_parser_in_drawer = Parser::boxed(choice((
        heading_subtree.clone(),
        footnote_definition.clone(),
        plain_list.clone(),
        horizontal_rule.clone(),
        latex_environment.clone(),
        keyword.clone(),
        center_block.clone(),
        quote_block.clone(),
        special_block.clone(),
        src_block.clone(),
        export_block.clone(),
        verse_block.clone(),
        example_block.clone(),
        comment_block.clone(),
        planning.clone(),
        comment.clone(),
        table.clone(),
    )));
    element_in_drawer.define(choice((
        non_paragraph_element_parser_in_drawer.clone(),
        paragraph_parser.clone(),
    )));

    (
        element_without_tablerow_and_item,
        non_paragraph_element_parser,
        element_in_section,
        heading_subtree,
        element_in_drawer,
    )
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

pub(crate) fn element_in_item_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    get_element_parser().2
}

pub(crate) fn heading_subtree_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    get_element_parser().3
}

pub(crate) fn element_in_drawer_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    get_element_parser().4
}
