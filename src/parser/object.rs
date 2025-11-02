//! Object paser (todo)

pub mod entity;
mod footnote_reference;
mod latex_fragment;
mod line_break;
mod link;
mod r#macro;
mod subscript_superscript;
mod target;
mod text;
mod text_markup;
mod timestamp;

use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::object::entity::entity_parser;
use crate::parser::object::footnote_reference::footnote_reference_parser;
use crate::parser::object::latex_fragment::latex_fragment_parser;
use crate::parser::object::line_break::line_break_parser;
use crate::parser::object::link::{angle_link_parser, plain_link_parser, regular_link_parser};
use crate::parser::object::r#macro::macro_parser;
use crate::parser::object::subscript_superscript::subscript_parser;
use crate::parser::object::subscript_superscript::superscript_parser;
use crate::parser::object::target::target_parser;
use crate::parser::object::text::plain_text_parser;
use crate::parser::object::text_markup::text_markup_parser;
use crate::parser::object::timestamp::timestamp_parser;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::GreenToken;

/// 解析行终止符：换行符或输入结束
pub(crate) fn newline_or_ending<'a>()
-> impl Parser<'a, &'a str, Option<String>, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>
+ Clone {
    just('\n').map(|c| Some(String::from(c))).or(end().to(None))
}

/// 创建一个不区分大小写的关键字解析器
pub(crate) fn just_case_insensitive<'a>(
    s: &'a str,
) -> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
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
-> impl Parser<'a, &'a str, Option<String>, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>
+ Clone {
    any()
        .repeated()
        .then(just('\n').map(|c| Some(String::from(c))).or(end().to(None)))
        .map(|_| Some("OK".to_string()))
}

/// 解析零个或多个空白字符（包括空格、制表符等）
pub(crate) fn whitespaces<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    one_of(" \t").repeated().collect::<String>()
}
/// 解析一个或多个空白字符（包括空格、制表符等）
pub(crate) fn whitespaces_g1<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    one_of(" \t").repeated().at_least(1).collect::<String>()
}

/// 解析一行:
/// Line <- (!EOL .)+
/// EOL <- '\r'? '\n'
pub(crate) fn line_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
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
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
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
-> impl Parser<'a, &'a str, GreenToken, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>
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

pub(crate) fn objects_parser<'a>()
-> impl Parser<'a, &'a str, Vec<S2>, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    object_parser().repeated().at_least(1).collect::<Vec<_>>()
}

/// objects_parser
// object defintion:
// minimal_set_object := entity + latex_fragment + subscript + superscript + text_markup + plain_text, 6 objects
// standard_set_object := entity + latex_fragment + angle_link + line_break + macro + target + timestamp +  statistics-cookie + inline-babel-call + export_snippet + inline_src_block + radio_link + regular_link + radio-target + subscript + superscript + text_markup + footnote_reference + citation + plain_text + plain_link, 21 objects
// Note: minimal_set_objet is subset of standard_set_object，which includes standard_set_object(12), tabel_cell and citation_reference.

// dependency:
// 1. entity/latex_fragment/angle_link/line_break/macro/target/timestamp/statistics-cookie/inline-babel-call/export_snippet/inline_src_block/plain_link 12 objects' parser, are independent parser(独立解析器)
// 2. radio_link/regular_link/table_cell/citation_reference/radio-target 5 objects' parser, depends on **minimal_set_object**
// 3. subscript/superscript/text_markup/footnote_reference/citation 5 objects' parser,depends on **standard_set_object**
// 4. plain_text's parser dpendnes all other 22 object's parsers，used to lookahead NOT
// TODO: select! use prev_char state? performance? first char
pub(crate) fn object_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
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
            // todo
            // statistics_cookie_parser(),
            // inline_babel_call_parser(),
            // export_snippet_parser(),
            // inline_src_block_parser(),
            plain_link_parser(), // 第12个
        ));

        // 第二层：minimal_set_object（6个）
        let minimal_set_object = {
            let entity_parser = entity_parser();
            let latex_fragment_parser = latex_fragment_parser();
            let subscript_parser = subscript_parser(object_parser.clone());
            let superscript_parser = superscript_parser(object_parser.clone());
            let text_markup_parser = text_markup_parser(object_parser.clone());

            let non_plain_text_parsers = choice((
                entity_parser.clone(),
                latex_fragment_parser.clone(),
                text_markup_parser.clone(),
                subscript_parser.clone(),
                superscript_parser.clone(),
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
            let text_markup_parser = text_markup_parser(object_parser.clone());
            let subscript_parser = subscript_parser(object_parser.clone());
            let superscript_parser = superscript_parser(object_parser.clone());
            let footnote_reference_parser = footnote_reference_parser(object_parser.clone());
            // let citation_parser = citation_parser(object_parser.clone());

            // 构建不包含 plain_text 的 standard_set_object（20个）
            let standard_set_without_plain_text = choice((
                independent_parsers.clone(), // 12个
                // radio_link_parser,           // 1个
                regular_link_parser, // 1个
                // radio_target_parser,         // 1个
                text_markup_parser, // 1个
                subscript_parser,   // 1个
                superscript_parser, // 1个
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
    use rowan::{GreenNode, GreenToken, NodeOrToken};

    #[test]
    fn test_is_ending() {
        let mut state = RollbackState(ParserState::default());
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
        let mut state = RollbackState(ParserState::default());
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
        let mut state = RollbackState(ParserState::default());
        let input = "a row\n";
        let s = line_parser().parse_with_state(input, &mut state);
        println!("{:?}", s);
    }

    #[test]
    fn test_line_lookahead() {
        let mut state = RollbackState(ParserState::default());
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
