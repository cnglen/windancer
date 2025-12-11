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
use crate::parser::syntax::OrgSyntaxKind;

use crate::parser::ParserState;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
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
    let mut element_in_keyword = Recursive::declare();

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
    let keyword = keyword::keyword_parser(element_in_keyword.clone());
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
            .then(section::section_parser(element_without_tablerow_and_item.clone()).or_not())
            .then(heading_subtree.clone().repeated().collect::<Vec<_>>())
            .map_with(|((headline_title, section), headings), e| {
                // println!(
                //     "headline_title={:?}\nsection={:?}\nchildren={:?}",
                //     headline_title, section, children
                // );
                let mut children = vec![];
                children.push(headline_title.green);

                if let Some(e) = section {
                    children.push(e);
                }
                for c in headings {
                    children.push(c);
                }
                e.state().0.level_stack.pop();
                // e.state().0.level_stack.pop_back();
                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::HeadingSubtree.into(),
                    children,
                ))
            }),
    );

    let non_paragraph_element_parser = choice((
        heading_subtree.clone(),
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
    ));
    let paragraph_parser = paragraph::paragraph_parser(non_paragraph_element_parser.clone()); // non_paragraph_element_parser used to negative lookehead

    element_without_tablerow_and_item.define(choice((
        non_paragraph_element_parser.clone(),
        paragraph_parser.clone(),
    )));

    // element in section: without heading
    let non_paragraph_element_parser_in_section = choice((
        footnote_definition.clone(),
        special_block.clone(),
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
    ));

    element_in_section.define(choice((
        non_paragraph_element_parser_in_section.clone(),
        paragraph_parser.clone(),
    )));

    let non_paragraph_element_parser_in_drawer = choice((
        heading_subtree.clone(),
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
    ));
    element_in_drawer.define(choice((
        non_paragraph_element_parser_in_drawer.clone(),
        paragraph_parser.clone(),
    )));

    // negative lookahead
    // dont' use keyword() here, or stackoverflow.
    let non_paragraph_element_parser_in_keyword = choice((
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
        table.clone(),
        fixed_width.clone(),
    ));
    let paragraph_parser_in_keyword =
        paragraph::paragraph_parser_with_at_least_n_affiliated_keywords(
            non_paragraph_element_parser_in_keyword.clone(),
            1,
        ); // non_paragraph_element_parser used to negative lookehead
    element_in_keyword.define(choice((
        non_paragraph_element_parser_in_keyword.clone(),
        paragraph_parser_in_keyword.clone(),
    )));

    (
        Parser::boxed(element_without_tablerow_and_item),
        Parser::boxed(non_paragraph_element_parser),
        Parser::boxed(element_in_section),
        Parser::boxed(heading_subtree),
        Parser::boxed(element_in_drawer),
        Parser::boxed(element_in_keyword),
    )
}

#[allow(unused)]
pub(crate) fn element_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    get_element_parser().0
}

#[allow(unused)]
pub(crate) fn elements_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    Vec<NodeOrToken<GreenNode, GreenToken>>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    element_parser().repeated().at_least(1).collect::<Vec<_>>()
}

#[allow(unused)]
pub(crate) fn element_in_paragraph_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    get_element_parser().1
}

#[allow(unused)]
pub(crate) fn element_in_section_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    get_element_parser().2
}

#[allow(unused)]
pub(crate) fn element_in_item_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    get_element_parser().2
}

#[allow(unused)]
pub(crate) fn heading_subtree_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    get_element_parser().3
}

#[allow(unused)]
pub(crate) fn element_in_drawer_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    get_element_parser().4
}

#[allow(unused)]
pub(crate) fn element_in_keyword_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    get_element_parser().5
}
