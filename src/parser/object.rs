//! Object paser (todo)

use crate::parser::ParserResult;
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::markup::text_markup_parser;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::input::InputRef;
use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;
// use chumsky::input::InputRef;

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

/// Text Parser
pub(crate) fn text_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    any()
        .and_is(text_markup_parser().not())
        .and_is(link_parser().not())
        .and_is(footnote_reference_parser().not())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .map(|s| {
            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Token(
                GreenToken::new(OrgSyntaxKind::Text.into(), &s),
            ))
        })
}

/// Link parser
pub(crate) fn link_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    just("[")
        .then(
            just("[")
                .then(none_of("]").repeated().collect::<String>())
                .then(just("]"))
                .map(|((lbracket, path), rbracket)| {
                    let mut children = vec![];
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::LeftBracket.into(),
                        lbracket,
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &path,
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::RightBracket.into(),
                        rbracket,
                    )));
                    NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                        OrgSyntaxKind::LinkPath.into(),
                        children,
                    ))
                }),
        )
        .then(
            just("[")
                .then(none_of("]").repeated().collect::<String>())
                .then(just("]"))
                .or_not()
                .map(|description| match description {
                    None => None,

                    Some(((lbracket, content), rbracket)) => {
                        let mut children = vec![];
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::LeftBracket.into(),
                            lbracket,
                        )));

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Text.into(),
                            &content,
                        )));

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::RightBracket.into(),
                            rbracket,
                        )));

                        Some(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                            OrgSyntaxKind::LinkDescription.into(),
                            children,
                        )))
                    }
                }),
        )
        .then(just("]"))
        .map(|(((lbracket, path), maybe_desc), rbracket)| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftBracket.into(),
                lbracket,
            )));

            children.push(path);

            match maybe_desc {
                None => {}
                Some(desc) => {
                    children.push(desc);
                }
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightBracket.into(),
                rbracket,
            )));

            let link = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Link.into(),
                children,
            ));

            S2::Single(link)
        })
}

/// Footntoe refrence
// fixme: only one pattern suppoted
// - [fn:LABEL] done
// - [fn:LABEL:DEFINITION] todo
// - [fn::DEFINITION] todo

pub(crate) fn footnote_reference_parser<'a>()
                                            -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    just("[fn:")
        .then(                  // label
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric() || matches!(c, '_'|'-'))
                .repeated()
                .at_least(1)
                .collect::<String>()
        )
        .then(just("]"))
        .map(|((left_fn_c, label), rbracket)| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftBracket.into(),
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
                OrgSyntaxKind::RightBracket.into(),
                rbracket,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteReference.into(),
                children,
            )))
        })
        
}
    
pub(crate) fn object_parser<'a>()
-> impl Parser<'a, &'a str, Vec<S2>, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    choice((
        text_markup_parser(),
        entity_parser(),
        link_parser(),
        footnote_reference_parser(),
        text_parser(),
    ))
    .repeated()
    .at_least(1)
    .collect::<Vec<_>>()
}

/// Entity parser
///
/// fixme: candidate symbols
pub(crate) fn entity_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    let a1 = just(r"\")
        .then(
            any()
                .filter(|c: &char| !matches!(c, '\r' | '\n' | ' ' | '\t' | '{' | '}' | '_' | '['))
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .and_is(any().filter(|c: &char| !matches!(c, 'a'..'z'| 'A'..'Z'|'\n'|'\r')))
        .map(|(backslash, name)| {
            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::BackSlash.into(),
                backslash,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::EntityName.into(),
                &name,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::Entity.into(),
                children,
            )))
        });

    let a2 = just(r"\")
        .then(
            any()
                .filter(|c: &char| !matches!(c, '\r' | '\n' | ' ' | '\t' | '{' | '}' | '_'))
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .then(just("{}"))
        .map(|((backslash, name), left_right_curly)| {
            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::BackSlash.into(),
                backslash,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::EntityName.into(),
                &name,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftCurlyBracket.into(),
                &left_right_curly[0..1],
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightCurlyBracket.into(),
                &left_right_curly[1..2],
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::Entity.into(),
                children,
            )))
        });
    let a3 = just(r"\")
        .then(just("_"))
        .then(one_of(" ").repeated().at_least(1).collect::<String>())
        .map(|((backslash, us), ws)| {
            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::BackSlash.into(),
                backslash,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::UnderScore.into(),
                us,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::EntityName.into(),
                &ws,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::Entity.into(),
                children,
            )))
        });

    a2.or(a1).or(a3)
}

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
    fn test_link() {
        let input = "[[https://www.baidu.com][baidu]]";

        let parser = link_parser();

        match parser.parse(input).unwrap() {
            S2::Single(node) => {
                let syntax_tree: SyntaxNode<OrgLanguage> =
                    SyntaxNode::new_root(node.into_node().expect("xxx"));
                println!("{:#?}", syntax_tree);

                assert_eq!(
                    format!("{syntax_tree:#?}"),
                    r###"Link@0..32
  LeftBracket@0..1 "["
  LinkPath@1..24
    LeftBracket@1..2 "["
    Text@2..23 "https://www.baidu.com"
    RightBracket@23..24 "]"
  LinkDescription@24..31
    LeftBracket@24..25 "["
    Text@25..30 "baidu"
    RightBracket@30..31 "]"
  RightBracket@31..32 "]"
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
