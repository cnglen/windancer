//! Paragraph parser
use crate::parser::element::keyword;
use crate::parser::{MyExtra, NT, OSK};
use crate::parser::{element, object};
use chumsky::prelude::*;

// non_paragraph_parser: used for negative lookahead
pub(crate) fn paragraph_parser<'a, C: 'a + std::default::Default>(
    non_paragraph_parser: impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    paragraph_parser_with_at_least_n_affiliated_keywords(non_paragraph_parser, 0)
}

pub(crate) fn paragraph_parser_with_at_least_n_affiliated_keywords<
    'a,
    C: 'a + std::default::Default,
>(
    non_paragraph_parser: impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone + 'a,
    n: usize,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let affiliated_keywords = keyword::affiliated_keyword_parser()
        .repeated()
        .at_least(n)
        .collect::<Vec<_>>();

    // Empty lines and other elements end paragraphs
    let inner = object::line_parser()
        .and_is(
            // use simple parsers for lookahead to reduce rewind() and speed up
            choice((
                object::blank_line_parser().ignored(),
                element::heading::simple_heading_row_parser().ignored(), // heading_tree is recursive, we use simple heading row for lookahead to avoid stackoverflow
                element::table::simple_table_parser(),
                element::footnote_definition::simple_footnote_definition_parser(),
                just("#+")
                    .ignore_then(
                        one_of("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_[]")
                            .repeated()
                            .at_least(1),
                    )
                    .ignore_then(just(":"))
                    .ignored(),
                // element::list::simple_plain_list_parser(element::item::simple_item_parser()),
                element::plain_list::simple_plain_list_parser(),
                element::drawer::simple_drawer_parser(),
                element::block::simple_center_block_parser(),
                element::block::simple_quote_block_parser(),
                element::block::simple_special_block_parser(),
                element::block::simple_verse_block_parser(),
                element::latex_environment::simple_latex_environment_parser(),
                element::block::simple_src_block_parser(),
                element::block::simple_export_block_parser(),
                element::block::simple_example_block_parser(),
                element::block::simple_comment_block_parser(),
                element::fixed_width::simple_fixed_width_parser(),
                element::horizontal_rule::horizontal_rule_parser().ignored(),
                element::comment::comment_parser().ignored(),
                non_paragraph_parser.ignored(), // other element, this is necessary to find the end of paragraph even thougn paragraph is the last element of choice
            ))
            .not(),
        )
        .repeated()
        .at_least(1)
        .to_slice();

    affiliated_keywords
        .then(object::standard_set_objects_parser().nested_in(inner))
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(|((keywords, lines), blanklines), _e| {
            let mut children = Vec::with_capacity(keywords.len() + lines.len() + blanklines.len());
            children.extend(keywords);
            children.extend(lines);
            children.extend(blanklines);
            crate::node!(OSK::Paragraph, children)
        })
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{common::get_parser_output, common::get_parsers_output, element};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_paragraph_01() {
        let input = r##"paragraph
foo
bar
"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser::<()>());
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
        let parser = paragraph_parser(element::element_in_paragraph_parser::<()>());
        get_parser_output(parser, input);
    }

    #[test]
    #[should_panic]
    fn test_paragraph_03_block() {
        let input = r##"block:
#+begin_src python
#+end_src
"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser::<()>());
        get_parser_output(parser, input);
    }

    #[test]
    #[should_panic]
    fn test_paragraph_04_list() {
        let input = r##"list:
- a
- b
"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser::<()>());
        get_parser_output(parser, input);
    }

    #[test]
    fn test_paragraph_n_line() {
        let input = r##"foo
bar
"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser::<()>());

        assert_eq!(
            get_parser_output(parser, input),
            r##"Paragraph@0..8
  Text@0..8 "foo\nbar\n"
"##
        );
    }

    #[test]
    fn test_paragraph_05() {
        let input = r##"paragraph"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser::<()>());
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
        let parser = paragraph_parser(element::element_in_paragraph_parser::<()>());
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
        //         let parser = paragraph_parser(element::element_in_paragraph_parser::<()>());
        //         assert_eq!(
        //             get_parser_output(parser, input),
        //             r##"
        // "##
        //         );

        assert_eq!(
            get_parsers_output(
                element::element_parser::<()>()
                    .repeated()
                    .collect::<Vec<_>>(),
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
                element::element_parser::<()>()
                    .repeated()
                    .collect::<Vec<_>>(),
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

    #[test]
    fn test_paragraph_09() {
        let input = r##"#+caption: export block test
a paragraph
"##;
        let parser = paragraph_parser(element::element_in_paragraph_parser::<()>());
        assert_eq!(
            get_parser_output(parser, input),
            r##"Paragraph@0..41
  AffiliatedKeyword@0..29
    HashPlus@0..2 "#+"
    KeywordKey@2..9
      Text@2..9 "caption"
    Colon@9..10 ":"
    Whitespace@10..11 " "
    KeywordValue@11..28
      Text@11..28 "export block test"
    Newline@28..29 "\n"
  Text@29..41 "a paragraph\n"
"##
        );
    }
}
