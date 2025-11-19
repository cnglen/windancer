//! Paragraph environment parser
use crate::parser::element::{block, comment, drawer, horizontal_rule, item, keyword, list, table};
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, footnote_definition, latex_environment, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// A simple heading row parser WITHOUT state, used by section parser to check whether the next part is heading to stop
pub(crate) fn simple_heading_row_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let stars = just('*').repeated().at_least(1).collect::<String>();
    let whitespaces = one_of(" \t").repeated().at_least(1).collect::<String>();
    let title = none_of("\n\r").repeated().collect::<String>();
    stars
        .then(whitespaces)
        .then(title)
        .then(object::newline_or_ending())
        .map(|(((stars, ws), title), nl)| match nl {
            Some(newline_str) => format!("{}{}{}{}", stars, ws, title, newline_str),
            None => format!("{}{}{}", stars, ws, title),
        })
}

/// paragraph的实现
///

pub(crate) fn paragraph_parser<'a>(
    non_paragraph_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let inner = object::line_parser()
        .and_is(non_paragraph_parser.not())
        .and_is(object::blank_line_parser().not()) // 遇到\n+blankline停止
        .map(|s| {
            println!("paragraph_parser: inner: s={s:?}");
            s
        })
        .repeated()
        .at_least(1)
        .collect::<Vec<String>>()
        // .map(|s| s.join("-"))
        // .map(|s| {println!("paragraph_parser: inner: s={s:?}"); s})
        .to_slice();

    object::standard_set_objects_parser()
        .nested_in(inner)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(|(lines, blanklines), _e| {
            let mut children = vec![];
            for node in lines {
                children.push(node);
            }
            for blankline in blanklines {
                children.push(NodeOrToken::Token(blankline));
            }
            let node = NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Paragraph.into(), children));
            node
        })
}

pub(crate) fn paragraph_parser_to_replace<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let inner = object::line_parser()
        .and_is(latex_environment::latex_environment_parser().not())
        .and_is(block::block_parser().not())
        .and_is(horizontal_rule::horizontal_rule_parser().not())
        .and_is(keyword::keyword_parser().not())
        .and_is(drawer::drawer_parser().not())
        .and_is(comment::comment_parser().not())
        .and_is(table::table_parser().not())
        .and_is(footnote_definition::footnote_definition_parser().not())
        .and_is(
            item::item_indent_parser()
                .then(item::item_bullet_parser())
                .then(item::item_counter_set_parser().or_not())
                .then(item::item_checkbox_parser().or_not())
                .then(item::item_tag_parser().or_not())
                .not(),
        )
        .and_is(simple_heading_row_parser().not()) // 遇到\n+headingRow停止
        .and_is(object::blank_line_parser().not()) // 遇到\n+blankline停止
        .repeated()
        .at_least(1)
        .collect::<Vec<String>>()
        .map(|s| s.join(""))
        .to_slice();

    object::standard_set_objects_parser()
        .nested_in(inner)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(|(lines, blanklines), _e| {
            // println!("lines={:?}", lines);
            let mut children = vec![];

            // todo: 合并连续的多个text node
            for node in lines {
                children.push(node);
            }

            for blankline in blanklines {
                children.push(NodeOrToken::Token(blankline));
            }

            let node = NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Paragraph.into(), children));

            node
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ParserState, SyntaxNode, common::get_parser_output, element};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_paragraph_01() {
        let input = r##"paragraph
foo
bar
"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser());
        assert_eq!(
            get_parser_output(parser, input),
            r##"Paragraph@0..18
  Text@0..18 "paragraph\nfoo\nbar\n"
"##
        );
    }

    #[test]
    #[should_panic]
    fn test_paragraph_02_drawer() {
        let input = r##"drawer
:a:
abc
:end:
"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser());
        get_parser_output(parser, input);
    }

    #[test]
    #[should_panic]
    fn test_paragraph_03_block() {
        let input = r##"block:
#+begin_src python
#+end_src
"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser());
        get_parser_output(parser, input);
    }

    #[test]
    #[should_panic]
    fn test_paragraph_04_list() {
        let input = r##"list:
- a
- b
"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser());
        get_parser_output(parser, input);
    }

    #[test]
    fn test_paragraph_n_line() {
        let input = r##"foo
bar
"##;
        let mut state = RollbackState(ParserState::default());
        let r = paragraph_parser(element::element_in_paragraph_parser())
            .parse_with_state(input, &mut state);

        for e in r.errors() {
            println!("error={:?}", e);
        }

        let syntax_tree = SyntaxNode::new_root(r.into_result().unwrap().into_node().expect("xxx"));
    }

    #[test]
    fn test_block() {
        let input = r##"#+begin_src python
#+end_src
"##;
        let mut state = RollbackState(ParserState::default());
        assert_eq!(
            block::block_parser()
                .parse_with_state(input, &mut state)
                .has_output(),
            true
        );
    }
}
