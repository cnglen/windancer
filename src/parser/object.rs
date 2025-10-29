//! Object paser (todo)

mod angle_link;
pub mod entity;
mod footnote_reference;
mod latex_fragment;
mod r#macro;
mod regular_link;
mod subscript_superscript;
mod target;
mod text;
mod text_markup;
mod timestamp;

use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::object::angle_link::angle_link_parser;
use crate::parser::object::entity::entity_parser;
use crate::parser::object::footnote_reference::footnote_reference_parser;
use crate::parser::object::latex_fragment::latex_fragment_parser;
use crate::parser::object::r#macro::macro_parser;
use crate::parser::object::regular_link::regular_link_parser;
use crate::parser::object::subscript_superscript::subscript_superscript_parser;
use crate::parser::object::target::target_parser;
// use crate::parser::object::text::text_parser;
use crate::parser::object::text::plain_text_parser;
use crate::parser::object::text_markup::text_markup_parser;
use crate::parser::object::timestamp::timestamp_parser;
use crate::parser::syntax::OrgSyntaxKind;

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

// // objects_parser
// // todo: select! use prev_char state?
// pub(crate) fn objects_parser_v1<'a>()
// -> impl Parser<'a, &'a str, Vec<S2>, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
// {
//     choice((
//         text_markup_parser(),
//         entity_parser(),
//         regular_link_parser(),
//         angle_link_parser(),
//         footnote_reference_parser(),
//         latex_fragment_parser(),
//         line_break_parser(),
//         macro_parser(),
//         subscript_superscript_parser(),
//         target_parser(),
//         timestamp_parser(),
//         text_parser(),
//     ))
//     .repeated()
//     .at_least(1)
//     .collect::<Vec<_>>()
// }

// recursive version
pub(crate) fn objects_parser<'a>()
-> impl Parser<'a, &'a str, Vec<S2>, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    object_parser().repeated().at_least(1).collect::<Vec<_>>()
}

// // define minimal/standard/all?
// pub(crate) fn object_parser<'a>()
// -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
//     recursive(|object_parser| {
//         let entity_parser = entity_parser();
//         let text_markup_parser = text_markup_parser();
//         let latex_fragment_parsre = latex_fragment_parser();
//         let subscript_superscript_parser = subscript_superscript_parser(object_parser.clone());
//         let footnote_reference_parser = footnote_reference_parser(object_parser.clone());

//         let non_plain_text_parsers = choice((
//             entity_parser.clone(),
//             text_markup_parser,
//             latex_fragment_parsre,
//             subscript_superscript_parser,
//             footnote_reference_parser,
//         ));

//         let plain_text_parser = plain_text_parser(non_plain_text_parsers.clone());
//         // let text_parser = text_parser();

//         choice((non_plain_text_parsers, plain_text_parser))
//     })
// }

pub(crate) fn minimal_object_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    recursive(|object_parser| {
        let entity_parser = entity_parser();
        let latex_fragment_parsre = latex_fragment_parser();
        let subscript_superscript_parser = subscript_superscript_parser(object_parser.clone());
        let text_markup_parser = text_markup_parser();
        let non_plain_text_parsers = choice((
            entity_parser.clone(),
            text_markup_parser,
            latex_fragment_parsre,
            subscript_superscript_parser,
        ));
        let plain_text_parser = plain_text_parser(non_plain_text_parsers.clone());

        choice((non_plain_text_parsers, plain_text_parser))
    })
}

pub(crate) fn standard_object_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    recursive(|object_parser| {
        let entity_parser = entity_parser();
        let text_markup_parser = text_markup_parser();
        let latex_fragment_parsre = latex_fragment_parser();
        let subscript_superscript_parser = subscript_superscript_parser(object_parser.clone());
        let non_plain_text_parsers = choice((
            entity_parser.clone(),
            text_markup_parser,
            latex_fragment_parsre,
            subscript_superscript_parser,
        ));

        let plain_text_parser = plain_text_parser(non_plain_text_parsers.clone());
        // let text_parser = text_parser();

        choice((non_plain_text_parsers, plain_text_parser))
    })
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
        let parser = objects_parser();
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
        let parser = objects_parser();
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

        for e in objects_parser().parse(input).unwrap() {
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

pub(crate) fn object_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    recursive(|object_parser| {
        // 第一层：12个独立解析器
        let independent_parsers = choice((
            entity_parser(),
            latex_fragment_parser(),
            angle_link_parser(),
            line_break_parser(),
            macro_parser(),
            target_parser(),
            timestamp_parser(),
            // statistics_cookie_parser(),
            // inline_babel_call_parser(),
            // export_snippet_parser(),
            // inline_src_block_parser(),
            // plain_link_parser(), // 第12个
        ));

        // 第二层：minimal_set_object（6个）
        let minimal_set_object = {
            let entity_parser = entity_parser();
            let latex_fragment_parser = latex_fragment_parser();
            let subscript_superscript_parser = subscript_superscript_parser(object_parser.clone());
            // let text_markup_parser = text_markup_parser(object_parser.clone());
            let text_markup_parser = text_markup_parser(); // todo

            let non_plain_text_parsers = choice((
                entity_parser.clone(),
                latex_fragment_parser.clone(),
                subscript_superscript_parser.clone(),
                text_markup_parser.clone(),
            ));

            // minimal_set_object 中的纯文本解析器
            let plain_text_parser = plain_text_parser(non_plain_text_parsers.clone());

            choice((non_plain_text_parsers, plain_text_parser))
        };

        // 第三层：standard_set_object（21个）
        let standard_set_object = {
            // 依赖 minimal_set_object 的解析器（只包含其中3个）
            // let radio_link_parser = radio_link_parser(minimal_set_object.clone());
            // let regular_link_parser = regular_link_parser(minimal_set_object.clone());
            let regular_link_parser = regular_link_parser();
            // let radio_target_parser = radio_target_parser(minimal_set_object.clone());

            // 依赖 standard_set_object 的解析器（5个）
            let subscript_superscript_parser = subscript_superscript_parser(object_parser.clone());
            // let subscript_parser = subscript_parser(object_parser.clone());
            // let superscript_parser = superscript_parser(object_parser.clone());
            // let text_markup_parser = text_markup_parser(object_parser.clone());
            let text_markup_parser = text_markup_parser(); // todo
            let footnote_reference_parser = footnote_reference_parser(object_parser.clone());
            // let citation_parser = citation_parser(object_parser.clone());

            // 构建不包含 plain_text 的 standard_set_object（20个）
            let standard_set_without_plain_text = choice((
                independent_parsers.clone(), // 12个
                // radio_link_parser,           // 1个
                regular_link_parser, // 1个
                // radio_target_parser,         // 1个
                // subscript_parser,            // 1个
                // superscript_parser,          // 1个
                subscript_superscript_parser,
                text_markup_parser, // 1个
                footnote_reference_parser, // 1个
                                    // citation_parser,             // 1个
                                    // 总共：12 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 = 20个
            ));

            // standard_set_object 中的纯文本解析器
            let plain_text_for_standard =
                plain_text_parser(standard_set_without_plain_text.clone());

            // 最终的 standard_set_object（21个）
            choice((
                standard_set_without_plain_text,
                plain_text_for_standard, // 第21个
            ))
        };

        // 第四层：完整的23个对象集合
        {
            // // 不在 standard_set_object 中的2个解析器
            // let table_cell_parser = table_cell_parser(minimal_set_object.clone());
            // let citation_reference_parser = citation_reference_parser(minimal_set_object.clone());

            // 构建不包含 plain_text 的完整集合（22个）
            let full_set_without_plain_text = choice((
                standard_set_object.clone(), // 21个（包含自己的 plain_text）
                                             // table_cell_parser,           // 1个
                                             // citation_reference_parser,   // 1个
                                             // 注意：这里会有重复，但 choice 会处理
            ));

            // 最终的纯文本解析器，依赖所有其他22个对象
            let final_plain_text = plain_text_parser(full_set_without_plain_text.clone());

            // 完整的23个对象集合
            choice((
                full_set_without_plain_text,
                final_plain_text, // 第23个
            ))
        }
    })
}

// 1. minimal_set_object定义：由entity/latex_fragment/subscript/superscript/text_markup/plain_text 6个parser组成。
// 2. standard_set_object定义: 由entity/latex_fragment/angle_link/line_break/macro/target/timestamp/ statistics-cookie/inline-babel-call/export_snippet/inline_src_block/radio_link/regular_link/radio-target/subscript/superscript/text_markup/footnote_reference/citation/plain_text/plain_link 21个parser组成。
// 注意: minimal_set_objet是standard_set_object的子集，除了standard_set_object外，还有tabel_cell和citation_reference 2个object。

// 依赖关系:
// 1. entity/latex_fragment/angle_link/line_break/macro/target/timestamp/statistics-cookie/inline-babel-call/export_snippet/inline_src_block/plain_link 12个object的parser, 是独立解析器
// 2. radio_link/regular_link/table_cell/citation_reference/radio-target 5个object的parser, 依赖上文定义的minimal_set_object
// 3. subscript/superscript/text_markup/footnote_reference/citation 5object的parser, 依赖上文定义的standard_set_object
// 4. plain_text的parser依赖其余的22个object的parser，用于否定前瞻。
