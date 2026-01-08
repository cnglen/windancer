//! Heading parser, including HeadingRow, HeadingSubtree
use crate::parser::config::OrgTodoKeywords;
use crate::parser::element::{drawer, planning, section};
use crate::parser::object;
use crate::parser::{MyExtra, MyState, NT, OSK};
use chumsky::prelude::*;

// todo: why usize in C
// pub(crate) fn heading_subtree_parser<'a, C:'a + std::default::Default>(
pub(crate) fn heading_subtree_parser<'a, C: 'a + std::default::Default>(
    config: OrgTodoKeywords,
    object_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
    element_parser: impl Parser<
        'a,
        &'a str,
        NT,
        // extra::Full<Rich<'a, char>, RollbackState<ParserState>, &'a str>,
        extra::Full<Rich<'a, char>, MyState, &'a str>,
    > + Clone
    + 'a,
    prev_level: &'a str,
    // ) -> impl Parser<'a, &'a str, NT, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
) -> impl Parser<'a, &'a str, NT, extra::Full<Rich<'a, char>, MyState, ()>> + Clone {
    let mut heading_subtree = Recursive::declare();

    let maybe_keyword_ws = choice((
        object::keyword_cs_parser_v2(config.requiring_action),
        object::keyword_cs_parser_v2(config.no_further_action),
    ))
    // let maybe_keyword_ws = choice((just("TODO"), just("DONE")))
    .then(object::whitespaces_g1())
    .or_not();
    let maybe_priority = just("[#")
        .then(one_of(
            "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ",
        ))
        .then(just("]"))
        .then(object::whitespaces_g1())
        .or_not();
    let maybe_comment = just("COMMENT").then(object::whitespaces_g1()).or_not();
    let stars = just('*')
        .repeated()
        .configure(|cfg, ctx: &&str| cfg.at_least((*ctx).len() + 1))
        .to_slice()
        // .map(|s: &str| s.len())
        ;

    let tags = just(":")
        .then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '#' | '@' | '%'))
                .repeated()
                .at_least(1)
                .to_slice()
                .separated_by(just(':'))
                .collect::<Vec<_>>(),
        )
        .then(just(":"))
        .then(object::whitespaces());
    let maybe_tag = tags.or_not();
    let maybe_title = none_of(object::CRLF)
        .and_is(
            one_of(" \t")
                .repeated()
                .at_least(1)
                .ignore_then(tags)
                .ignored()
                .not(),
        )
        .and_is(
            one_of(" \t")
                .repeated()
                .ignore_then(object::newline())
                .ignored()
                .not(),
        )
        .repeated()
        .at_least(1)
        .to_slice()
        .then(object::whitespaces())
        .or_not();
    let maybe_section_parser = section::section_parser(element_parser.clone()).or_not();
    heading_subtree.define(choice((stars
        .then_with_ctx(
            one_of(" \t")
                .repeated()
                .at_least(1)
                .to_slice()
                .then(maybe_keyword_ws)
                .then(maybe_priority)
                .then(maybe_comment)
                .then(maybe_title)
                .then(maybe_tag)
                .then(object::newline())
                .then(planning::planning_parser().or_not())
                .then(drawer::property_drawer_parser().or_not())
                .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
                .then(maybe_section_parser.clone())
                .then(heading_subtree.clone().repeated().collect::<Vec<_>>()),
        )
        .map(
            move |(
                stars,
                (
                    (
                        (
                            (
                                (
                                    (
                                        (
                                            (
                                                (
                                                    (
                                                        (whitespace1, maybe_keyword_ws),
                                                        maybe_priority,
                                                    ),
                                                    maybe_comment,
                                                ),
                                                maybe_title,
                                            ),
                                            maybe_tag,
                                        ),
                                        newline,
                                    ),
                                    maybe_planning,
                                ),
                                maybe_property_drawer,
                            ),
                            blanklines,
                        ),
                        maybe_section,
                    ),
                    subtrees,
                ),
            )| {
                let mut children = vec![];

                children.push(crate::token!(
                    OSK::HeadingRowStars,
                    stars // "*".repeat(stars).as_str() // "*".repeat(stars.prev_heading_level).as_str(),
                          // "*".repeat(e.ctx().prev_heading_level).as_str(),
                ));

                children.push(crate::token!(OSK::Whitespace, whitespace1));

                match maybe_keyword_ws {
                    Some((kw, ws)) if kw.to_uppercase() == "TODO" => {
                        children.push(crate::token!(OSK::HeadingRowKeywordTodo, kw));
                        children.push(crate::token!(OSK::Whitespace, &ws));
                    }
                    Some((kw, ws)) if kw.to_uppercase() == "DONE" => {
                        children.push(crate::token!(OSK::HeadingRowKeywordDone, kw));

                        children.push(crate::token!(OSK::Whitespace, &ws));
                    }

                    Some((kw, ws)) => {
                        children.push(crate::token!(OSK::HeadingRowKeywordOther, kw));

                        children.push(crate::token!(OSK::Whitespace, &ws));
                    }
                    None => {}
                }

                if let Some((((leftbracket_hash, level), rightbracket), whitespace)) =
                    maybe_priority
                {
                    let p_children = vec![
                        crate::token!(OSK::LeftSquareBracket, &leftbracket_hash[0..1]),
                        crate::token!(OSK::Hash, &leftbracket_hash[1..2]),
                        crate::token!(OSK::Text, &level.to_string()),
                        crate::token!(OSK::RightSquareBracket, rightbracket),
                    ];

                    let priority_node = crate::node!(OSK::HeadingRowPriority, p_children);

                    let ws_token = crate::token!(OSK::Whitespace, whitespace);

                    children.push(priority_node);
                    children.push(ws_token);
                }

                if let Some((comment, whitespace)) = maybe_comment {
                    children.push(crate::token!(OSK::HeadingRowComment, comment));

                    children.push(crate::token!(OSK::Whitespace, whitespace));
                }

                if let Some((title, whitespace)) = maybe_title {
                    // let title_token = crate::token!(OSK::HeadingRowTitle, title);
                    // children.push(title_token);

                    let title_node = object_parser
                        .clone()
                        .repeated()
                        .at_least(1)
                        .collect::<Vec<NT>>()
                        .map(|s| crate::node!(OSK::HeadingRowTitle, s))
                        .parse(title)
                        .into_output()
                        .unwrap();
                    children.push(title_node);

                    if !whitespace.is_empty() {
                        let ws_token = crate::token!(OSK::Whitespace, whitespace);
                        children.push(ws_token);
                    }
                }

                if let Some((((left_colon, tags), right_colon), whitespace)) = maybe_tag {
                    let mut tag_token_children: Vec<NT> = vec![];
                    tag_token_children.push(crate::token!(OSK::Colon, &left_colon.to_string()));

                    for tag in tags {
                        tag_token_children.push(crate::token!(OSK::HeadingRowTag, tag));

                        tag_token_children.push(crate::token!(OSK::Colon, right_colon));
                    }

                    let tag_node: NT = crate::node!(OSK::HeadingRowTags, tag_token_children);
                    children.push(tag_node);

                    if whitespace.len() > 0 {
                        children.push(crate::token!(OSK::Whitespace, whitespace));
                    }
                }

                children.push(crate::token!(OSK::Newline, newline));

                if let Some(planning) = maybe_planning {
                    children.push(planning);
                }

                if let Some(property_drawer) = maybe_property_drawer {
                    children.push(property_drawer);
                }

                children.extend(blanklines);
                let head_row = crate::node!(OSK::HeadingRow, children);

                let mut children = vec![];
                children.push(head_row);
                if let Some(section) = maybe_section {
                    children.push(section);
                }
                for subtree in subtrees {
                    children.push(subtree);
                }
                crate::node!(OSK::HeadingSubtree, children)
            },
        ),)));
    heading_subtree.with_ctx(prev_level).boxed()
}

/// A simple heading row parser WITHOUT state, ONLY used for look ahead
// - section parser: to check whether the next part is heading to stop
pub(crate) fn simple_heading_row_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone {
    let stars = just('*').repeated().at_least(1);
    let whitespaces = one_of(" \t").repeated().at_least(1);
    let title = none_of(object::CRLF).repeated();
    stars
        .ignore_then(whitespaces)
        .ignore_then(title)
        .ignore_then(object::newline_or_ending())
        .to_slice()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::{get_parser_output, get_parsers_output};
    use crate::parser::config::OrgParserConfig;
    use crate::parser::element::element_parser;
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_heading_subtree_01() {
        let input = "* 标题1\n 测试\n** 标题1.1\n测试\n测试\ntest \n*** 1.1.1 title\nContent\n";
        let parser = heading_subtree_parser(
            OrgParserConfig::default().org_todo_keywords,
            object::standard_set_object_parser::<()>(OrgParserConfig::default()),
            element_parser(OrgParserConfig::default()),
            "",
        );
        assert_eq!(
            get_parser_output(parser, input),
            r##"HeadingSubtree@0..75
  HeadingRow@0..10
    HeadingRowStars@0..1 "*"
    Whitespace@1..2 " "
    HeadingRowTitle@2..9
      Text@2..9 "标题1"
    Newline@9..10 "\n"
  Section@10..18
    Paragraph@10..18
      Text@10..18 " 测试\n"
  HeadingSubtree@18..75
    HeadingRow@18..31
      HeadingRowStars@18..20 "**"
      Whitespace@20..21 " "
      HeadingRowTitle@21..30
        Text@21..30 "标题1.1"
      Newline@30..31 "\n"
    Section@31..51
      Paragraph@31..51
        Text@31..51 "测试\n测试\ntest \n"
    HeadingSubtree@51..75
      HeadingRow@51..67
        HeadingRowStars@51..54 "***"
        Whitespace@54..55 " "
        HeadingRowTitle@55..66
          Text@55..66 "1.1.1 title"
        Newline@66..67 "\n"
      Section@67..75
        Paragraph@67..75
          Text@67..75 "Content\n"
"##
        );
    }

    #[test]
    fn test_heading_subtree_02() {
        let input = "* 标题1\n 测试\n** 标题1.1\n测试\n测试\ntest\n*** 1.1.1 title\nContent\n* Title\nI have a dream\n"; // overflow
        let parser = heading_subtree_parser(
            OrgParserConfig::default().org_todo_keywords,
            object::standard_set_object_parser::<()>(OrgParserConfig::default()),
            element_parser(OrgParserConfig::default()),
            "",
        )
        .repeated()
        .collect::<Vec<_>>();
        assert_eq!(
            get_parsers_output(parser, input),
            r##"Root@0..97
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
"##
        );
    }
}
