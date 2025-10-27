//! Object paser (todo)

mod angle_link;
pub mod entity;
mod latex_fragment;
mod r#macro;
mod regular_link;
mod subscript_superscript;
mod target;
mod timestamp;

use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::markup::text_markup_parser;
use crate::parser::object::angle_link::angle_link_parser;
use crate::parser::object::entity::entity_parser;
use crate::parser::object::regular_link::regular_link_parser;
use crate::parser::object::target::target_parser;
use crate::parser::object::timestamp::timestamp_parser;

use crate::parser::object::latex_fragment::latex_fragment_parser;
use crate::parser::object::r#macro::macro_parser;
use crate::parser::object::subscript_superscript::superscript_parser;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::input::MapExtra;
use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;

/// 解析行终止符：换行符或输入结束
pub(crate) fn newline_or_ending<'a>()
-> impl Parser<'a, &'a str, Option<String>, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>>
+ Clone {
    just('\n').map(|c| Some(String::from(c))).or(end().to(None))
}

/// 创建一个不区分大小写的关键字解析器
pub(crate) fn just_case_insensitive<'a>(
    s: &'a str,
) -> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    let s_lower = s.to_lowercase();

    any()
        .filter(|c: &char| c.is_ascii())
        .repeated()
        .exactly(s.chars().count())
        .collect::<String>()
        .try_map_with(move |t, e| {
            if t.to_lowercase() == s_lower {
                Ok(t)
            } else {
                Err(Rich::custom(
                    e.span(),
                    format!("Expected '{}' (case-insensitive)", t),
                ))
            }
        })
}

#[allow(dead_code)]
pub(crate) fn is_ending<'a>()
-> impl Parser<'a, &'a str, Option<String>, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>>
+ Clone {
    any()
        .repeated()
        .then(just('\n').map(|c| Some(String::from(c))).or(end().to(None)))
        .map(|_| Some("OK".to_string()))
}

/// 解析零个或多个空白字符（包括空格、制表符等）
pub(crate) fn whitespaces<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    one_of(" \t").repeated().collect::<String>()
}
/// 解析一个或多个空白字符（包括空格、制表符等）
pub(crate) fn whitespaces_g1<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    one_of(" \t").repeated().at_least(1).collect::<String>()
}

/// 解析一行:
/// Line <- (!EOL .)+
/// EOL <- '\r'? '\n'
pub(crate) fn line_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    let end_of_line = one_of("\r")
        .repeated()
        .at_most(1)
        .collect::<String>()
        .then(just("\n"))
        .map(|(s, n)| {
            let mut ans = String::from(s);
            ans.push_str(n);
            ans
        });

    any()
        .and_is(end_of_line.not())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then(end_of_line)
        .map(|(line, eol)| {
            let mut ans = String::from(line);
            ans.push_str(&eol);
            ans
        })
}

pub(crate) fn blank_line_str_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    whitespaces()
        .then(one_of("\r").repeated().at_most(1).collect::<String>())
        .then(just("\n"))
        .map(|((ws, cr), nl)| {
            let mut text = String::new();

            if ws.len() > 0 {
                text.push_str(&ws);
            }

            if cr.len() > 0 {
                text.push_str(&cr);
            }

            text.push_str(nl);

            text
        })
}

/// Blank Line Parser := 空白字符后紧跟行终止符, PEG定义如下
/// ```text
/// BlankLine <- WS* EOL
/// WS <- [ \t]
/// EOL <- '\r'? '\n'
/// ```
pub(crate) fn blank_line_parser<'a>()
-> impl Parser<'a, &'a str, GreenToken, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>>
+ Clone {
    whitespaces()
        .then(one_of("\r").repeated().at_most(1).collect::<String>())
        .then(just("\n"))
        .map(|((ws, cr), nl)| {
            let mut text = String::new();

            if ws.len() > 0 {
                text.push_str(&ws);
            }

            if cr.len() > 0 {
                text.push_str(&cr);
            }

            text.push_str(nl);

            GreenToken::new(OrgSyntaxKind::BlankLine.into(), &text)
        })
}

// ---------------------------------------------------------------------
/// Line break parser
pub(crate) fn line_break_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    // PRE\\SPACE
    just(r##"\\"##)
        .then(whitespaces())
        .then_ignore(
            one_of("\r")
                .repeated()
                .at_most(1)
                .collect::<String>()
                .then(just("\n"))
                .rewind(),
        )
        .try_map_with(|(line_break, maybe_ws), e| {
            if let Some('\\') = e.state().prev_char {
                let error = Rich::custom::<&str>(
                    SimpleSpan::from(Range {
                        start: e.span().start(),
                        end: e.span().end(),
                    }),
                    &format!("PRE is \\ not mathced, NOT line break"),
                );
                Err(error)
            } else {
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::BackSlash2.into(),
                    line_break,
                )));

                if maybe_ws.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &maybe_ws,
                    )));
                    e.state().prev_char = maybe_ws.chars().last();
                } else {
                    e.state().prev_char = line_break.chars().last();
                }

                Ok(S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(
                    GreenNode::new(OrgSyntaxKind::LineBreak.into(), children),
                )))
            }
        })
}

/// Text Parser
pub(crate) fn text_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    any()
        .and_is(text_markup_parser().not())
        .and_is(entity_parser().not())
        .and_is(regular_link_parser().not())
        .and_is(angle_link_parser().not())
        .and_is(latex_fragment_parser().not())
        .and_is(footnote_reference_parser().not())
        .and_is(line_break_parser().not())
        .and_is(macro_parser().not())
        .and_is(superscript_parser().not())
        .and_is(target_parser().not())
        .and_is(timestamp_parser().not())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map_with(|s, e| {
            // let z: &mut MapExtra<'_, '_, &str, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>> = e;
            if let Some(c) = s.chars().last() {
                e.state().prev_char = Some(c);
            }

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Token(
                GreenToken::new(OrgSyntaxKind::Text.into(), &s),
            ))
        })
}

/// Footntoe refrence
// - [fn:LABEL] done
// - [fn:LABEL:DEFINITION] todo
// - [fn::DEFINITION] todo
pub(crate) fn footnote_reference_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    let label = any()
        .filter(|c: &char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        .repeated()
        .at_least(1)
        .collect::<String>();

    // let definition = object_parser(); // make object_parser: recursive
    // FIXME: simplified version
    let definition = any().and_is(just("]").not()).repeated().collect::<String>();

    // [fn:LABEL:DEFINITION]
    let t2 = just("[fn:")
        .then(label)
        .then(just(":"))
        .then(definition)
        .then(just("]"))
        .map_with(
            |((((_left_fn_c, label), colon), definition), rbracket),
             e: &mut MapExtra<
                '_,
                '_,
                &str,
                extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>,
            >| {
                e.state().prev_char = rbracket.chars().last();
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftSquareBracket.into(),
                    "[",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    "fn",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    colon,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &label,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    ":",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &definition,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightSquareBracket.into(),
                    rbracket,
                )));

                S2::Single(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::FootnoteReference.into(),
                    children,
                )))
            },
        );

    // [fn::DEFINITION]
    let t3 = just("[fn::").then(definition).then(just("]")).map_with(
        |((_left_fn_c_c, definition), rbracket), e| {
            e.state().prev_char = rbracket.chars().last();
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                "[",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                "fn",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon2.into(),
                "::",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &definition,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteReference.into(),
                children,
            )))
        },
    );

    // [fn:LABEL]
    let t1 =
        just("[fn:")
            .then(label)
            .then(just("]"))
            .map_with(|((_left_fn_c, label), rbracket), e| {
                e.state().prev_char = rbracket.chars().last();
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftSquareBracket.into(),
                    "[",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    "fn",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    ":",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &label,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightSquareBracket.into(),
                    rbracket,
                )));

                S2::Single(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::FootnoteReference.into(),
                    children,
                )))
            });

    // t1
    t1.or(t2).or(t3)
}

// objects_parser
// todo: select! use prev_char state?
pub(crate) fn object_parser<'a>()
-> impl Parser<'a, &'a str, Vec<S2>, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    choice((
        text_markup_parser(),
        entity_parser(),
        regular_link_parser(),
        angle_link_parser(),
        footnote_reference_parser(),
        latex_fragment_parser(),
        line_break_parser(),
        macro_parser(),
        superscript_parser(),
        target_parser(),
        timestamp_parser(),
        text_parser(),
    ))
    .repeated()
    .at_least(1)
    .collect::<Vec<_>>()
}

#[allow(unused)]
fn is_all_whitespace(s: String) -> bool {
    for c in s.chars() {
        if !matches!(c, '\t' | ' ' | '​') {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::syntax::OrgLanguage;
    use rowan::SyntaxNode;

    #[test]
    fn test_is_ending() {
        let mut state = SimpleState(ParserState::default());
        for input in vec![" \n", "\t\n", "\n", " \t   \n", " ", "\t", "abc"] {
            assert_eq!(
                is_ending()
                    .parse_with_state(input, &mut state)
                    .into_result(),
                Ok(Some("OK".to_string()))
            );
        }
    }

    #[test]
    fn test_blank_line() {
        let mut state = SimpleState(ParserState::default());
        for input in vec![" \n", "\t\n", "\n", " \t   \n"] {
            assert_eq!(
                blank_line_parser()
                    .parse_with_state(input, &mut state)
                    .into_result(),
                Ok(GreenToken::new(OrgSyntaxKind::BlankLine.into(), input))
            );
        }

        for input in vec![" \n "] {
            assert_eq!(
                blank_line_parser()
                    .parse_with_state(input, &mut state)
                    .has_errors(),
                true
            );
        }
    }
    #[test]
    fn test_line() {
        let mut state = SimpleState(ParserState::default());
        let input = "a row\n";
        let s = line_parser().parse_with_state(input, &mut state);
        println!("{:?}", s);
    }

    #[test]
    fn test_line_lookahead() {
        let mut state = SimpleState(ParserState::default());
        let input = r##"L1
L2
L3

"##;

        // How to debug?
        // x.repeated().collect().then_ignore(y.rewind().not()) BAD
        // x.then_ignore(y.rewind().not()).repeated().collect() OK
        // L1 L2 L3 BL
        let parser = line_parser()
            .then_ignore(blank_line_parser().rewind().not())
            .repeated()
            .collect::<Vec<String>>()
            .then(line_parser())
            .then(blank_line_parser())
            .then(end())
            .map(|s| {
                // println!("s={:?}", s);
                Some(1u32)
            });

        // collect()后似乎不能回退!!
        let parser_bad = line_parser()
            .repeated()
            .collect::<Vec<String>>()
            .then_ignore(blank_line_parser().rewind().not())
            .then(any().repeated())
            .then(end())
            .map(|s| {
                // println!("s={:?}", s);
                Some(1u32)
            });

        // println!("input={:?}", input);
        // let s = parser.lazy().parse_with_state(input, & mut state);
        // println!("{:?}, has_output={:?}, has_errors={:?}", s, s.has_output(), s.has_errors());

        println!("input={:?}", input);
        let s = parser_bad.lazy().parse_with_state(input, &mut state);
        println!(
            "{:?}, has_output={:?}, has_errors={:?}",
            s,
            s.has_output(),
            s.has_errors()
        );
    }

    #[test]
    fn test_correct_entity() {
        let input = vec![
            // pattern1
            "\\alpha ",
            "\\alpha\n",
            // pattern2
            "\\alpha{}",
            // pattern3
            "\\_ \n",
            "\\_  \n",
            "\\_                       \n",
        ];
        let parser = object_parser();
        for e in input {
            let s = parser.parse(e);
            let s1 = s.output().unwrap().iter().next();

            match s1 {
                Some(S2::Single(node)) => {
                    let kind = node.kind();
                    assert_eq!(kind, OrgSyntaxKind::Entity.into());
                }
                _ => {}
            }

            println!(
                "{:?}, has_output={:?}, has_errors={:?}",
                s,
                s.has_output(),
                s.has_errors()
            );
        }
    }

    #[test]
    fn test_incorrect_entity() {
        let input = vec!["\\alphA ", "\\deltab "];
        let parser = object_parser();
        for e in input {
            let s = parser.parse(e);
            let s1 = s.output().unwrap().iter().next();

            match s1 {
                Some(S2::Single(node)) => {
                    let kind = node.kind();
                    assert_ne!(kind, OrgSyntaxKind::Entity.into());
                }
                _ => {}
            }

            // println!(
            //     "{:?}, has_output={:?}, has_errors={:?}",
            //     s,
            //     s.has_output(),
            //     s.has_errors()
            // );
        }
    }

    #[test]
    fn test_link() {
        let input = "[[https://www.baidu.com][baidu]]";

        let parser = regular_link_parser();

        match parser.parse(input).unwrap() {
            S2::Single(node) => {
                let syntax_tree: SyntaxNode<OrgLanguage> =
                    SyntaxNode::new_root(node.into_node().expect("xxx"));
                println!("{:#?}", syntax_tree);

                assert_eq!(
                    format!("{syntax_tree:#?}"),
                    r###"Link@0..32
  LeftSquareBracket@0..1 "["
  LinkPath@1..24
    LeftSquareBracket@1..2 "["
    Text@2..23 "https://www.baidu.com"
    RightSquareBracket@23..24 "]"
  LinkDescription@24..31
    LeftSquareBracket@24..25 "["
    Text@25..30 "baidu"
    RightSquareBracket@30..31 "]"
  RightSquareBracket@31..32 "]"
"###
                );
            }

            _ => {}
        };
    }

    #[test]
    fn test_object() {
        // let input = "[[https://www.baidu.com][baidu]]";
        let input = "foo [[https://www.baidu.com][baidu]]";

        for e in object_parser().parse(input).unwrap() {
            match e {
                S2::Single(node_or_token) => {
                    match node_or_token {
                        NodeOrToken::Node(node) => {
                            let syntax_tree: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(node);
                            println!("{:#?}", syntax_tree);
                        }

                        NodeOrToken::Token(token) => {
                            println!("{:#?}", token);
                        }

                        _ => {}
                    }
                    // println!("{:?}", node);
                    // let syntax_tree: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(node.into_node().expect("xxx"));
                    // println!("{:#?}", syntax_tree);
                }
                _ => {}
            };
        }
    }
}

// block_parser
//   source_block_parser
//   center_block_parser
//   quote_block_parser
// drawer_parser
// dynmic_block_parser
// footnote_definition_parser
// inline_task?
// list_parser
//   items?
//   plain_list_parser: recusive?
// table_parser

// whitespace_config?
