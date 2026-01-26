//! Keyword parser
use crate::compiler::parser::config::OrgParserConfig;
use crate::compiler::parser::object::blank_line_parser;
use crate::compiler::parser::{MyExtra, NT, OSK};
use crate::compiler::parser::{element, object};
use chumsky::prelude::*;
use std::collections::HashSet;
use std::ops::Range;

pub(crate) fn affiliated_keyword_parser_inner<'a, C: 'a>(
    org_element_dual_keywords_parsed: HashSet<String>,
    org_element_dual_keywords_string: HashSet<String>,
    org_element_affiliated_keywords_nondual_string: HashSet<String>,
    value_parser: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let key_optvalue_parsed = object::keyword_ci_parser_v2(org_element_dual_keywords_parsed); // 1
    let key_optvalue_string = object::keyword_ci_parser_v2(org_element_dual_keywords_string); // 1
    let key_nonvalue_string =
        object::keyword_ci_parser_v2(org_element_affiliated_keywords_nondual_string); // 11

    let backend = any()
        .filter(|c: &char| matches!(c, '-' | '_') || c.is_alphanumeric())
        .repeated()
        .at_least(1)
        .to_slice();
    let string_without_nl = none_of(object::CRLF).repeated().at_least(0).to_slice(); // `#+RESULTS:` will report error!! to check

    let mut single_expression = Recursive::declare(); // foo / [foo] / [[[foo]]]
    single_expression.define(
        (none_of("[]\r\n").repeated().at_least(1).to_slice()).or(just("[")
            .then(single_expression.clone().repeated())
            .then(just("]"))
            .to_slice()),
    );
    let optval = single_expression.clone().repeated().at_least(1).to_slice();

    just("#+")
        .then(choice((
            // #+KEY[OPTVAL]: VALUE(string)
            key_optvalue_string
                .then(just("[").then(optval.clone()).then(just("]")).or_not())
                .then(just(":"))
                .then(object::whitespaces())
                .then(string_without_nl)
                .map(|((((key, maybe_lsb_optval_rsb), colon), ws), value)| {
                    let mut children = Vec::with_capacity(7);
                    children.push(crate::node!(
                        OSK::KeywordKey,
                        vec![crate::token!(OSK::Text, key)]
                    ));

                    if let Some(((lsb, optval), rsb)) = maybe_lsb_optval_rsb {
                        children.push(crate::token!(OSK::LeftSquareBracket, lsb));

                        children.push(crate::node!(
                            OSK::KeywordOptvalue,
                            vec![crate::token!(OSK::Text, optval)]
                        ));

                        children.push(crate::token!(OSK::RightSquareBracket, rsb));
                    }

                    children.push(crate::token!(OSK::Colon, colon));
                    if !ws.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, ws));
                    }
                    children.push(crate::node!(
                        OSK::KeywordValue,
                        vec![crate::token!(OSK::Text, value)]
                    ));

                    // e.state().prev_char = value.chars().last();
                    children
                }),
            // #+KEY[OPTVAL]: VALUE(objects)
            key_optvalue_parsed
                .then(just("[").then(optval.clone()).then(just("]")).or_not())
                .then(just(":"))
                .then(object::whitespaces())
                .then(value_parser)
                .map(|((((key, maybe_lsb_optval_rsb), colon), ws), value)| {
                    let mut children = Vec::with_capacity(7);
                    children.push(crate::node!(
                        OSK::KeywordKey,
                        vec![crate::token!(OSK::Text, key)]
                    ));

                    if let Some(((lsb, optval), rsb)) = maybe_lsb_optval_rsb {
                        children.push(crate::token!(OSK::LeftSquareBracket, lsb));

                        children.push(crate::node!(
                            OSK::KeywordOptvalue,
                            vec![crate::token!(OSK::Text, optval)]
                        ));

                        children.push(crate::token!(OSK::RightSquareBracket, rsb));
                    }
                    children.push(crate::token!(OSK::Colon, colon));
                    if !ws.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, ws));
                    }
                    children.push(crate::node!(OSK::KeywordValue, value));
                    children
                }),
            // #+KEY: VALUE(string)
            key_nonvalue_string
                .then(just(":"))
                .then(object::whitespaces())
                .then(string_without_nl)
                .map(|(((key, colon), ws), value)| {
                    let mut children = Vec::with_capacity(4);
                    children.push(crate::node!(
                        OSK::KeywordKey,
                        vec![crate::token!(OSK::Text, key)]
                    ));
                    children.push(crate::token!(OSK::Colon, colon));
                    if !ws.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, ws));
                    }
                    children.push(crate::node!(
                        OSK::KeywordValue,
                        vec![crate::token!(OSK::Text, value)]
                    ));

                    // e.state().prev_char = value.chars().last();
                    children
                }),
            // #+attr_BACKEND: VALUE
            object::just_case_insensitive("attr_")
                .then(backend)
                .to_slice()
                .then(just(":"))
                .then(object::whitespaces())
                .then(string_without_nl)
                .map(|(((attr_backend, colon), ws), value)| {
                    let mut children = Vec::with_capacity(4);

                    children.push(crate::node!(
                        OSK::KeywordKey,
                        vec![crate::token!(OSK::Text, attr_backend)]
                    ));

                    children.push(crate::token!(OSK::Colon, colon));

                    if !ws.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, ws));
                    }

                    children.push(crate::node!(
                        OSK::KeywordValue,
                        vec![crate::token!(OSK::Text, value)]
                    ));

                    // e.state().prev_char = value.chars().last();
                    children
                }),
        )))
        .then(object::newline_or_ending())
        .map(|((hash_plus, others), maybe_newline)| {
            let mut children = Vec::with_capacity(2 + others.len());
            children.push(crate::token!(OSK::HashPlus, hash_plus));
            children.extend(others);
            if let Some(newline) = maybe_newline {
                children.push(crate::token!(OSK::Newline, newline));
                // e.state().prev_char = newline.chars().last();
            }

            crate::node!(OSK::AffiliatedKeyword, children)
        })
        .boxed()
}

// affliated keyword is NOT a element, it's part of some element.
// #+KEY: VALUE(string)
// #+KEY[OPTVAL]: VALUE(string)
// #+KEY[OPTVAL]: VALUE(objects)
// #+attr_BACKEND: VALUE
pub(crate) fn affiliated_keyword_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let string_without_nl = none_of(object::CRLF).repeated().at_least(1).to_slice();
    let objects_parser = object::object_in_keyword_parser(config.clone())
        .repeated()
        .at_least(1)
        .collect::<Vec<NT>>();
    let value_parser = objects_parser.nested_in(string_without_nl);

    affiliated_keyword_parser_inner(
        config.org_element_dual_keywords_parsed(),
        config.org_element_dual_keywords_string(),
        config.org_element_affiliated_keywords_nondual_string(),
        value_parser,
    )
}

// only for lookahead, no object_parser is need
pub(crate) fn simple_affiliated_keyword_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let string_without_nl = none_of(object::CRLF).repeated().at_least(1).to_slice();
    let value_parser = string_without_nl.map(|s| vec![crate::token!(OSK::Text, s)]);

    affiliated_keyword_parser_inner(
        config.org_element_dual_keywords_parsed(),
        config.org_element_dual_keywords_string(),
        config.org_element_affiliated_keywords_nondual_string(),
        value_parser,
    )
    .ignored()
    .to(crate::node!(OSK::AffiliatedKeyword, vec![]))
}

// find last colon(:), all previous chars are `key`, such as "#+key:with:colon: value"
fn key_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, String, MyExtra<'a, C>> + Clone {
    custom::<_, &str, _, MyExtra<'a, C>>(|inp| {
        let remaining = inp.slice_from(std::ops::RangeFrom {
            start: &inp.cursor(),
        });

        let content: String = remaining
            .chars()
            .take_while(|c| !matches!(c, ' ' | '\t' | '\r' | '\n'))
            .collect();

        // last colon
        let last_colon = content.char_indices().rev().find(|(_, c)| matches!(c, ':'));

        let (idx, _) = last_colon.ok_or_else(|| {
            let n_char = content.chars().count();
            Rich::custom(
                SimpleSpan::from(Range {
                    start: *inp.cursor().inner(),
                    end: (inp.cursor().inner() + n_char),
                }),
                format!("keyword must be followd by a colon: '{}'", content),
            )
        })?;

        let key = content.chars().take(idx + 0).collect::<String>();
        for _ in 0..idx + 0 {
            inp.next();
        }
        Ok(key)
    })
}

pub(crate) fn keyword_parser_inner<'a, C: 'a + std::default::Default>(
    config: OrgParserConfig,
    value_parser: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let element_parser_having_affiliated_keywords_for_lookahead = choice((
        element::footnote_definition::simple_footnote_definition_parser(config.clone()),
        // element::list::simple_plain_list_parser(element::item::simple_item_parser()),
        element::plain_list::simple_plain_list_parser(config.clone()),
        element::block::simple_center_block_parser(config.clone()),
        element::block::simple_quote_block_parser(config.clone()),
        element::block::simple_special_block_parser(config.clone()),
        element::block::simple_verse_block_parser(config.clone()),
        element::table::simple_table_parser(config.clone()),
        element::horizontal_rule::horizontal_rule_parser().ignored(),
        element::latex_environment::simple_latex_environment_parser(config.clone()),
        element::block::simple_src_block_parser(config.clone()),
        element::block::simple_export_block_parser(config.clone()),
        element::block::simple_example_block_parser(config.clone()),
        element::block::simple_comment_block_parser(config.clone()),
        element::fixed_width::simple_fixed_width_parser(config.clone()),
        element::paragraph::paragraph_parser_with_at_least_n_affiliated_keywords(
            choice((
                element::horizontal_rule::horizontal_rule_parser().ignored(), // placeholder
            )),
            1,
            config.clone(),
        )
        .ignored(),
    ));

    // PEG: !whitespace any()*
    // last if not :
    let string_without_nl = none_of(object::CRLF).repeated().at_least(0).to_slice();
    // let key_with_objects = object::keyword_ci_parser(&ORG_ELEMENT_KEYWORDS_OPTVALUE_PARSED); // 1
    let key_with_objects = object::keyword_ci_parser_v2(config.org_element_dual_keywords_parsed()); // 1    

    // FIXME: better method? element vs blankline?
    // #+KEY: VALUE(string)
    let p1_part1 = just("#+")
        .then(key_parser())
        .then(just(":"))
        .then(object::whitespaces())
        .then(string_without_nl);

    // part + end()
    // part + \n + end()
    // part + \n + blankline*
    // (part + \n) !(element_with_affiliated_keywords)
    let p1 = choice((
        p1_part1.clone().then(end().to(None)),
        p1_part1
            .clone()
            .then(object::newline().then(end()).to_slice().map(|s| Some(s))),
        p1_part1
            .clone()
            .then(object::newline().map(|c| Some(c)))
            .then_ignore(blank_line_parser().repeated().at_least(1).rewind()),
        p1_part1
            .clone()
            .then(object::newline().map(|c| Some(c)))
            // .map(|s|{println!("dbg: s={s:?}"); s})
            .and_is(
                element_parser_having_affiliated_keywords_for_lookahead
                    .clone()
                    .ignored()
                    // .map(|s|{println!("dbg@and_is: s={s:?}"); s})
                    .not(),
            ),
    ))
    .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
    .map(
        |((((((hash_plus, key), colon), ws), value), nl), blank_lines)| {
            let mut children = Vec::with_capacity(6 + blank_lines.len());

            children.push(crate::token!(OSK::HashPlus, hash_plus));

            children.push(crate::node!(
                OSK::KeywordKey,
                vec![crate::token!(OSK::Text, &key)]
            ));

            children.push(crate::token!(OSK::Colon, colon));

            if !ws.is_empty() {
                children.push(crate::token!(OSK::Whitespace, ws));
            }

            children.push(crate::node!(
                OSK::KeywordValue,
                vec![crate::token!(OSK::Text, &value)]
            ));

            match nl {
                Some(newline) => {
                    children.push(crate::token!(OSK::Newline, newline));
                    // e.state().prev_char = newline.chars().last();
                }
                None => {
                    // e.state().prev_char = value.chars().last();
                }
            }

            if blank_lines.len() > 0 {
                children.extend(blank_lines);
                // e.state().prev_char = Some('\n');
            }

            crate::node!(OSK::Keyword, children)
        },
    );

    // #+KEY: VALUE(objects)
    let p1a_part1 = just("#+")
        .then(key_with_objects)
        .then(just(":"))
        .then(object::whitespaces())
        .then(value_parser);

    let p1a = choice((
        p1a_part1.clone().then(end().to(None)),
        p1a_part1
            .clone()
            .then(just('\n').then(end()).to_slice().to(Some("\n"))),
        p1a_part1.clone().then(just("\n").map(|c| Some(c))).and_is(
            element_parser_having_affiliated_keywords_for_lookahead
                .clone()
                .ignored()
                .not(),
        ), // todo: better use simple_afflitaed keyword?
        p1a_part1
            .clone()
            .then(object::newline_or_ending())
            .then_ignore(blank_line_parser().repeated().at_least(1).rewind()),
    ))
    .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
    .map(
        |((((((hash_plus, key), colon), ws), value), nl), blank_lines)| {
            let mut children = Vec::with_capacity(6 + blank_lines.len());
            children.push(crate::token!(OSK::HashPlus, hash_plus));
            children.push(crate::node!(
                OSK::KeywordKey,
                vec![crate::token!(OSK::Text, &key)]
            ));
            children.push(crate::token!(OSK::Colon, colon));
            if !ws.is_empty() {
                children.push(crate::token!(OSK::Whitespace, ws));
            }
            children.push(crate::node!(OSK::KeywordValue, value));
            match nl {
                Some(newline) => {
                    children.push(crate::token!(OSK::Newline, newline));
                    // e.state().prev_char = newline.chars().last();
                }
                None => {}
            }
            if blank_lines.len() > 0 {
                children.extend(blank_lines);
                // e.state().prev_char = Some('\n');
            }

            crate::node!(OSK::Keyword, children)
        },
    );

    Parser::boxed(choice((p1a, p1)))
}

// element_parser: <element with affiliated word>
pub(crate) fn keyword_parser<'a, C: 'a + std::default::Default>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let string_without_nl = none_of(object::CRLF).repeated().at_least(0).to_slice();
    let objects_parser = object::object_in_keyword_parser(config.clone())
        .repeated()
        .at_least(0)
        .collect::<Vec<NT>>();
    let value_parser = objects_parser.clone().nested_in(string_without_nl);

    keyword_parser_inner(config, value_parser)
}

pub(crate) fn simple_keyword_parser<'a, C: 'a + std::default::Default>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    let string_without_nl = none_of(object::CRLF).repeated().at_least(0).to_slice();
    let value_parser = string_without_nl.map(|s| vec![crate::token!(OSK::Text, s)]);

    keyword_parser_inner(config, value_parser).ignored()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::parser::common::{get_parser_output, get_parsers_output};
    use crate::compiler::parser::config::OrgParserConfig;
    use crate::compiler::parser::element;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_keyword_01() {
        assert_eq!(
            get_parser_output(
                // keyword_parser(element::element_in_keyword_parser::<()>()),
                keyword_parser::<()>(OrgParserConfig::default()),
                r"#+key: value    "
            ),
            r###"Keyword@0..16
  HashPlus@0..2 "#+"
  KeywordKey@2..5
    Text@2..5 "key"
  Colon@5..6 ":"
  Whitespace@6..7 " "
  KeywordValue@7..16
    Text@7..16 "value    "
"###
        );
    }

    #[test]
    fn test_keyword_91() {
        assert_eq!(
            get_parser_output(
                keyword_parser::<()>(OrgParserConfig::default()),
                r"#+title: org test

"
            ),
            r###"Keyword@0..19
  HashPlus@0..2 "#+"
  KeywordKey@2..7
    Text@2..7 "title"
  Colon@7..8 ":"
  Whitespace@8..9 " "
  KeywordValue@9..17
    Text@9..17 "org test"
  Newline@17..18 "\n"
  BlankLine@18..19 "\n"
"###
        );
    }

    #[test]
    fn test_keyword_02() {
        assert_eq!(
            get_parser_output(
                // keyword_parser(element::element_in_keyword_parser::<()>()),
                // keyword_parser::<()>(),
                keyword_parser::<()>(OrgParserConfig::default()),
                r"#+key:has:colons: value    "
            ),
            r###"Keyword@0..27
  HashPlus@0..2 "#+"
  KeywordKey@2..16
    Text@2..16 "key:has:colons"
  Colon@16..17 ":"
  Whitespace@17..18 " "
  KeywordValue@18..27
    Text@18..27 "value    "
"###
        );
    }

    #[test]
    fn test_affliated_keyword_01() {
        assert_eq!(
            get_parser_output(
                element::keyword::affiliated_keyword_parser::<()>(OrgParserConfig::default()),
                r"#+caption: value    "
            ),
            r###"AffiliatedKeyword@0..20
  HashPlus@0..2 "#+"
  KeywordKey@2..9
    Text@2..9 "caption"
  Colon@9..10 ":"
  Whitespace@10..11 " "
  KeywordValue@11..20
    Text@11..20 "value    "
"###
        );
    }

    #[test]
    fn test_affliated_keyword_02() {
        assert_eq!(
            get_parser_output(
                affiliated_keyword_parser::<()>(OrgParserConfig::default()),
                r"#+CAPTION[Short caption]: Longer caption."
            ),
            r###"AffiliatedKeyword@0..41
  HashPlus@0..2 "#+"
  KeywordKey@2..9
    Text@2..9 "CAPTION"
  LeftSquareBracket@9..10 "["
  KeywordOptvalue@10..23
    Text@10..23 "Short caption"
  RightSquareBracket@23..24 "]"
  Colon@24..25 ":"
  Whitespace@25..26 " "
  KeywordValue@26..41
    Text@26..41 "Longer caption."
"###
        );
    }

    #[test]
    fn test_affliated_keyword_03() {
        assert_eq!(
            get_parser_output(
                affiliated_keyword_parser::<()>(OrgParserConfig::default()),
                r"#+attr_html: value"
            ),
            r###"AffiliatedKeyword@0..18
  HashPlus@0..2 "#+"
  KeywordKey@2..11
    Text@2..11 "attr_html"
  Colon@11..12 ":"
  Whitespace@12..13 " "
  KeywordValue@13..18
    Text@13..18 "value"
"###
        );
    }
    #[test]
    fn test_affliated_keyword_04() {
        assert_eq!(
            get_parser_output(
                affiliated_keyword_parser::<()>(OrgParserConfig::default()),
                r"#+CAPTION[Short caption]: *Longer* caption."
            ),
            r###"AffiliatedKeyword@0..43
  HashPlus@0..2 "#+"
  KeywordKey@2..9
    Text@2..9 "CAPTION"
  LeftSquareBracket@9..10 "["
  KeywordOptvalue@10..23
    Text@10..23 "Short caption"
  RightSquareBracket@23..24 "]"
  Colon@24..25 ":"
  Whitespace@25..26 " "
  KeywordValue@26..43
    Bold@26..34
      Asterisk@26..27 "*"
      Text@27..33 "Longer"
      Asterisk@33..34 "*"
    Text@34..43 " caption."
"###
        );
    }

    #[test]
    fn test_affliated_keyword_05() {
        assert_eq!(
            get_parser_output(
                affiliated_keyword_parser::<()>(OrgParserConfig::default()),
                r"#+caption:value: value    "
            ),
            r###"AffiliatedKeyword@0..26
  HashPlus@0..2 "#+"
  KeywordKey@2..9
    Text@2..9 "caption"
  Colon@9..10 ":"
  KeywordValue@10..26
    Text@10..26 "value: value    "
"###
        );
    }

    #[test]
    fn test_affliated_keyword_06() {
        let input = r##"#+caption: export block test
#+begin_export html
<span style="color:green;">hello org</span>
#+end_export
"##;

        assert_eq!(
            get_parsers_output(
                element::elements_parser::<()>(OrgParserConfig::default()),
                input
            ),
            r###"Root@0..106
  ExportBlock@0..106
    AffiliatedKeyword@0..29
      HashPlus@0..2 "#+"
      KeywordKey@2..9
        Text@2..9 "caption"
      Colon@9..10 ":"
      Whitespace@10..11 " "
      KeywordValue@11..28
        Text@11..28 "export block test"
      Newline@28..29 "\n"
    BlockBegin@29..49
      Text@29..37 "#+begin_"
      Text@37..43 "export"
      Whitespace@43..44 " "
      Text@44..48 "html"
      Newline@48..49 "\n"
    BlockContent@49..93
      Text@49..93 "<span style=\"color:gr ..."
    BlockEnd@93..106
      Text@93..99 "#+end_"
      Text@99..105 "export"
      Newline@105..106 "\n"
"###,
            "<affiliated keyword> is immediately preceding a <export block>"
        );
    }

    #[test]
    fn test_affliated_keyword_07() {
        let input = r##"#+caption: export block test

#+begin_export html
<span style="color:green;">hello org</span>
#+end_export
"##;

        assert_eq!(
            get_parsers_output(
                element::elements_parser::<()>(OrgParserConfig::default()),
                input
            ),
            r###"Root@0..107
  Keyword@0..30
    HashPlus@0..2 "#+"
    KeywordKey@2..9
      Text@2..9 "caption"
    Colon@9..10 ":"
    Whitespace@10..11 " "
    KeywordValue@11..28
      Text@11..28 "export block test"
    Newline@28..29 "\n"
    BlankLine@29..30 "\n"
  ExportBlock@30..107
    BlockBegin@30..50
      Text@30..38 "#+begin_"
      Text@38..44 "export"
      Whitespace@44..45 " "
      Text@45..49 "html"
      Newline@49..50 "\n"
    BlockContent@50..94
      Text@50..94 "<span style=\"color:gr ..."
    BlockEnd@94..107
      Text@94..100 "#+end_"
      Text@100..106 "export"
      Newline@106..107 "\n"
"###,
            "<affiliated keyword> should be immediately preceding a valid element, or it will be parsed as <keyword>"
        );
    }

    #[test]
    fn test_affliated_keyword_08() {
        let input = r##"#+caption: export block test
a paragraph
"##;

        assert_eq!(
            get_parsers_output(
                element::elements_parser::<()>(OrgParserConfig::default()),
                input
            ),
            r###"Root@0..41
  Paragraph@0..41
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
"###,
            "<affiliated keyword> is immediately preceding a <paragraph>"
        );
    }

    #[test]
    fn test_affliated_keyword_09() {
        let input = r##"#+caption: export block test
#+key: value
a paragraph
"##;

        assert_eq!(
            get_parsers_output(
                element::elements_parser::<()>(OrgParserConfig::default()),
                input
            ),
            r###"Root@0..54
  Keyword@0..29
    HashPlus@0..2 "#+"
    KeywordKey@2..9
      Text@2..9 "caption"
    Colon@9..10 ":"
    Whitespace@10..11 " "
    KeywordValue@11..28
      Text@11..28 "export block test"
    Newline@28..29 "\n"
  Keyword@29..42
    HashPlus@29..31 "#+"
    KeywordKey@31..34
      Text@31..34 "key"
    Colon@34..35 ":"
    Whitespace@35..36 " "
    KeywordValue@36..41
      Text@36..41 "value"
    Newline@41..42 "\n"
  Paragraph@42..54
    Text@42..54 "a paragraph\n"
"###,
            "<keyword> is immediately preceding a <paragraph>"
        );
    }
}
