//! Heading parser, including HeadingRow, HeadingSubtree
use crate::parser::element::{drawer, planning};
use crate::parser::object;
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserResult, ParserState, S2};

use chumsky::input::InputRef;
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;

/// Imperative `HeadingRowStars` parser, implemented by `custom()` to parse stateful title level
//  - 仅解析stars, 不好含stars后的空格
/// - 标题嵌套, 标题有level, 如二级标题包含三级标题，有状态
/// - 关键部分用命令式解析，其他部分尽量用声明式解析
pub(crate) fn heading_row_stars_parser<'a>()
-> impl Parser<'a, &'a str, ParserResult, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>
+ Clone {
    custom(
        |inp: &mut InputRef<
            'a,
            '_,
            &'a str,
            extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
        >| {
            let binding = inp.cursor();
            let start = binding.inner();
            let state = inp.state().clone();

            let remaining = inp.slice_from(std::ops::RangeFrom {
                start: &inp.cursor(),
            });

            // println!("custom: remaining = {:?}", remaining);

            // 计算星号数量（标题级别）
            let mut level = 0;
            for c in remaining.chars() {
                if c == '*' {
                    level += 1;
                } else {
                    break;
                }
            }
            let state_level = state.level_stack.last().unwrap();
            if level == 0 || level <= *state_level {
                let error = Rich::custom::<&str>(
                    SimpleSpan::from(Range {
                        start: *inp.cursor().inner(),
                        end: (inp.cursor().inner() + level),
                    }),
                    &format!(
                        "标题级别应该在 1 到 {} 之间，但得到 {} 个星号",
                        state_level, level
                    ),
                );
                return Err(error);
            }

            let stars = &remaining[0..level];
            for _ in 0..level {
                inp.next();
            }

            let stars_token = NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::HeadingRowStars.into(),
                &stars,
            ));

            let span: SimpleSpan = SimpleSpan {
                start: *start,
                end: *inp.cursor().inner(),
                context: (),
            };

            Ok(ParserResult {
                green: stars_token,
                text: format!("{}", stars),
                span: Range {
                    start: span.start,
                    end: span.end,
                },
            })
        },
    )
}

/// HeadingRowTag parser, 解析标题行中的可选的tag
/// - 0: None, 没有Tag
/// - 1: TagNode
/// - 2: (TagNode, WhiteSpaceToken)
///
/// Note: 仅解析Tag部分，至于后续是否newline/end(), 不在此处判断，由HeadingRow统筹统一处理
pub fn heading_row_tag_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let tag = just(':')
        .then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '#' | '@' | '%'))
                .repeated()
                .at_least(1)
                .to_slice() // slice?
                .separated_by(just(':'))
                .collect::<Vec<_>>(),
        )
        .then(just(':'))
        .then(object::whitespaces())
        .or_not()
        .map(|s| {
            // println!("heading_row_tag_parser: s={:?}", s);
            match s {
                Some((((lc, tags), _rc), ws)) => {
                    let mut tag_token_children: Vec<NodeOrToken<GreenNode, GreenToken>> = vec![];
                    tag_token_children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Colon.into(),
                        &lc.to_string(),
                    )));

                    for tag in tags {
                        tag_token_children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::HeadingRowTag.into(),
                            tag,
                        )));

                        tag_token_children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Colon.into(),
                            ":",
                        )));
                    }

                    let tag_node: NodeOrToken<GreenNode, GreenToken> = NodeOrToken::Node(
                        GreenNode::new(OrgSyntaxKind::HeadingRowTags.into(), tag_token_children),
                    );

                    match ws.len() > 0 {
                        false => S2::Single(tag_node),
                        true => {
                            let ws_token = NodeOrToken::<GreenNode, GreenToken>::Token(
                                GreenToken::new(OrgSyntaxKind::Whitespace.into(), &ws),
                            );
                            S2::Double(tag_node, ws_token)
                        }
                    }
                }
                None => S2::None,
            }
        });
    tag
}

// FIXME: panic at corner case
// * asdf  :xx:yy:                                                      :da:
/// HeadingRowTitle parser, 解析标题行中的可选的Title
pub fn heading_row_title_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    // let newline_or_end = just("\n").map(Some).or(end().to(None));
    let tag_char =
        any().filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '#' | '@' | '%'));
    let title = none_of("\n")
        // FIXME: duplicate loginc of tag parser
        // and_is 放在哪里?.then(whitespaces)之后检查，有无问题?
        .and_is(
            one_of(" \t")
                .repeated()
                .at_least(1)
                .then(just(':'))
                .then(tag_char.repeated())
                .then(just(':'))
                .then(object::whitespaces())
                .not(),
        ) // 后面不是[space]?+标签
        .and_is(one_of(" \t").repeated().then(just("\n")).not()) // 后面不是[space]?+换行
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then(object::whitespaces())
        .or_not()
        .map(|s| {
            // println!("title_parser: s={:?}", s);
            match s {
                Some((title, ws)) => {
                    let title_token = NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::HeadingRowTitle.into(),
                        &title,
                    ));
                    match ws.len() > 0 {
                        true => {
                            let ws_token = NodeOrToken::<GreenNode, GreenToken>::Token(
                                GreenToken::new(OrgSyntaxKind::Whitespace.into(), &ws),
                            );
                            S2::Double(title_token, ws_token)
                        }
                        false => S2::Single(title_token),
                    }
                }
                None => S2::None,
            }
        });

    title
}

/// HeadingRowTitle parser, 解析标题行中的可选的Priority
pub fn heading_row_priority_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let priority = just("[#")
        .then(one_of('0'..'9').or(one_of('a'..'z').or(one_of('A'..'Z'))))
        .then(just(']'))
        .then(object::whitespaces_g1())
        .or_not()
        .map(|s: Option<(((&str, char), char), String)>| match s {
            Some((((_, level), _), ws)) => {
                let p_children = vec![
                    NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::LeftSquareBracket.into(),
                        "[",
                    )),
                    NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Hash.into(), "#")),
                    NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &level.to_string(),
                    )),
                    NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::RightSquareBracket.into(),
                        "]",
                    )),
                ];
                let priority_node = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::HeadingRowPriority.into(),
                    p_children,
                ));

                let ws_token = NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                ));

                match ws.len() > 0 {
                    false => S2::Single(priority_node),
                    true => S2::Double(priority_node, ws_token),
                }
            }
            None => S2::None,
        });

    priority
}

/// HeadingRow parser, 解析标题行`STARS KEYWORD PRIORITY COMMENT TITLE TAGS`
pub(crate) fn heading_row_parser<'a>()
-> impl Parser<'a, &'a str, ParserResult, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>
+ Clone {
    let keyword_ws = just("TODO")
        .or(just("DONE"))
        .then(object::whitespaces_g1())
        .or_not()
        .map(|s| match s {
            Some((kw, ws)) if kw.to_uppercase() == "TODO" => Some((
                NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::HeadingRowKeywordTodo.into(),
                    kw,
                )),
                NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )),
            )),

            Some((kw, ws)) if kw.to_uppercase() == "DONE" => Some((
                NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::HeadingRowKeywordDone.into(),
                    kw,
                )),
                NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )),
            )),

            Some((kw, ws)) => Some((
                NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::HeadingRowKeywordOther.into(),
                    kw,
                )),
                NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )),
            )),

            None => None,
        });

    let comment_token = "COMMENT";
    let comment = just(comment_token)
        .then(object::whitespaces_g1())
        .or_not()
        .map(|s| match s {
            Some((cmt, ws)) => Some((
                NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::HeadingRowComment.into(),
                    cmt,
                )),
                NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )),
            )),
            None => None,
        });

    heading_row_stars_parser()
        .then(
            object::whitespaces_g1()
                .then(keyword_ws)
                .then(heading_row_priority_parser())
                .then(comment)
                .then(heading_row_title_parser())
                .then(heading_row_tag_parser())
                .then(
                    just('\n')
                        .then(planning::planning_parser().or_not())
                        .then(drawer::property_drawer_parser().or_not())
                        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
                        .map(
                            |(((nl, maybe_planning), maybe_property_drawer), maybe_blanklines)| {
                                let mut children = vec![];
                                children.push(NodeOrToken::Token(GreenToken::new(
                                    OrgSyntaxKind::Newline.into(),
                                    &nl.to_string(),
                                )));

                                if let Some(planning) = maybe_planning {
                                    children.push(planning);
                                }

                                if let Some(property_drawer) = maybe_property_drawer {
                                    children.push(property_drawer);
                                }

                                for blankline_token in maybe_blanklines {
                                    children.push(NodeOrToken::Token(blankline_token))
                                }

                                Some(children)
                            },
                        )
                        .or(end().to(None)),
                ),
        )
        .map_with(
            |(
                stars_token_result,
                ((((((ws, kw_ws), priority_ws), comment_ws), title_ws), tag_ws), nl_blank_tokens),
            ),
             e| {
                e.state().prev_char = Some('\n'); // fixme: hard coded!

                let span: SimpleSpan = e.span();
                let mut children = vec![stars_token_result.green];
                let mut text = String::new();
                text.push_str(&stars_token_result.text);

                if ws.len() > 0 {
                    let ws_token =
                        NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Whitespace.into(), &ws));
                    children.push(ws_token);
                    text.push_str(&ws);
                }

                // println!("kw_ws: {:?}", kw_ws);
                match kw_ws {
                    Some((kw_token, ws_token)) => {
                        // FIXME: to check
                        text.push_str(&kw_token.to_string());
                        text.push_str(&ws_token.to_string());
                        children.push(kw_token);
                        children.push(ws_token);
                    }
                    None => {}
                }
                // println!("priority_ws: {:?}", priority_ws);
                match priority_ws {
                    S2::Double(priority_node, ws_token) => {
                        text.push_str(&priority_node.to_string());
                        text.push_str(&ws_token.to_string());

                        children.push(priority_node);
                        children.push(ws_token);
                    }
                    S2::Single(priority_node) => {
                        text.push_str(&priority_node.to_string());
                        children.push(priority_node);
                    }

                    S2::None => {}
                }

                // println!("comment_ws: {:?}", comment_ws);

                match comment_ws {
                    Some((comment_token, ws_token)) => {
                        text.push_str(&comment_token.to_string());
                        text.push_str(&ws_token.to_string());
                        children.push(comment_token);
                        children.push(ws_token);
                    }
                    None => {}
                }

                // println!("title_ws: {:?}", title_ws);
                match title_ws {
                    S2::Double(title_token, ws_token) => {
                        text.push_str(&title_token.to_string());
                        text.push_str(&ws_token.to_string());
                        children.push(title_token);
                        children.push(ws_token);
                    }
                    S2::Single(title_token) => {
                        text.push_str(&title_token.to_string());
                        children.push(title_token);
                    }
                    S2::None => {}
                }

                match tag_ws {
                    S2::Double(tag_node, ws_token) => {
                        text.push_str(&tag_node.to_string());
                        text.push_str(&ws_token.to_string());
                        children.push(tag_node);
                        children.push(ws_token);
                    }
                    S2::Single(tag_node) => {
                        text.push_str(&tag_node.to_string());
                        children.push(tag_node);
                    }
                    S2::None => {}
                }

                match nl_blank_tokens {
                    None => {}
                    Some(maybe_nl_or_blank_tokens) => {
                        for e in maybe_nl_or_blank_tokens {
                            text.push_str(&e.to_string());
                            children.push(e)
                        }
                    }
                }

                let level = stars_token_result.text.chars().count();

                // 仅当构造HeadingRow成功时，更新state
                e.state().level_stack.push(level);

                ParserResult {
                    green: NodeOrToken::Node(GreenNode::new(
                        OrgSyntaxKind::HeadingRow.into(),
                        children,
                    )),
                    text: format!("{}", text),
                    span: Range {
                        start: span.start,
                        end: span.end,
                    },
                }
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::{get_parser_output, get_parsers_output};
    use crate::parser::element::element_in_section_parser;
    use crate::parser::{element::section::section_parser, syntax::OrgLanguage};
    use pretty_assertions::assert_eq;
    use rowan::SyntaxNode;

    #[test]
    fn test_heading_subtree_01() {
        // let input = "* 标题1\n 测试\n** 标题1.1\n测试\n测试\ntest\n*** 1.1.1 title\nContent\n* Title\nI have a dream\n";
        let input = "* 标题1\n 测试\n** 标题1.1\n测试\n测试\ntest \n*** 1.1.1 title\nContent\n";
        // let parser = heading_subtree_parser(section_parser(element_in_section_parser()));
        let parser = heading_subtree_parser();
        assert_eq!(
            get_parser_output(parser, input),
            r##"HeadingSubtree@0..75
  HeadingRow@0..10
    HeadingRowStars@0..1 "*"
    Whitespace@1..2 " "
    HeadingRowTitle@2..9 "标题1"
    Newline@9..10 "\n"
  Section@10..18
    Paragraph@10..18
      Text@10..18 " 测试\n"
  HeadingSubtree@18..75
    HeadingRow@18..31
      HeadingRowStars@18..20 "**"
      Whitespace@20..21 " "
      HeadingRowTitle@21..30 "标题1.1"
      Newline@30..31 "\n"
    Section@31..51
      Paragraph@31..51
        Text@31..51 "测试\n测试\ntest \n"
    HeadingSubtree@51..75
      HeadingRow@51..67
        HeadingRowStars@51..54 "***"
        Whitespace@54..55 " "
        HeadingRowTitle@55..66 "1.1.1 title"
        Newline@66..67 "\n"
      Section@67..75
        Paragraph@67..75
          Text@67..75 "Content\n"
"##
        );
    }

    #[test]
    fn test_heading_subtree_02() {
        // let input = "* 1 \n** 1.1\n*** 1.1.1\n* Title"; // panic
        // let input = "* 1 \n** 1.1\n*** 1.1.1\n* 2\n"; // overflow
        let input = "* 标题1\n 测试\n** 标题1.1\n测试\n测试\ntest\n*** 1.1.1 title\nContent\n* Title\nI have a dream\n"; // overflow
        // let parser = heading_subtree_parser(section_parser(element_in_section_parser())).repeated().collect::<Vec<_>>();
        let parser = heading_subtree_parser().repeated().collect::<Vec<_>>();
        assert_eq!(
            get_parsers_output(parser, input),
            r##"Root@0..97
  HeadingSubtree@0..74
    HeadingRow@0..10
      HeadingRowStars@0..1 "*"
      Whitespace@1..2 " "
      HeadingRowTitle@2..9 "标题1"
      Newline@9..10 "\n"
    Section@10..18
      Paragraph@10..18
        Text@10..18 " 测试\n"
    HeadingSubtree@18..74
      HeadingRow@18..31
        HeadingRowStars@18..20 "**"
        Whitespace@20..21 " "
        HeadingRowTitle@21..30 "标题1.1"
        Newline@30..31 "\n"
      Section@31..50
        Paragraph@31..50
          Text@31..50 "测试\n测试\ntest\n"
      HeadingSubtree@50..74
        HeadingRow@50..66
          HeadingRowStars@50..53 "***"
          Whitespace@53..54 " "
          HeadingRowTitle@54..65 "1.1.1 title"
          Newline@65..66 "\n"
        Section@66..74
          Paragraph@66..74
            Text@66..74 "Content\n"
  HeadingSubtree@74..97
    HeadingRow@74..82
      HeadingRowStars@74..75 "*"
      Whitespace@75..76 " "
      HeadingRowTitle@76..81 "Title"
      Newline@81..82 "\n"
    Section@82..97
      Paragraph@82..97
        Text@82..97 "I have a dream\n"
"##
        );
    }

    #[test]
    fn test_heading_row_tag_3() {
        let input = ":taga:tag#:  ";
        let parser = heading_row_tag_parser();
        let ans: S2 = parser
            .parse_with_state(input, &mut RollbackState(ParserState::default()))
            .into_result()
            .unwrap();

        assert_eq!(matches!(ans, S2::Double(_, _)), true);

        match ans {
            S2::Single(NodeOrToken::Node(g)) => {
                panic!("error");
                // let syntax_node: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(g);
                // println!("{:#?}", syntax_node);
            }
            S2::Double(NodeOrToken::Node(g), NodeOrToken::Token(t)) => {
                let syntax_node: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(g);
                println!("syntax_node={:#?}\nleaf={:#?}", syntax_node, t);
                assert_eq!(
                    format!("{:#?}", syntax_node),
                    r##"HeadingRowTags@0..11
  Colon@0..1 ":"
  HeadingRowTag@1..5 "taga"
  Colon@5..6 ":"
  HeadingRowTag@6..10 "tag#"
  Colon@10..11 ":"
"##
                );
            }
            _ => {}
        }
    }

    #[test]
    fn test_heading_row_tag() {
        let input = ":taga:tag#:  ";
        let parser = heading_row_tag_parser();
        let ans: S2 = parser
            .parse_with_state(input, &mut RollbackState(ParserState::default()))
            .into_result()
            .unwrap();

        assert_eq!(matches!(ans, S2::Double(_, _)), true);

        match ans {
            S2::Single(NodeOrToken::Node(g)) => {
                panic!("error");
                // let syntax_node: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(g);
                // println!("{:#?}", syntax_node);
            }
            S2::Double(NodeOrToken::Node(g), NodeOrToken::Token(t)) => {
                let syntax_node: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(g);
                println!("syntax_node={:#?}\nleaf={:#?}", syntax_node, t);
                assert_eq!(
                    format!("{:#?}", syntax_node),
                    r##"HeadingRowTags@0..11
  Colon@0..1 ":"
  HeadingRowTag@1..5 "taga"
  Colon@5..6 ":"
  HeadingRowTag@6..10 "tag#"
  Colon@10..11 ":"
"##
                );
            }
            _ => {}
        }
    }

    #[test]
    fn test_heading_row_tag_2() {
        let input = "taga:tag#";
        let parser = heading_row_tag_parser();
        assert_eq!(
            parser
                .parse_with_state(input, &mut RollbackState(ParserState::default()))
                .has_errors(),
            true
        )
    }

    #[test]
    fn test_heading_row() {
        let input = "** TODO [#A]  Title :taga:tag#:   ";

        let parser = heading_row_parser();

        let syntax_node = parser
            .parse_with_state(input, &mut RollbackState(ParserState::default()))
            .into_result()
            .unwrap()
            .syntax();
        println!("{}", format!("{syntax_node:#?}"));
        assert_eq!(
            format!("{:#?}", syntax_node),
            r##"HeadingRow@0..34
  HeadingRowStars@0..2 "**"
  Whitespace@2..3 " "
  HeadingRowKeywordTodo@3..7 "TODO"
  Whitespace@7..8 " "
  HeadingRowPriority@8..12
    LeftSquareBracket@8..9 "["
    Hash@9..10 "#"
    Text@10..11 "A"
    RightSquareBracket@11..12 "]"
  Whitespace@12..14 "  "
  HeadingRowTitle@14..19 "Title"
  Whitespace@19..20 " "
  HeadingRowTags@20..31
    Colon@20..21 ":"
    HeadingRowTag@21..25 "taga"
    Colon@25..26 ":"
    HeadingRowTag@26..30 "tag#"
    Colon@30..31 ":"
  Whitespace@31..34 "   "
"##
        );
    }
}
