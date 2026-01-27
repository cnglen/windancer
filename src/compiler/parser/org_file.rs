//! OrgFile parser
use crate::compiler::parser::{MyState, NT, OSK};
use crate::compiler::parser::{element, object};
use chumsky::prelude::*;

use super::config::OrgParserConfig;

/// document <- zeroth_section? heading_subtree*
/// zeroth_section <- blank_line* comment? property_drawer? section?
pub(crate) fn org_file_parser<'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, extra::Full<Rich<'a, char>, MyState, ()>> + Clone {
    let parser = object::blank_line_parser()
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .or_not()
        .then(element::comment::comment_parser().or_not())
        .then(element::drawer::property_drawer_parser().or_not())
        .then(
            element::section::section_parser(element::element_in_section_parser(config.clone()))
                .or_not(),
        )
        .then(
            element::heading::heading_subtree_parser(
                config.clone().org_todo_keywords,
                object::standard_set_object_parser::<()>(config.clone()),
                element::element_parser(config.clone()),
                "",
            )
            .repeated()
            .collect::<Vec<_>>(),
        )
        .map(
            |(
                (((maybe_blank_lines, maybe_comment), maybe_property_drawer), maybe_section),
                headings,
            )| {
                let mut children = Vec::new();
                if let Some(blank_lines) = maybe_blank_lines {
                    children.extend(blank_lines);
                }

                let estimated_0 = maybe_comment.as_ref().map(|_| 1).unwrap_or(0)
                    + maybe_property_drawer.as_ref().map(|_| 1).unwrap_or(0);
                let mut children_in_section_preamble = Vec::with_capacity(estimated_0);
                children_in_section_preamble.extend(maybe_comment.into_iter());
                children_in_section_preamble.extend(maybe_property_drawer.into_iter());
                let zeroth_section_preamble =
                    crate::node!(OSK::ZerothSectionPreamble, children_in_section_preamble);

                let estimated_1 = maybe_section
                    .as_ref()
                    .map(|s| s.as_node().unwrap().children().count())
                    .unwrap_or(0);
                let mut children_in_section = Vec::with_capacity(estimated_1 + 1);
                children_in_section.push(zeroth_section_preamble);
                if let Some(section) = maybe_section {
                    children_in_section
                        .extend(section.as_node().unwrap().children().map(|e| e.to_owned()));
                }

                if !children_in_section.is_empty() {
                    let zeroth_section = crate::node!(OSK::Section, children_in_section);
                    children.push(zeroth_section);
                }

                children.extend(headings);

                crate::node!(OSK::OrgFile, children)
            },
        );

    Parser::boxed(parser)
}

#[cfg(test)]
mod tests {
    use crate::compiler::parser::config::OrgParserConfig;
    use crate::compiler::parser::{common::get_parser_output, org_file::org_file_parser};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_doc_01() {
        let input = "* 标题1\n 测试\n** 标题1.1\n测试\n测试\ntest\n*** 1.1.1 title\nContent\n* Title\nI have a dream\n"; // (signal: 11, SIGSEGV: invalid memory reference)
        let parser = org_file_parser(OrgParserConfig::default());

        let expected_output = r###"OrgFile@0..97
  HeadingSubtree@0..74
    HeadingRow@0..10
      HeadingRowStars@0..1 "*"
      Whitespace@1..2 " "
      HeadingRowTitle@2..9
        Text@2..9 "标题1"
      Newline@9..10 "\n"
    Section@10..18
      Paragraph@10..18
        Text@10..18 " 测试\n"
    HeadingSubtree@18..74
      HeadingRow@18..31
        HeadingRowStars@18..20 "**"
        Whitespace@20..21 " "
        HeadingRowTitle@21..30
          Text@21..30 "标题1.1"
        Newline@30..31 "\n"
      Section@31..50
        Paragraph@31..50
          Text@31..50 "测试\n测试\ntest\n"
      HeadingSubtree@50..74
        HeadingRow@50..66
          HeadingRowStars@50..53 "***"
          Whitespace@53..54 " "
          HeadingRowTitle@54..65
            Text@54..65 "1.1.1 title"
          Newline@65..66 "\n"
        Section@66..74
          Paragraph@66..74
            Text@66..74 "Content\n"
  HeadingSubtree@74..97
    HeadingRow@74..82
      HeadingRowStars@74..75 "*"
      Whitespace@75..76 " "
      HeadingRowTitle@76..81
        Text@76..81 "Title"
      Newline@81..82 "\n"
    Section@82..97
      Paragraph@82..97
        Text@82..97 "I have a dream\n"
"###;

        assert_eq!(get_parser_output(parser, input), expected_output);
    }

    #[test]
    fn test_doc_02() {
        let input = "* 标题1\na";
        let parser = org_file_parser(OrgParserConfig::default());
        let expected_output = r##"OrgFile@0..11
  HeadingSubtree@0..11
    HeadingRow@0..10
      HeadingRowStars@0..1 "*"
      Whitespace@1..2 " "
      HeadingRowTitle@2..9
        Text@2..9 "标题1"
      Newline@9..10 "\n"
    Section@10..11
      Paragraph@10..11
        Text@10..11 "a"
"##;

        assert_eq!(get_parser_output(parser, input), expected_output);
    }
}

// todo: test of radio link
