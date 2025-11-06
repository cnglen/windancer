//! Object paser (todo)

pub mod entity;
mod footnote_reference;
mod latex_fragment;
mod line_break;
mod link;
mod r#macro;
mod radio_link;
mod radio_target;
mod subscript_superscript;
pub(crate) mod table_cell;
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
use crate::parser::object::radio_link::radio_link_parser;
use crate::parser::object::radio_target::radio_target_parser;
use crate::parser::object::subscript_superscript::subscript_parser;
use crate::parser::object::subscript_superscript::superscript_parser;
use crate::parser::object::table_cell::table_cell_parser;
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

pub(crate) fn object_parser<'a>() -> 
impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone{
    get_object_parser().0
}

pub(crate) fn standard_set_object_parser<'a>() -> 
impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone{
    get_object_parser().1
}

pub(crate) fn minimal_set_object_parser<'a>() -> 
impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone{
    get_object_parser().2
}

pub(crate) fn object_in_regular_link_parser<'a>() -> 
impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone{
    get_object_parser().3
}

pub(crate) fn object_in_table_cell_parser<'a>() -> 
impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone{
    get_object_parser().4
}

// (full_set_object, standard_set_object, minimal_set_object, object_in_regular_link, object_in_table_cell)
pub(crate) fn get_object_parser<'a>()
                                       -> (
    impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone,
    impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone,    
    impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone,
    impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone,
    impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
)
{
    let mut object_parser = Recursive::declare();

    // independent object (12)
    let independent_object = Parser::boxed(choice((
        latex_fragment_parser(),
        entity_parser(),
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
    )));

    // minimal set object parsers (6)
    let minimal_set_object = {
        let latex_fragment_parser = latex_fragment_parser();
        let entity_parser = entity_parser();
        let subscript_parser = subscript_parser(object_parser.clone());
        let superscript_parser = superscript_parser(object_parser.clone());
        let text_markup_parser = text_markup_parser(object_parser.clone());

        let non_plain_text_parsers = choice((
            latex_fragment_parser.clone(),
            entity_parser.clone(),
            text_markup_parser.clone(),
            subscript_parser.clone(),
            superscript_parser.clone(),
        ));

        // minimal_set_object 中的纯文本解析器
        let plain_text_parser = plain_text_parser(non_plain_text_parsers.clone());

        Parser::boxed(choice((non_plain_text_parsers, plain_text_parser)))
    };

    // regular link :: DESCRIPTION
    // One or more objects enclosed by square brackets. It can contain the minimal set of objects as well as export snippets, inline babel calls, inline source blocks, macros, and statistics cookies. It can also contain another link, but only when it is a plain or angle link. It can contain square brackets, but not ]].
    let object_in_regular_link = {
        let non_plain_text_parsers_for_regular_link = choice((
            // minimal set
            latex_fragment_parser().clone(),
            entity_parser().clone(),
            text_markup_parser(object_parser.clone()).clone(),
            subscript_parser(object_parser.clone()).clone(),
            superscript_parser(object_parser.clone()).clone(),
            // other
            // export_snippet_parser().clone(),
            // inline_babel_call_parser().clone(),
            // inline_src_block_parser().clone(),
            macro_parser().clone(),
            // statistics_cookie_parser().clone(),
            angle_link_parser().clone(),
            plain_link_parser().clone(),
        ));

        // minimal_set_object 中的纯文本解析器
        let plain_text_parser = plain_text_parser(non_plain_text_parsers_for_regular_link.clone());
        Parser::boxed(choice((
            non_plain_text_parsers_for_regular_link,
            plain_text_parser,
        )))
    };

    let object_in_table_cell = {
        let non_plain_text_parsers_for_table_cell = choice((
            // minimal set
            latex_fragment_parser().clone(),
            entity_parser().clone(),
            text_markup_parser(object_parser.clone()).clone(),
            subscript_parser(object_parser.clone()).clone(),
            superscript_parser(object_parser.clone()).clone(),
            // other
            // citation_parser(object_parser.clone()),
            // export_snippet_parser().clone(),
            footnote_reference_parser(object_parser.clone()).clone(),
            angle_link_parser().clone(),
            plain_link_parser().clone(),
            regular_link_parser(object_parser.clone()).clone(),
            radio_link_parser(object_parser.clone()).clone(),
            macro_parser().clone(),
            radio_target_parser(object_parser.clone()).clone(),
            target_parser(),
            timestamp_parser(),
        ));

        // minimal_set_object 中的纯文本解析器
        let plain_text_parser = plain_text_parser(non_plain_text_parsers_for_table_cell.clone());
        Parser::boxed(choice((
            non_plain_text_parsers_for_table_cell,
            plain_text_parser,
        )))
    };

    // standard set object (21)
    let standard_set_object = {
        // 依赖 minimal_set_object 的解析器（只包含其中3个）
        let radio_link_parser = radio_link_parser(minimal_set_object.clone());
        let regular_link_parser = regular_link_parser(object_in_regular_link.clone());
        let radio_target_parser = radio_target_parser(minimal_set_object.clone());

        // 依赖 standard_set_object 的解析器（5个）
        let text_markup_parser = text_markup_parser(object_parser.clone());
        let subscript_parser = subscript_parser(object_parser.clone());
        let superscript_parser = superscript_parser(object_parser.clone());
        let footnote_reference_parser = footnote_reference_parser(object_parser.clone());
        // let citation_parser = citation_parser(object_parser.clone());

        // 构建不包含 plain_text 的 standard_set_object（20个）
        let standard_set_without_plain_text = choice((
            radio_link_parser,           // 1个
            regular_link_parser,         // 1个
            independent_object.clone(), // 12个
            radio_target_parser,         // 1个
            text_markup_parser,          // 1个
            subscript_parser,            // 1个
            superscript_parser,          // 1个
            footnote_reference_parser,   // 1个
                                         // citation_parser,             // 1个
                                         // 总共：12 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 = 20个
        ));

        // standard_set_object 中的纯文本解析器
        let plain_text_for_standard = plain_text_parser(standard_set_without_plain_text.clone());

        // 最终的 standard_set_object（21个）

        Parser::boxed(choice((
            standard_set_without_plain_text,
            plain_text_for_standard, // 第21个
        )))
    };

    // the other 2 object (not in standard set object)
    let table_cell_parser = table_cell_parser(object_in_table_cell.clone());
    // let citation_reference_parser = citation_reference_parser(minimal_set_object.clone());

    // 构建不包含 plain_text 的完整集合（22个）
    let full_set_without_plain_text = choice((
        standard_set_object.clone(), // 21个（包含自己的 plain_text）
        table_cell_parser,           // 1个
                                     // citation_reference_parser,   // 1个
                                     // 注意：这里会有重复，但 choice 会处理
    ));

    // 最终的纯文本解析器，依赖所有其他22个对象
    let final_plain_text = plain_text_parser(full_set_without_plain_text.clone());

    // 完整的23个对象集合
    object_parser.define(choice((
        full_set_without_plain_text,
        final_plain_text, // 23th
    )));
    let full_set_object = Parser::boxed(object_parser);

    (full_set_object, standard_set_object, minimal_set_object, object_in_regular_link, object_in_table_cell)
}

pub(crate) fn object_parser_old<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    recursive(|object_parser| {
        // 第一层：12个独立解析器
        let independent_object = Parser::boxed(choice((
            latex_fragment_parser(),
            entity_parser(),
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
        )));

        // 第二层：minimal_set_object（6个）
        let minimal_set_object = {
            let latex_fragment_parser = latex_fragment_parser();
            let entity_parser = entity_parser();
            let subscript_parser = subscript_parser(object_parser.clone());
            let superscript_parser = superscript_parser(object_parser.clone());
            let text_markup_parser = text_markup_parser(object_parser.clone());

            let non_plain_text_parsers = choice((
                latex_fragment_parser.clone(),
                entity_parser.clone(),
                text_markup_parser.clone(),
                subscript_parser.clone(),
                superscript_parser.clone(),
            ));

            // minimal_set_object 中的纯文本解析器
            let plain_text_parser = plain_text_parser(non_plain_text_parsers.clone());

            Parser::boxed(choice((non_plain_text_parsers, plain_text_parser)))
        };

        // regular link :: DESCRIPTION
        // One or more objects enclosed by square brackets. It can contain the minimal set of objects as well as export snippets, inline babel calls, inline source blocks, macros, and statistics cookies. It can also contain another link, but only when it is a plain or angle link. It can contain square brackets, but not ]].
        let object_in_regular_link = {
            let non_plain_text_parsers_for_regular_link = choice((
                // minimal set
                latex_fragment_parser().clone(),
                entity_parser().clone(),
                text_markup_parser(object_parser.clone()).clone(),
                subscript_parser(object_parser.clone()).clone(),
                superscript_parser(object_parser.clone()).clone(),
                // other
                // export_snippet_parser().clone(),
                // inline_babel_call_parser().clone(),
                // inline_src_block_parser().clone(),
                macro_parser().clone(),
                // statistics_cookie_parser().clone(),
                angle_link_parser().clone(),
                plain_link_parser().clone(),
            ));

            // minimal_set_object 中的纯文本解析器
            let plain_text_parser =
                plain_text_parser(non_plain_text_parsers_for_regular_link.clone());
            Parser::boxed(choice((
                non_plain_text_parsers_for_regular_link,
                plain_text_parser,
            )))
        };

        let object_in_table_cell = {
            let non_plain_text_parsers_for_table_cell = choice((
                // minimal set
                latex_fragment_parser().clone(),
                entity_parser().clone(),
                text_markup_parser(object_parser.clone()).clone(),
                subscript_parser(object_parser.clone()).clone(),
                superscript_parser(object_parser.clone()).clone(),
                // other
                // citation_parser(object_parser.clone()),
                // export_snippet_parser().clone(),
                footnote_reference_parser(object_parser.clone()).clone(),
                angle_link_parser().clone(),
                plain_link_parser().clone(),
                regular_link_parser(object_parser.clone()).clone(),
                radio_link_parser(object_parser.clone()).clone(),
                macro_parser().clone(),
                radio_target_parser(object_parser.clone()).clone(),
                target_parser(),
                timestamp_parser(),
            ));

            // minimal_set_object 中的纯文本解析器
            let plain_text_parser =
                plain_text_parser(non_plain_text_parsers_for_table_cell.clone());
            Parser::boxed(choice((
                non_plain_text_parsers_for_table_cell,
                plain_text_parser,
            )))
        };

        // 第三层：standard_set_object（21个）
        let standard_set_object = {
            // 依赖 minimal_set_object 的解析器（只包含其中3个）
            let radio_link_parser = radio_link_parser(minimal_set_object.clone());
            let regular_link_parser =
                regular_link_parser(object_in_regular_link);
            let radio_target_parser = radio_target_parser(minimal_set_object.clone());

            // 依赖 standard_set_object 的解析器（5个）
            let text_markup_parser = text_markup_parser(object_parser.clone());
            let subscript_parser = subscript_parser(object_parser.clone());
            let superscript_parser = superscript_parser(object_parser.clone());
            let footnote_reference_parser = footnote_reference_parser(object_parser.clone());
            // let citation_parser = citation_parser(object_parser.clone());

            // 构建不包含 plain_text 的 standard_set_object（20个）
            let standard_set_without_plain_text = choice((
                radio_link_parser,           // 1个
                regular_link_parser,         // 1个
                independent_object.clone(), // 12个
                radio_target_parser,         // 1个
                text_markup_parser,          // 1个
                subscript_parser,            // 1个
                superscript_parser,          // 1个
                footnote_reference_parser,   // 1个
                                             // citation_parser,             // 1个
                                             // 总共：12 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 = 20个
            ));

            // standard_set_object 中的纯文本解析器
            let plain_text_for_standard =
                plain_text_parser(standard_set_without_plain_text.clone());

            // 最终的 standard_set_object（21个）

            Parser::boxed(choice((
                standard_set_without_plain_text,
                plain_text_for_standard, // 第21个
            )))
        };

        // 第四层：完整的23个对象集合
        {
            // // 不在 standard_set_object 中的2个解析器
            let table_cell_parser =
                table_cell_parser(object_in_table_cell.clone());
            // let citation_reference_parser = citation_reference_parser(minimal_set_object.clone());

            // 构建不包含 plain_text 的完整集合（22个）
            let full_set_without_plain_text = choice((
                standard_set_object.clone(), // 21个（包含自己的 plain_text）
                table_cell_parser,           // 1个
                                             // citation_reference_parser,   // 1个
                                             // 注意：这里会有重复，但 choice 会处理
            ));

            // 最终的纯文本解析器，依赖所有其他22个对象
            let final_plain_text = plain_text_parser(full_set_without_plain_text.clone());

            // 完整的23个对象集合
            Parser::boxed(choice((
                full_set_without_plain_text,
                final_plain_text, // 第23个
            )))
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
    use crate::parser::common::get_parsers_output;
    use crate::parser::syntax::OrgLanguage;
    use rowan::SyntaxNode;
    use rowan::{GreenNode, GreenToken, NodeOrToken};
    use pretty_assertions::assert_eq;


    fn get_objects_string()-> String {
        r##"
minimal set objects(6)

01 text-markup: *bold*, /italic/, _underline_, +strike-throught+, ​~code~, =verbatim=

02 entity: \alpha

03 latex-fragment: $\sum_{i=1}^{n}i$

04 suscripot: a_{i,j}

05 supscript: a^3

06 plain-text: text

standard set object(21)

07 footnote-reference: [fn:1]

08 line-break: \\

09 timestamp: <1234-07-31>

10 macro: {{{title}}}

11 radio-target: <<<radio target>>>

12 target: <<target>>

13-16 link: plain link https://foo.bar, angle link <mailto:foo@bar>, radio link radio target, regular link [[target]]

17 statistics-cookie: todo

18 inline-babel-call: todo

19 inline-src-block: todo

20 citation: todo

21 export-snippet: todo


other objects (2):

22 table-cell:
| foo |   | bar |

23 citation-reference: todo
"##.to_owned()

            
    }
    
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
    fn test_minimal_set_object() {
        let minimal_objects_parser = minimal_set_object_parser().repeated().at_least(1).collect::<Vec<_>>();

        assert_eq!(get_parsers_output(minimal_objects_parser, &get_objects_string()), r##"Root@0..749
  Text@0..41 "\nminimal set objects( ..."
  Bold@41..47
    Asterisk@41..42 "*"
    Text@42..46 "bold"
    Asterisk@46..47 "*"
  Text@47..49 ", "
  Italic@49..57
    Slash@49..50 "/"
    Text@50..56 "italic"
    Slash@56..57 "/"
  Text@57..59 ", "
  Underline@59..70
    Underscore@59..60 "_"
    Text@60..69 "underline"
    Underscore@69..70 "_"
  Text@70..72 ", "
  Strikethrough@72..89
    Plus@72..73 "+"
    Text@73..88 "strike-throught"
    Plus@88..89 "+"
  Text@89..94 ", \u{200b}"
  Code@94..100
    Tilde@94..95 "~"
    Text@95..99 "code"
    Tilde@99..100 "~"
  Text@100..102 ", "
  Verbatim@102..112
    Equals@102..103 "="
    Text@103..111 "verbatim"
    Equals@111..112 "="
  Text@112..125 "\n\n02 entity: "
  Entity@125..131
    BackSlash@125..126 "\\"
    EntityName@126..131 "alpha"
  Text@131..152 "\n\n03 latex-fragment: "
  LatexFragment@152..169
    Dollar@152..153 "$"
    Text@153..168 "\\sum_{i=1}^{n}i"
    Dollar@168..169 "$"
  Text@169..186 "\n\n04 suscripot: a"
  Subscript@186..192
    Caret@186..187 "_"
    LeftCurlyBracket@187..188 "{"
    Text@188..191 "i,j"
    RightCurlyBracket@191..192 "}"
  Text@192..209 "\n\n05 supscript: a"
  Superscript@209..211
    Caret@209..210 "^"
    Text@210..211 "3"
  Text@211..749 "\n\n06 plain-text: text ..."
"##);
        
    }

    #[test]
    fn test_standard_set_object() {
        let standard_objects_parser = standard_set_object_parser().repeated().at_least(1).collect::<Vec<_>>();

        assert_eq!(get_parsers_output(standard_objects_parser, &get_objects_string()), r##"Root@0..749
  Text@0..41 "\nminimal set objects( ..."
  Bold@41..47
    Asterisk@41..42 "*"
    Text@42..46 "bold"
    Asterisk@46..47 "*"
  Text@47..49 ", "
  Italic@49..57
    Slash@49..50 "/"
    Text@50..56 "italic"
    Slash@56..57 "/"
  Text@57..59 ", "
  Underline@59..70
    Underscore@59..60 "_"
    Text@60..69 "underline"
    Underscore@69..70 "_"
  Text@70..72 ", "
  Strikethrough@72..89
    Plus@72..73 "+"
    Text@73..88 "strike-throught"
    Plus@88..89 "+"
  Text@89..94 ", \u{200b}"
  Code@94..100
    Tilde@94..95 "~"
    Text@95..99 "code"
    Tilde@99..100 "~"
  Text@100..102 ", "
  Verbatim@102..112
    Equals@102..103 "="
    Text@103..111 "verbatim"
    Equals@111..112 "="
  Text@112..125 "\n\n02 entity: "
  Entity@125..131
    BackSlash@125..126 "\\"
    EntityName@126..131 "alpha"
  Text@131..152 "\n\n03 latex-fragment: "
  LatexFragment@152..169
    Dollar@152..153 "$"
    Text@153..168 "\\sum_{i=1}^{n}i"
    Dollar@168..169 "$"
  Text@169..186 "\n\n04 suscripot: a"
  Subscript@186..192
    Caret@186..187 "_"
    LeftCurlyBracket@187..188 "{"
    Text@188..191 "i,j"
    RightCurlyBracket@191..192 "}"
  Text@192..209 "\n\n05 supscript: a"
  Superscript@209..211
    Caret@209..210 "^"
    Text@210..211 "3"
  Text@211..282 "\n\n06 plain-text: text ..."
  FootnoteReference@282..288
    LeftSquareBracket@282..283 "["
    Text@283..285 "fn"
    Colon@285..286 ":"
    FootnoteReferenceLabel@286..287 "1"
    RightSquareBracket@287..288 "]"
  Text@288..305 "\n\n08 line-break: "
  LineBreak@305..307
    BackSlash2@305..307 "\\\\"
  Text@307..323 "\n\n09 timestamp: "
  Timestamp@323..335
    Text@323..335 "<1234-07-31>"
  Text@335..347 "\n\n10 macro: "
  Macro@347..358
    LeftCurlyBracket3@347..350 "{{{"
    MacroName@350..355 "title"
    RightCurlyBracket3@355..358 "}}}"
  Text@358..377 "\n\n11 radio-target: "
  RadioTarget@377..395
    LeftAngleBracket3@377..380 "<<<"
    Text@380..392 "radio target"
    RightAngleBracket3@392..395 ">>>"
  Text@395..408 "\n\n12 target: "
  Target@408..418
    LeftAngleBracket2@408..410 "<<"
    Text@410..416 "target"
    RightAngleBracket2@416..418 ">>"
  Text@418..443 "\n\n13-16 link: plain l ..."
  PlainLink@443..458
    Text@443..448 "https"
    Colon@448..449 ":"
    Text@449..458 "//foo.bar"
  Text@458..471 ", angle link "
  AngleLink@471..487
    LeftAngleBracket@471..472 "<"
    Text@472..478 "mailto"
    Colon@478..479 ":"
    Text@479..486 "foo@bar"
    RightAngleBracket@486..487 ">"
  Text@487..527 ", radio link radio ta ..."
  Link@527..537
    LeftSquareBracket@527..528 "["
    LinkPath@528..536
      LeftSquareBracket@528..529 "["
      Text@529..535 "target"
      RightSquareBracket@535..536 "]"
    RightSquareBracket@536..537 "]"
  Text@537..749 "\n\n17 statistics-cooki ..."
"##);
        
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
