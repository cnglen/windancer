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
        .repeated()
        .at_least(1)
        .collect::<Vec<String>>()
        // .map(|s| s.join(""))
        // .map(|s| {println!("paragraph_parser@inner:s=|{s:?}|"); s})
        .to_slice();

    object::standard_set_objects_parser()
        .nested_in(inner)
        // inner
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(|(lines, blanklines), _e| {
            let mut children = vec![];
            // children.push(NodeOrToken::Token(GreenToken::new(
            //     OrgSyntaxKind::Text.into(),
            //     &lines,
            // )));

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
    use crate::parser::{
        ParserState, SyntaxNode, common::get_parser_output, common::get_parsers_output, element,
    };
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
    fn test_paragraph_05() {
        let input = r##"paragraph"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser());
        assert_eq!(
            get_parser_output(parser, input),
            r##"Paragraph@0..9
  Text@0..9 "paragraph"
"##
        );
    }

    #[test]
    fn test_paragraph_06() {
        let input = r##"paragraph
"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser());
        assert_eq!(
            get_parser_output(parser, input),
            r##"Paragraph@0..10
  Text@0..10 "paragraph\n"
"##
        );
    }

    #[test]
    fn test_paragraph_07() {
        let input = r##"text
#+begin_center
center
#+end_center
"##;
        //         let parser = paragraph_parser(element::element_in_paragraph_parser());
        //         assert_eq!(
        //             get_parser_output(parser, input),
        //             r##"
        // "##
        //         );

        assert_eq!(
            get_parsers_output(
                element::element_parser().repeated().collect::<Vec<_>>(),
                input
            ),
            r##"Root@0..40
  Paragraph@0..5
    Text@0..5 "text\n"
  CenterBlock@5..40
    BlockBegin@5..20
      Text@5..13 "#+begin_"
      Text@13..19 "CENTER"
      Newline@19..20 "\n"
    BlockContent@20..27
      Paragraph@20..27
        Text@20..27 "center\n"
    BlockEnd@27..40
      Text@27..33 "#+end_"
      Text@33..39 "CENTER"
      Newline@39..40 "\n"
"##
        );
    }

    #[test]
    fn test_paragraph_08() {
        let input = r##"text
#+begin_example
example
#+end_example
"##;
        assert_eq!(
            get_parsers_output(
                element::element_parser().repeated().collect::<Vec<_>>(),
                input
            ),
            r##"Root@0..43
  Paragraph@0..5
    Text@0..5 "text\n"
  ExampleBlock@5..43
    BlockBegin@5..21
      Text@5..13 "#+begin_"
      Text@13..20 "EXAMPLE"
      Newline@20..21 "\n"
    BlockContent@21..29
      Text@21..29 "example\n"
    BlockEnd@29..43
      Text@29..35 "#+end_"
      Text@35..42 "EXAMPLE"
      Newline@42..43 "\n"
"##
        );
    }
}
