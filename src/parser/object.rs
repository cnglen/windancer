//! Object paser
// extern crate test;
use std::collections::HashSet;
pub mod entity;
mod footnote_reference;
mod latex_fragment;
mod line_break;
mod link;
mod r#macro;
mod radio_link;
mod radio_target;
pub(crate) mod subscript_superscript;
pub(crate) mod table_cell;
mod target;
mod text;
mod text_markup;
pub(crate) mod timestamp;
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
use crate::parser::object::target::target_parser;
use crate::parser::object::text::plain_text_parser;
use crate::parser::object::text_markup::text_markup_parser;
use crate::parser::object::timestamp::timestamp_parser;
use crate::parser::{MyExtra, NT, OSK};

use chumsky::prelude::*;

use crate::parser::config::OrgParserConfig;

use super::config::OrgUseSubSuperscripts;

pub const LF: &str = "\n";
pub const CRLF: &str = "\r\n";

// // Input which suppport get the previous Token
// pub trait PrevInput<'src>: Input<'src> {
//     unsafe fn prev(cache: &mut Self::Cache, cursor: &Self::Cursor) -> Option<Self::Token>;
// }

// impl<'src> PrevInput<'src> for &'src str {
//     #[inline(always)]
//     unsafe fn prev(this: &mut Self::Cache, cursor: &Self::Cursor) -> Option<Self::Token> {
//         let idx_byte_current = *cursor;

//         if idx_byte_current == 0 {
//             return None;
//         }

//         let bytes = this.as_bytes();
//         let start = idx_byte_current.saturating_sub(3);
//         for idx in (start..idx_byte_current).rev() {
//             let b = bytes[idx];
//             // from is_utf8_char_boundary()
//             // This is bit magic equivalent to: b < 128 || b >= 192
//             if (b as i8) >= -0x40 {
//                 return Some(unsafe {
//                     this.get_unchecked(idx..).chars().next().unwrap_unchecked()
//                 });
//             }
//         }

//         if start > 0 && (bytes[start - 1] as i8) >= -0x40 {
//             Some(unsafe {
//                 this.get_unchecked(start - 1..)
//                     .chars()
//                     .next()
//                     .unwrap_unchecked()
//             })
//         } else {
//             None // this should be unreachable
//         }
//     }
// }

// // Add prev() API for InputRef
// pub trait PrevInputRef<'src, I> {
//     fn prev(&mut self) -> Option<I::Token>
//     where
//         I: PrevInput<'src>;
// }

// // fixme: InputRef input.rs cache: pub(crate) -> pub
// impl<'src, 'parse, I: Input<'src>, E: extra::ParserExtra<'src, I>> PrevInputRef<'src, I>
//     for chumsky::input::InputRef<'src, 'parse, I, E>
// {
//     #[inline(always)]
//     fn prev(&mut self) -> Option<I::Token>
//     where
//         I: PrevInput<'src>,
//     {

//         // E0716
//         let binding = self.cursor();
//         let a = binding.inner();
//         let token = unsafe { I::prev(self.cache, a) };

//         token
//     }
// }

// // // valid prev char using `f`
// // pub(crate) fn prev_valid_parser<'a, C: 'a, F: Fn(Option<char>) -> bool + Clone>(
// //     f: F,
// // ) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
// //     custom(move |inp| {
// //         let before = inp.cursor();
// //         let maybe_prev = inp.prev();
// //         if f(maybe_prev) {
// //             Ok(())
// //         } else {
// //             Err(Rich::custom(
// //                 inp.span_since(&before),
// //                 format!("invalid PRE: {maybe_prev:?}"),
// //             ))
// //         }
// //     })
// // }

fn get_prev_char_index(s: &str, index: usize) -> Option<char> {
    if index == 0 {
        return None;
    }

    let bytes = s.as_bytes();
    let start = index.saturating_sub(3);
    for idx in (start..index).rev() {
        let b = bytes[idx];
        if (b as i8) >= -0x40 {
            return Some(unsafe { s.get_unchecked(idx..).chars().next().unwrap_unchecked() });
        }
    }

    if start > 0 && (bytes[start - 1] as i8) >= -0x40 {
        Some(unsafe {
            s.get_unchecked(start - 1..)
                .chars()
                .next()
                .unwrap_unchecked()
        })
    } else {
        None // this should be unreachable
    }
}

pub(crate) fn prev_valid_parser<'a, C: 'a, F: Fn(Option<char>) -> bool + Clone>(
    f: F,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    custom(move |inp| {
        let before = inp.cursor();
        let idx_byte_current = before.inner();
        // println!("s={}", inp.full_slice());
        let maybe_prev = get_prev_char_index(inp.full_slice(), *idx_byte_current);

        if f(maybe_prev) {
            Ok(())
        } else {
            Err(Rich::custom(
                inp.span_since(&before),
                format!("invalid PRE: {maybe_prev:?}"),
            ))
        }
    })
}

// case insensitive keyword parser:
// - name <- (a-zA-Z0-9)*
// - if name in allowe_keywords, Ok(name)
// example:
// - OK: keyword_ci_parser("def").parse("def")
// - ERR: keyword_ci_parser("def").parse("define")
pub(crate) fn keyword_ci_parser_v2<'a, C: 'a>(
    allowed_keywords: HashSet<String>,
) -> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone {
    custom(move |inp| {
        let before = inp.cursor();

        loop {
            match inp.peek() {
                Some(c) if matches!(c, 'a'..'z' | 'A'..'Z'| '0'..'9') => {
                    inp.next();
                }
                _ => {
                    break;
                }
            }
        }
        let name: &str = inp.slice_since(&before..);

        if name.is_empty() {
            return Err(Rich::custom(
                inp.span_since(&before),
                format!("no valid string found: empty found"),
            ));
        }

        let allowed_keywords_uppercase: std::collections::HashSet<String> =
            allowed_keywords.iter().map(|m| m.to_uppercase()).collect();
        if !allowed_keywords_uppercase.contains(&name.to_uppercase()) {
            return Err(Rich::custom(
                inp.span_since(&before),
                format!("invalid key: '{}'", name),
            ));
        }
        Ok(name)
    })
}

pub(crate) fn keyword_cs_parser_v2<'a, C: 'a>(
    allowed_keywords: HashSet<String>,
) -> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone {
    custom(move |inp| {
        let before = inp.cursor();
        loop {
            match inp.peek() {
                Some(c) if matches!(c, 'a'..'z' | 'A'..'Z'| '0'..'9') => {
                    inp.next();
                }
                _ => {
                    break;
                }
            }
        }
        let name: &str = inp.slice_since(&before..);

        if name.is_empty() {
            return Err(Rich::custom(
                inp.span_since(&before),
                format!("no valid string found: empty found"),
            ));
        }

        if !allowed_keywords.contains(&name.to_string()) {
            return Err(Rich::custom(
                inp.span_since(&before),
                format!("invalid key: '{}'", name),
            ));
        }

        Ok(name)
    })
}

// case sensitive keyword parser:
// - name <- (a-zA-Z0-9)*
// - if name in allowe_keywords, Ok(name)
pub(crate) fn keyword_cs_parser<'a, C: 'a>(
    allowed_keywords: &phf::Set<&'static str>,
) -> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone {
    custom(|inp| {
        let before = inp.cursor();
        loop {
            match inp.peek() {
                Some(c) if matches!(c, 'a'..'z' | 'A'..'Z'| '0'..'9') => {
                    inp.next();
                }
                _ => {
                    break;
                }
            }
        }
        let name: &str = inp.slice_since(&before..);

        if name.is_empty() {
            return Err(Rich::custom(
                inp.span_since(&before),
                format!("no valid string found: empty found"),
            ));
        }

        if !allowed_keywords.contains(&name) {
            return Err(Rich::custom(
                inp.span_since(&before),
                format!("invalid key: '{}'", name),
            ));
        }

        Ok(name)
    })
}

/// 解析行终止符：换行符(LF/CRLF)或输入结束
pub(crate) fn newline_or_ending<'a, C: 'a>()
-> impl Parser<'a, &'a str, Option<&'a str>, MyExtra<'a, C>> + Clone + Copy {
    choice((
        just(LF).to(Some(LF)),
        just(CRLF).to(Some(CRLF)),
        end().to(None),
    ))
}

/// 解析行终止符：换行符(LF/CRLF)
pub(crate) fn newline<'a, C: 'a>() -> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone {
    choice((just(LF), just(CRLF)))
}

pub(crate) fn just_case_insensitive<'a, C: 'a>(
    s: &'a str,
) -> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone + Copy {
    custom(move |inp| {
        let before = inp.cursor();
        for expected_char in s.chars() {
            let z: Option<char> = inp.next();
            match z {
                Some(r) if r.eq_ignore_ascii_case(&expected_char) => {}
                _ => {
                    let found = inp.slice_since(&before..);
                    let error = Rich::custom(
                        inp.span_since(&before),
                        &format!("expected '{}' found '{}'", s, found),
                    );

                    return Err(error);
                }
            }
        }
        Ok(inp.slice_since(&before..))
    })
}

/// zero or more whitespaces(including space, \tab)
pub(crate) fn whitespaces<'a, C: 'a>()
-> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone + Copy {
    one_of(" \t").repeated().to_slice()
}
/// one or more whitespaces(including space, \tab)
pub(crate) fn whitespaces_g1<'a, C: 'a>()
-> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone + Copy {
    one_of(" \t").repeated().at_least(1).to_slice()
}

/// 解析一行:
/// Line <- (!EOL .)+
/// EOL <- CR? LF
pub(crate) fn line_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone + Copy {
    let end_of_line = choice((just(LF).to(()), just(CRLF).to(()), end()));

    none_of(CRLF)
        .repeated()
        .at_least(1)
        .then(end_of_line)
        .to_slice()
}

/// 解析一行: 允许空行
pub(crate) fn line_parser_allow_blank<'a, C: 'a>()
-> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone {
    let end_of_line = choice((just(LF).to(()), just(CRLF).to(()), end()));

    none_of(CRLF).repeated().then(end_of_line).to_slice()
}

pub(crate) fn blank_line_str_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone + Copy {
    whitespaces().then(just(LF).or(just(CRLF))).to_slice()
}

/// Blank Line Parser := 空白字符后紧跟行终止符, PEG定义如下
/// ```text
/// BlankLine <- WS* EOL
/// WS <- [ \t]
/// EOL <- '\r'? '\n'
/// ```
pub(crate) fn blank_line_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone
{
    whitespaces()
        .then(just(LF).or(just(CRLF)))
        .to_slice()
        .map(|s| crate::token!(OSK::BlankLine, s))
}

pub(crate) fn objects_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone {
    object_parser(config)
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
}

pub(crate) fn standard_set_objects_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone {
    standard_set_object_parser(config)
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
}

/// objects_parser
// object defintion:
// minimal_set_object := entity + latex_fragment + subscript + superscript + text_markup + plain_text, 6 objects
// standard_set_object := entity + latex_fragment + angle_link + line_break + macro + target + timestamp +  statistics-cookie + inline-babel-call + export_snippet + inline_src_block + radio_link + regular_link + radio-target + subscript + superscript + text_markup + footnote_reference + citation + plain_text + plain_link, 21 objects
// Note: minimal_set_objet is subset of standard_set_object，full_set object includes standard_set_object(12), tabel_cell and citation_reference.

// dependency:
// 1. entity/latex_fragment/angle_link/line_break/macro/target/timestamp/statistics-cookie/inline-babel-call/export_snippet/inline_src_block/plain_link 12 objects' parser, are independent parser(独立解析器)
// 2. radio_link/regular_link/table_cell/citation_reference/radio-target 5 objects' parser, depends on **minimal_set_object**
// 3. subscript/superscript/text_markup/footnote_reference/citation 5 objects' parser,depends on **standard_set_object**
// 4. plain_text's parser dpendnes all other 22 object's parsers，used to lookahead NOT
// TODO: select! use prev_char state? performance? first char

// full set object
pub(crate) fn object_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    get_object_parser(config).0
}

pub(crate) fn standard_set_object_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    get_object_parser(config).1
}

#[allow(unused)]
pub(crate) fn minimal_set_object_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    get_object_parser(config).2
}

#[allow(unused)]
pub(crate) fn object_in_regular_link_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    get_object_parser(config).3
}

pub(crate) fn object_in_table_cell_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    get_object_parser(config).4
}

pub(crate) fn object_in_keyword_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    get_object_parser(config).5
}

// (full_set_object, standard_set_object, minimal_set_object, object_in_regular_link, object_in_table_cell)
// - full_set_object DOES NOT include table_cell
pub(crate) fn get_object_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> (
    impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
    impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
    impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
    impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
    impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
    impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
) {
    let org_use_sub_superscripts = config.clone().org_use_sub_superscripts;
    let mut full_set_object = Recursive::declare();
    let mut minimal_set_object = Recursive::declare();
    let mut standard_set_object = Recursive::declare();
    let mut object_in_table_cell = Recursive::declare();
    let mut object_in_regular_link = Recursive::declare();
    let mut object_in_keyword = Recursive::declare();

    // total 22 object parsers exlucidng plain_text
    let latex_fragment = latex_fragment_parser();
    let entity = entity_parser();

    let angle_link = angle_link_parser();
    let line_break = line_break_parser();
    let r#macro = macro_parser();
    let target = target_parser();
    let timestamp = timestamp_parser();
    // let statistics_cookie = statistics_cookie_parser();
    // let inline_babel_call = inline_babel_call_parser();
    // let export_snippet = export_snippet_parser();
    // let inline_src_block = inline_src_block_parser();
    let plain_link = plain_link_parser();

    let text_markup = text_markup_parser(standard_set_object.clone());
    let radio_link = radio_link_parser(minimal_set_object.clone());
    let regular_link = regular_link_parser(object_in_regular_link.clone());
    let radio_target = radio_target_parser(minimal_set_object.clone());
    let footnote_reference = footnote_reference_parser(standard_set_object.clone());
    // let citation_parser = citation_parser(standard_set_object.clone());
    // let table_cell = table_cell_parser(object_in_table_cell.clone()); // table_cell_parser ONLY used in table.rs
    // let citation_reference = citation_reference_parser(minimal_set_object.clone());

    // independent object (12)
    let independent_object = Parser::boxed(choice((
        latex_fragment.clone(), // 10%'s rewind!!! 20e4
        entity.clone(),         // 5%
        angle_link.clone(),
        line_break.clone(),
        r#macro.clone(),
        target.clone(),
        timestamp.clone(),
        // statistics_cookie.clone(),
        // inline_babel_call.clone(),
        // export_snippet.clone(),
        // inline_src_block.clone(),
        plain_link.clone(), // 第12个
    )));

    match org_use_sub_superscripts {
        OrgUseSubSuperscripts::Brace | OrgUseSubSuperscripts::True => {
            let subscript = subscript_parser(
                org_use_sub_superscripts.clone(),
                standard_set_object.clone(),
            );
            let superscript = superscript_parser(
                org_use_sub_superscripts.clone(),
                standard_set_object.clone(),
            );

            // minimal set object: 6
            let non_plain_text_for_minimal = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup.clone(),
                subscript.clone(),
                superscript.clone(),
            )));
            let non_plain_text_for_minimal_lookahead = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup::simple_text_markup_parser(),
                subscript_superscript::simple_subscript_parser(org_use_sub_superscripts.clone()),
                subscript_superscript::simple_superscript_parser(org_use_sub_superscripts.clone()),
            )));

            let plain_text_for_minimal = plain_text_parser(non_plain_text_for_minimal_lookahead);
            minimal_set_object.define(choice((non_plain_text_for_minimal, plain_text_for_minimal)));

            // object allowed in regular link: 13
            let non_plain_text_parsers_for_regular_link = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup.clone(),
                subscript.clone(),
                superscript.clone(),
                // export_snippet.clone(),
                // inline_babel_call.clone(),
                // inline_src_block.clone(),
                r#macro.clone(),
                // statistics_cookie.clone(),
                angle_link.clone(),
                plain_link.clone(),
            )));
            // object allowed in regular link: 13
            let non_plain_text_parsers_for_regular_link_lookahead = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup::simple_text_markup_parser(),
                subscript_superscript::simple_subscript_parser(org_use_sub_superscripts.clone()),
                subscript_superscript::simple_superscript_parser(org_use_sub_superscripts.clone()),
                // export_snippet.clone(),
                // inline_babel_call.clone(),
                // inline_src_block.clone(),
                r#macro.clone(),
                // statistics_cookie.clone(),
                angle_link.clone(),
                plain_link.clone(),
            )));

            let plain_text_parser_for_regular_link =
                plain_text_parser(non_plain_text_parsers_for_regular_link_lookahead);
            object_in_regular_link.define(choice((
                non_plain_text_parsers_for_regular_link,
                plain_text_parser_for_regular_link,
            )));

            // object allowed in table cell: 17
            let non_plain_text_parsers_for_table_cell = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup.clone(),
                subscript.clone(),
                superscript.clone(),
                // citation.clone(),
                // export_snippet.clone(),
                footnote_reference.clone(),
                angle_link.clone(),
                plain_link.clone(),
                regular_link.clone(),
                radio_link.clone(),
                r#macro.clone(),
                radio_target.clone(),
                target.clone(),
                timestamp.clone(),
            )));
            let non_plain_text_parsers_for_table_cell_lookahead = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup::simple_text_markup_parser(),
                subscript_superscript::simple_subscript_parser(org_use_sub_superscripts.clone()),
                subscript_superscript::simple_superscript_parser(org_use_sub_superscripts.clone()),
                // citation.clone(),
                // export_snippet.clone(),
                footnote_reference::simple_footnote_reference_parser(),
                angle_link.clone(),
                plain_link.clone(),
                link::simple_regular_link_parser(),
                radio_link::simple_radio_link_parser(),
                r#macro.clone(),
                radio_target::simple_radio_target_parser(),
                target.clone(),
                timestamp.clone(),
            )));

            let plain_text_parser_for_table_cell =
                plain_text_parser(non_plain_text_parsers_for_table_cell_lookahead);
            object_in_table_cell.define(choice((
                non_plain_text_parsers_for_table_cell,
                plain_text_parser_for_table_cell,
            )));

            // standard set object: 21
            let standard_set_without_plain_text = Parser::boxed(choice((
                radio_link.clone(),         // 1个
                regular_link.clone(),       // 1个
                independent_object.clone(), // 12个
                radio_target.clone(),       // 1个
                text_markup.clone(),        // 1个
                subscript.clone(),          // 1个
                superscript.clone(),        // 1个
                footnote_reference.clone(), // 1个
                                            // citation.clone(),             // 1个
            )));
            let standard_set_without_plain_text_lookahead = Parser::boxed(choice((
                radio_link::simple_radio_link_parser(),
                link::simple_regular_link_parser(),
                independent_object.clone(), // 12个
                radio_target::simple_radio_target_parser(),
                text_markup::simple_text_markup_parser(),
                subscript_superscript::simple_subscript_parser(org_use_sub_superscripts.clone()),
                subscript_superscript::simple_superscript_parser(org_use_sub_superscripts.clone()),
                footnote_reference::simple_footnote_reference_parser(),
                // citation.clone(),             // 1个
            )));

            let plain_text_for_standard =
                plain_text_parser(standard_set_without_plain_text_lookahead);
            standard_set_object.define(choice((
                standard_set_without_plain_text,
                plain_text_for_standard,
            )));

            // full set object: 23
            let full_set_without_plain_text = Parser::boxed(choice((
                radio_link.clone(),         // 1
                regular_link.clone(),       // 1
                independent_object.clone(), // 12
                radio_target.clone(),       // 1
                text_markup.clone(),        // 1
                subscript.clone(),          // 1
                superscript.clone(),        // 1
                footnote_reference.clone(), // 1
                                            // citation.clone(),             // 1

                                            // table_cell,            // table cell only in table_row of table, DONOT INCLUDE THIS
                                            // citation_reference,
            )));
            let full_set_without_plain_text_lookahead = Parser::boxed(choice((
                radio_link::simple_radio_link_parser(),
                link::simple_regular_link_parser(),
                independent_object.clone(), // 12
                radio_target::simple_radio_target_parser(),
                text_markup::simple_text_markup_parser(),
                subscript_superscript::simple_subscript_parser(org_use_sub_superscripts.clone()),
                subscript_superscript::simple_superscript_parser(org_use_sub_superscripts.clone()),
                footnote_reference::simple_footnote_reference_parser(),
                // citation.clone(),             // 1

                // table_cell,            // table cell only in table_row of table, DONOT INCLUDE THIS
                // citation_reference,
            )));
            let plain_text_for_full = plain_text_parser(full_set_without_plain_text_lookahead);
            full_set_object.define(choice((full_set_without_plain_text, plain_text_for_full)));

            let non_plain_text_parsers_for_keyword = Parser::boxed(choice((
                radio_link.clone(),         // 1个
                regular_link.clone(),       // 1个
                independent_object.clone(), // 12个
                radio_target.clone(),       // 1个
                text_markup.clone(),        // 1个
                subscript.clone(),          // 1个
                superscript.clone(),        // 1个
                                            // citation.clone(),             // 1个
            )));
            let non_plain_text_parsers_for_keyword_lookahead = Parser::boxed(choice((
                radio_link::simple_radio_link_parser(),
                link::simple_regular_link_parser(),
                independent_object.clone(), // 12个
                radio_target::simple_radio_target_parser(),
                text_markup::simple_text_markup_parser(),
                subscript_superscript::simple_subscript_parser(org_use_sub_superscripts.clone()),
                subscript_superscript::simple_superscript_parser(org_use_sub_superscripts.clone()),
                // citation.clone(),             // 1个
            )));
            let plain_text_parser_for_keyword =
                plain_text_parser(non_plain_text_parsers_for_keyword_lookahead);
            object_in_keyword.define(choice((
                non_plain_text_parsers_for_keyword,
                plain_text_parser_for_keyword,
            )));
            (
                full_set_object.boxed(),
                standard_set_object.boxed(),
                minimal_set_object.boxed(),
                object_in_regular_link.boxed(),
                object_in_table_cell.boxed(),
                object_in_keyword.boxed(),
            )
        }

        OrgUseSubSuperscripts::Nil => {
            // minimal set object: 6
            let non_plain_text_for_minimal = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup.clone(),
            )));
            let non_plain_text_for_minimal_lookahead = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup::simple_text_markup_parser(),
            )));

            let plain_text_for_minimal = plain_text_parser(non_plain_text_for_minimal_lookahead);
            minimal_set_object.define(choice((non_plain_text_for_minimal, plain_text_for_minimal)));

            // object allowed in regular link: 13
            let non_plain_text_parsers_for_regular_link = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup.clone(),
                // export_snippet.clone(),
                // inline_babel_call.clone(),
                // inline_src_block.clone(),
                r#macro.clone(),
                // statistics_cookie.clone(),
                angle_link.clone(),
                plain_link.clone(),
            )));
            // object allowed in regular link: 13
            let non_plain_text_parsers_for_regular_link_lookahead = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup::simple_text_markup_parser(),
                // export_snippet.clone(),
                // inline_babel_call.clone(),
                // inline_src_block.clone(),
                r#macro.clone(),
                // statistics_cookie.clone(),
                angle_link.clone(),
                plain_link.clone(),
            )));

            let plain_text_parser_for_regular_link =
                plain_text_parser(non_plain_text_parsers_for_regular_link_lookahead);
            object_in_regular_link.define(choice((
                non_plain_text_parsers_for_regular_link,
                plain_text_parser_for_regular_link,
            )));

            // object allowed in table cell: 17
            let non_plain_text_parsers_for_table_cell = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup.clone(),
                // citation.clone(),
                // export_snippet.clone(),
                footnote_reference.clone(),
                angle_link.clone(),
                plain_link.clone(),
                regular_link.clone(),
                radio_link.clone(),
                r#macro.clone(),
                radio_target.clone(),
                target.clone(),
                timestamp.clone(),
            )));
            let non_plain_text_parsers_for_table_cell_lookahead = Parser::boxed(choice((
                latex_fragment.clone(),
                entity.clone(),
                text_markup::simple_text_markup_parser(),
                // citation.clone(),
                // export_snippet.clone(),
                footnote_reference::simple_footnote_reference_parser(),
                angle_link.clone(),
                plain_link.clone(),
                link::simple_regular_link_parser(),
                radio_link::simple_radio_link_parser(),
                r#macro.clone(),
                radio_target::simple_radio_target_parser(),
                target.clone(),
                timestamp.clone(),
            )));

            let plain_text_parser_for_table_cell =
                plain_text_parser(non_plain_text_parsers_for_table_cell_lookahead);
            object_in_table_cell.define(choice((
                non_plain_text_parsers_for_table_cell,
                plain_text_parser_for_table_cell,
            )));

            // standard set object: 21
            let standard_set_without_plain_text = Parser::boxed(choice((
                radio_link.clone(),         // 1个
                regular_link.clone(),       // 1个
                independent_object.clone(), // 12个
                radio_target.clone(),       // 1个
                text_markup.clone(),        // 1个
                footnote_reference.clone(), // 1个
                                            // citation.clone(),             // 1个
            )));
            let standard_set_without_plain_text_lookahead = Parser::boxed(choice((
                radio_link::simple_radio_link_parser(),
                link::simple_regular_link_parser(),
                independent_object.clone(), // 12个
                radio_target::simple_radio_target_parser(),
                text_markup::simple_text_markup_parser(),
                footnote_reference::simple_footnote_reference_parser(),
                // citation.clone(),             // 1个
            )));

            let plain_text_for_standard =
                plain_text_parser(standard_set_without_plain_text_lookahead);
            standard_set_object.define(choice((
                standard_set_without_plain_text,
                plain_text_for_standard,
            )));

            // full set object: 23
            let full_set_without_plain_text = Parser::boxed(choice((
                radio_link.clone(),         // 1
                regular_link.clone(),       // 1
                independent_object.clone(), // 12
                radio_target.clone(),       // 1
                text_markup.clone(),        // 1
                footnote_reference.clone(), // 1
                                            // citation.clone(),             // 1

                                            // table_cell,            // table cell only in table_row of table, DONOT INCLUDE THIS
                                            // citation_reference,
            )));
            let full_set_without_plain_text_lookahead = Parser::boxed(choice((
                radio_link::simple_radio_link_parser(),
                link::simple_regular_link_parser(),
                independent_object.clone(), // 12
                radio_target::simple_radio_target_parser(),
                text_markup::simple_text_markup_parser(),
                footnote_reference::simple_footnote_reference_parser(),
                // citation.clone(),             // 1

                // table_cell,            // table cell only in table_row of table, DONOT INCLUDE THIS
                // citation_reference,
            )));
            let plain_text_for_full = plain_text_parser(full_set_without_plain_text_lookahead);
            full_set_object.define(choice((full_set_without_plain_text, plain_text_for_full)));

            let non_plain_text_parsers_for_keyword = Parser::boxed(choice((
                radio_link.clone(),         // 1个
                regular_link.clone(),       // 1个
                independent_object.clone(), // 12个
                radio_target.clone(),       // 1个
                text_markup.clone(),        // 1个
                                            // citation.clone(),             // 1个
            )));
            let non_plain_text_parsers_for_keyword_lookahead = Parser::boxed(choice((
                radio_link::simple_radio_link_parser(),
                link::simple_regular_link_parser(),
                independent_object.clone(), // 12个
                radio_target::simple_radio_target_parser(),
                text_markup::simple_text_markup_parser(),
                // citation.clone(),             // 1个
            )));
            let plain_text_parser_for_keyword =
                plain_text_parser(non_plain_text_parsers_for_keyword_lookahead);
            object_in_keyword.define(choice((
                non_plain_text_parsers_for_keyword,
                plain_text_parser_for_keyword,
            )));
            (
                full_set_object.boxed(),
                standard_set_object.boxed(),
                minimal_set_object.boxed(),
                object_in_regular_link.boxed(),
                object_in_table_cell.boxed(),
                object_in_keyword.boxed(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parsers_output;
    use crate::parser::{OrgParser, config::OrgParserConfig};
    use pretty_assertions::assert_eq;

    fn get_objects_string() -> String {
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
    fn test_just_ignore_case() {
        let test_cases = vec![
            r##"\BEGIN{test}"##,
            r##"\begin{test}"##,
            r##"\Begin{test}"##,
            r##"\BeGiN{test}"##,
        ];
        let parser = just_case_insensitive::<()>(r"\BeGiN{test}");
        for case in test_cases {
            assert!(!parser.parse(case).has_errors());
        }
    }

    // #[bench]
    // fn bench_just_ignore_case(b: &mut Bencher) {
    //     let test_cases = vec![
    //         r##"\BEGIN{test}"##,
    //         r##"\begin{test}"##,
    //         r##"\Begin{test}"##,
    //         r##"\BeGiN{test}"##,
    //     ];
    //     let parser = just_case_insensitive::<()>(r"\BeGiN{test}");
    //     b.iter(|| {
    //         for case in &test_cases {
    //             assert!(!parser.parse(case).has_errors());
    //         }
    //     })
    // }

    #[test]
    fn test_blank_line() {
        // let mut state = RollbackState(ParserState::default());
        for input in vec![" \n", "\t\n", "\n", " \t   \n"] {
            assert_eq!(
                blank_line_parser::<()>().parse(input).into_result(),
                Ok(crate::token!(OSK::BlankLine, input))
            );
        }

        for input in vec![" \n "] {
            assert_eq!(
                blank_line_parser::<()>()
                    // .parse_with_state(input, &mut state)
                    .parse(input)
                    .has_errors(),
                true
            );
        }
    }

    #[test]
    fn test_line() {
        // let mut state = RollbackState(ParserState::default());
        let input = "a row\n";
        // let s = line_parser::<()>().parse_with_state(input, &mut state);
        let s = line_parser::<()>().parse(input);
        println!("{:?}", s);
    }

    #[test]
    fn test_minimal_set_object() {
        let minimal_objects_parser = minimal_set_object_parser::<()>(OrgParserConfig::default())
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>();

        assert_eq!(
            get_parsers_output(minimal_objects_parser, &get_objects_string()),
            r##"Root@0..749
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
    Underscore@186..187 "_"
    LeftCurlyBracket@187..188 "{"
    Text@188..191 "i,j"
    RightCurlyBracket@191..192 "}"
  Text@192..209 "\n\n05 supscript: a"
  Superscript@209..211
    Caret@209..210 "^"
    Text@210..211 "3"
  Text@211..749 "\n\n06 plain-text: text ..."
"##
        );
    }

    #[test]
    fn test_standard_set_object() {
        let standard_objects_parser = standard_set_object_parser::<()>(OrgParserConfig::default())
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>();

        assert_eq!(
            get_parsers_output(standard_objects_parser, &get_objects_string()),
            r##"Root@0..749
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
    Underscore@186..187 "_"
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
  Link@527..538
    LeftSquareBracket@527..528 "["
    LinkPath@528..536
      LeftSquareBracket@528..529 "["
      Text@529..535 "target"
      RightSquareBracket@535..536 "]"
    RightSquareBracket@536..537 "]"
    Newline@537..538 "\n"
  Text@538..749 "\n17 statistics-cookie ..."
"##
        );
    }

    #[test]
    fn test_object_in_table_cell() {
        let table_string_with_objects = r##"* table
| object kind           | object value           | supported in table cell |
|-----------------------+------------------------+-------------------------|
| 01 text markup        | *bold*                 | y                       |
| 02 entity             | \alpha                 | y                       |
| 03 latex fragment     | $\sum_{i=1}^{n} i$     | y                       |
| 04 superscript        | a_1                    | y                       |
| 05 subscript          | c^{2}                  | y                       |
| 06 plain text         | just text              | y                       |
| 07 footnote-reference | [fn:1]                 | y                       |
| 08 line-break         | \\                     | n                       |
| 09 timestamp          | <1234-07-31>           | y                       |
| 10 macro              | {{{title}}}            | y                       |
| 11 radio-target       | <<<radio target 011>>> | y                       |
| 12 target             | <<target 012>>         | y                       |
| 13 plain link         | https://foo.bar        | y                       |
| 14 angle link         | <mailto:foo@bar>       | y                       |
| 15 radio link         | radio target 011       | y                       |
| 16 regular link       | [[target 011]]         | y                       |
| 17 statistics-cookie  | todo                   | n                       |
| 18 inline-babel-call  | todo                   | n                       |
| 19 inline-src-block   | todo                   | n                       |
| 20 citation           | todo                   | y                       |
| 21 export-snippet     | todo                   | y                       |
| 22 table-cell         | NA                     | n                       |
| 23 citation-reference | todo                   | n                       |
"##;

        let org_config = OrgParserConfig::default();
        let mut parser = OrgParser::new(org_config);
        let parser_output = parser.parse(&table_string_with_objects);
        let _green_tree = parser_output.green();
        let syntax_tree = parser_output.syntax();

        assert_eq!(
            format!("{syntax_tree:#?}"),
            r##"Document@0..1933
  HeadingSubtree@0..1933
    HeadingRow@0..8
      HeadingRowStars@0..1 "*"
      Whitespace@1..2 " "
      HeadingRowTitle@2..7 "table"
      Newline@7..8 "\n"
    Section@8..1933
      Table@8..1933
        TableStandardRow@8..85
          Pipe@8..9 "|"
          TableCell@9..33
            Text@9..21 " object kind"
            Whitespace@21..32 "           "
            Pipe@32..33 "|"
          TableCell@33..58
            Text@33..46 " object value"
            Whitespace@46..57 "           "
            Pipe@57..58 "|"
          TableCell@58..84
            Text@58..82 " supported in table cell"
            Whitespace@82..83 " "
            Pipe@83..84 "|"
          Newline@84..85 "\n"
        TableRuleRow@85..162
          Pipe@85..86 "|"
          Dash@86..87 "-"
          Text@87..161 "--------------------- ..."
          Newline@161..162 "\n"
        TableStandardRow@162..239
          Pipe@162..163 "|"
          TableCell@163..187
            Text@163..178 " 01 text markup"
            Whitespace@178..186 "        "
            Pipe@186..187 "|"
          TableCell@187..212
            Text@187..188 " "
            Bold@188..194
              Asterisk@188..189 "*"
              Text@189..193 "bold"
              Asterisk@193..194 "*"
            Whitespace@194..211 "                 "
            Pipe@211..212 "|"
          TableCell@212..238
            Text@212..214 " y"
            Whitespace@214..237 "                       "
            Pipe@237..238 "|"
          Newline@238..239 "\n"
        TableStandardRow@239..316
          Pipe@239..240 "|"
          TableCell@240..264
            Text@240..250 " 02 entity"
            Whitespace@250..263 "             "
            Pipe@263..264 "|"
          TableCell@264..289
            Text@264..265 " "
            Entity@265..271
              BackSlash@265..266 "\\"
              EntityName@266..271 "alpha"
            Whitespace@271..288 "                 "
            Pipe@288..289 "|"
          TableCell@289..315
            Text@289..291 " y"
            Whitespace@291..314 "                       "
            Pipe@314..315 "|"
          Newline@315..316 "\n"
        TableStandardRow@316..393
          Pipe@316..317 "|"
          TableCell@317..341
            Text@317..335 " 03 latex fragment"
            Whitespace@335..340 "     "
            Pipe@340..341 "|"
          TableCell@341..366
            Text@341..342 " "
            LatexFragment@342..360
              Dollar@342..343 "$"
              Text@343..359 "\\sum_{i=1}^{n} i"
              Dollar@359..360 "$"
            Whitespace@360..365 "     "
            Pipe@365..366 "|"
          TableCell@366..392
            Text@366..368 " y"
            Whitespace@368..391 "                       "
            Pipe@391..392 "|"
          Newline@392..393 "\n"
        TableStandardRow@393..470
          Pipe@393..394 "|"
          TableCell@394..418
            Text@394..409 " 04 superscript"
            Whitespace@409..417 "        "
            Pipe@417..418 "|"
          TableCell@418..443
            Text@418..420 " a"
            Subscript@420..422
              Underscore@420..421 "_"
              Text@421..422 "1"
            Whitespace@422..442 "                    "
            Pipe@442..443 "|"
          TableCell@443..469
            Text@443..445 " y"
            Whitespace@445..468 "                       "
            Pipe@468..469 "|"
          Newline@469..470 "\n"
        TableStandardRow@470..547
          Pipe@470..471 "|"
          TableCell@471..495
            Text@471..484 " 05 subscript"
            Whitespace@484..494 "          "
            Pipe@494..495 "|"
          TableCell@495..520
            Text@495..497 " c"
            Superscript@497..501
              Caret@497..498 "^"
              LeftCurlyBracket@498..499 "{"
              Text@499..500 "2"
              RightCurlyBracket@500..501 "}"
            Whitespace@501..519 "                  "
            Pipe@519..520 "|"
          TableCell@520..546
            Text@520..522 " y"
            Whitespace@522..545 "                       "
            Pipe@545..546 "|"
          Newline@546..547 "\n"
        TableStandardRow@547..624
          Pipe@547..548 "|"
          TableCell@548..572
            Text@548..562 " 06 plain text"
            Whitespace@562..571 "         "
            Pipe@571..572 "|"
          TableCell@572..597
            Text@572..582 " just text"
            Whitespace@582..596 "              "
            Pipe@596..597 "|"
          TableCell@597..623
            Text@597..599 " y"
            Whitespace@599..622 "                       "
            Pipe@622..623 "|"
          Newline@623..624 "\n"
        TableStandardRow@624..701
          Pipe@624..625 "|"
          TableCell@625..649
            Text@625..647 " 07 footnote-reference"
            Whitespace@647..648 " "
            Pipe@648..649 "|"
          TableCell@649..674
            Text@649..650 " "
            FootnoteReference@650..656
              LeftSquareBracket@650..651 "["
              Text@651..653 "fn"
              Colon@653..654 ":"
              FootnoteReferenceLabel@654..655 "1"
              RightSquareBracket@655..656 "]"
            Whitespace@656..673 "                 "
            Pipe@673..674 "|"
          TableCell@674..700
            Text@674..676 " y"
            Whitespace@676..699 "                       "
            Pipe@699..700 "|"
          Newline@700..701 "\n"
        TableStandardRow@701..778
          Pipe@701..702 "|"
          TableCell@702..726
            Text@702..716 " 08 line-break"
            Whitespace@716..725 "         "
            Pipe@725..726 "|"
          TableCell@726..751
            Text@726..729 " \\\\"
            Whitespace@729..750 "                     "
            Pipe@750..751 "|"
          TableCell@751..777
            Text@751..753 " n"
            Whitespace@753..776 "                       "
            Pipe@776..777 "|"
          Newline@777..778 "\n"
        TableStandardRow@778..855
          Pipe@778..779 "|"
          TableCell@779..803
            Text@779..792 " 09 timestamp"
            Whitespace@792..802 "          "
            Pipe@802..803 "|"
          TableCell@803..828
            Text@803..804 " "
            Timestamp@804..816
              Text@804..816 "<1234-07-31>"
            Whitespace@816..827 "           "
            Pipe@827..828 "|"
          TableCell@828..854
            Text@828..830 " y"
            Whitespace@830..853 "                       "
            Pipe@853..854 "|"
          Newline@854..855 "\n"
        TableStandardRow@855..932
          Pipe@855..856 "|"
          TableCell@856..880
            Text@856..865 " 10 macro"
            Whitespace@865..879 "              "
            Pipe@879..880 "|"
          TableCell@880..905
            Text@880..881 " "
            Macro@881..892
              LeftCurlyBracket3@881..884 "{{{"
              MacroName@884..889 "title"
              RightCurlyBracket3@889..892 "}}}"
            Whitespace@892..904 "            "
            Pipe@904..905 "|"
          TableCell@905..931
            Text@905..907 " y"
            Whitespace@907..930 "                       "
            Pipe@930..931 "|"
          Newline@931..932 "\n"
        TableStandardRow@932..1009
          Pipe@932..933 "|"
          TableCell@933..957
            Text@933..949 " 11 radio-target"
            Whitespace@949..956 "       "
            Pipe@956..957 "|"
          TableCell@957..982
            Text@957..958 " "
            RadioTarget@958..980
              LeftAngleBracket3@958..961 "<<<"
              Text@961..977 "radio target 011"
              RightAngleBracket3@977..980 ">>>"
            Whitespace@980..981 " "
            Pipe@981..982 "|"
          TableCell@982..1008
            Text@982..984 " y"
            Whitespace@984..1007 "                       "
            Pipe@1007..1008 "|"
          Newline@1008..1009 "\n"
        TableStandardRow@1009..1086
          Pipe@1009..1010 "|"
          TableCell@1010..1034
            Text@1010..1020 " 12 target"
            Whitespace@1020..1033 "             "
            Pipe@1033..1034 "|"
          TableCell@1034..1059
            Text@1034..1035 " "
            Target@1035..1049
              LeftAngleBracket2@1035..1037 "<<"
              Text@1037..1047 "target 012"
              RightAngleBracket2@1047..1049 ">>"
            Whitespace@1049..1058 "         "
            Pipe@1058..1059 "|"
          TableCell@1059..1085
            Text@1059..1061 " y"
            Whitespace@1061..1084 "                       "
            Pipe@1084..1085 "|"
          Newline@1085..1086 "\n"
        TableStandardRow@1086..1163
          Pipe@1086..1087 "|"
          TableCell@1087..1111
            Text@1087..1101 " 13 plain link"
            Whitespace@1101..1110 "         "
            Pipe@1110..1111 "|"
          TableCell@1111..1136
            Text@1111..1112 " "
            PlainLink@1112..1127
              Text@1112..1117 "https"
              Colon@1117..1118 ":"
              Text@1118..1127 "//foo.bar"
            Whitespace@1127..1135 "        "
            Pipe@1135..1136 "|"
          TableCell@1136..1162
            Text@1136..1138 " y"
            Whitespace@1138..1161 "                       "
            Pipe@1161..1162 "|"
          Newline@1162..1163 "\n"
        TableStandardRow@1163..1240
          Pipe@1163..1164 "|"
          TableCell@1164..1188
            Text@1164..1178 " 14 angle link"
            Whitespace@1178..1187 "         "
            Pipe@1187..1188 "|"
          TableCell@1188..1213
            Text@1188..1189 " "
            AngleLink@1189..1205
              LeftAngleBracket@1189..1190 "<"
              Text@1190..1196 "mailto"
              Colon@1196..1197 ":"
              Text@1197..1204 "foo@bar"
              RightAngleBracket@1204..1205 ">"
            Whitespace@1205..1212 "       "
            Pipe@1212..1213 "|"
          TableCell@1213..1239
            Text@1213..1215 " y"
            Whitespace@1215..1238 "                       "
            Pipe@1238..1239 "|"
          Newline@1239..1240 "\n"
        TableStandardRow@1240..1317
          Pipe@1240..1241 "|"
          TableCell@1241..1265
            Text@1241..1255 " 15 radio link"
            Whitespace@1255..1264 "         "
            Pipe@1264..1265 "|"
          TableCell@1265..1290
            Text@1265..1266 " "
            RadioLink@1266..1282
              Text@1266..1282 "radio target 011"
            Whitespace@1282..1289 "       "
            Pipe@1289..1290 "|"
          TableCell@1290..1316
            Text@1290..1292 " y"
            Whitespace@1292..1315 "                       "
            Pipe@1315..1316 "|"
          Newline@1316..1317 "\n"
        TableStandardRow@1317..1394
          Pipe@1317..1318 "|"
          TableCell@1318..1342
            Text@1318..1334 " 16 regular link"
            Whitespace@1334..1341 "       "
            Pipe@1341..1342 "|"
          TableCell@1342..1367
            Text@1342..1343 " "
            Link@1343..1357
              LeftSquareBracket@1343..1344 "["
              LinkPath@1344..1356
                LeftSquareBracket@1344..1345 "["
                Text@1345..1355 "target 011"
                RightSquareBracket@1355..1356 "]"
              RightSquareBracket@1356..1357 "]"
            Whitespace@1357..1366 "         "
            Pipe@1366..1367 "|"
          TableCell@1367..1393
            Text@1367..1369 " y"
            Whitespace@1369..1392 "                       "
            Pipe@1392..1393 "|"
          Newline@1393..1394 "\n"
        TableStandardRow@1394..1471
          Pipe@1394..1395 "|"
          TableCell@1395..1419
            Text@1395..1416 " 17 statistics-cookie"
            Whitespace@1416..1418 "  "
            Pipe@1418..1419 "|"
          TableCell@1419..1444
            Text@1419..1424 " todo"
            Whitespace@1424..1443 "                   "
            Pipe@1443..1444 "|"
          TableCell@1444..1470
            Text@1444..1446 " n"
            Whitespace@1446..1469 "                       "
            Pipe@1469..1470 "|"
          Newline@1470..1471 "\n"
        TableStandardRow@1471..1548
          Pipe@1471..1472 "|"
          TableCell@1472..1496
            Text@1472..1493 " 18 inline-babel-call"
            Whitespace@1493..1495 "  "
            Pipe@1495..1496 "|"
          TableCell@1496..1521
            Text@1496..1501 " todo"
            Whitespace@1501..1520 "                   "
            Pipe@1520..1521 "|"
          TableCell@1521..1547
            Text@1521..1523 " n"
            Whitespace@1523..1546 "                       "
            Pipe@1546..1547 "|"
          Newline@1547..1548 "\n"
        TableStandardRow@1548..1625
          Pipe@1548..1549 "|"
          TableCell@1549..1573
            Text@1549..1569 " 19 inline-src-block"
            Whitespace@1569..1572 "   "
            Pipe@1572..1573 "|"
          TableCell@1573..1598
            Text@1573..1578 " todo"
            Whitespace@1578..1597 "                   "
            Pipe@1597..1598 "|"
          TableCell@1598..1624
            Text@1598..1600 " n"
            Whitespace@1600..1623 "                       "
            Pipe@1623..1624 "|"
          Newline@1624..1625 "\n"
        TableStandardRow@1625..1702
          Pipe@1625..1626 "|"
          TableCell@1626..1650
            Text@1626..1638 " 20 citation"
            Whitespace@1638..1649 "           "
            Pipe@1649..1650 "|"
          TableCell@1650..1675
            Text@1650..1655 " todo"
            Whitespace@1655..1674 "                   "
            Pipe@1674..1675 "|"
          TableCell@1675..1701
            Text@1675..1677 " y"
            Whitespace@1677..1700 "                       "
            Pipe@1700..1701 "|"
          Newline@1701..1702 "\n"
        TableStandardRow@1702..1779
          Pipe@1702..1703 "|"
          TableCell@1703..1727
            Text@1703..1721 " 21 export-snippet"
            Whitespace@1721..1726 "     "
            Pipe@1726..1727 "|"
          TableCell@1727..1752
            Text@1727..1732 " todo"
            Whitespace@1732..1751 "                   "
            Pipe@1751..1752 "|"
          TableCell@1752..1778
            Text@1752..1754 " y"
            Whitespace@1754..1777 "                       "
            Pipe@1777..1778 "|"
          Newline@1778..1779 "\n"
        TableStandardRow@1779..1856
          Pipe@1779..1780 "|"
          TableCell@1780..1804
            Text@1780..1794 " 22 table-cell"
            Whitespace@1794..1803 "         "
            Pipe@1803..1804 "|"
          TableCell@1804..1829
            Text@1804..1807 " NA"
            Whitespace@1807..1828 "                     "
            Pipe@1828..1829 "|"
          TableCell@1829..1855
            Text@1829..1831 " n"
            Whitespace@1831..1854 "                       "
            Pipe@1854..1855 "|"
          Newline@1855..1856 "\n"
        TableStandardRow@1856..1933
          Pipe@1856..1857 "|"
          TableCell@1857..1881
            Text@1857..1879 " 23 citation-reference"
            Whitespace@1879..1880 " "
            Pipe@1880..1881 "|"
          TableCell@1881..1906
            Text@1881..1886 " todo"
            Whitespace@1886..1905 "                   "
            Pipe@1905..1906 "|"
          TableCell@1906..1932
            Text@1906..1908 " n"
            Whitespace@1908..1931 "                       "
            Pipe@1931..1932 "|"
          Newline@1932..1933 "\n"
"##
        );
    }
}
