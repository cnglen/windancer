//! inline source block
use chumsky::prelude::*;

use crate::compiler::parser::{MyExtra, NT, OSK, object};

// src_LANG {}
// src_LANG [HEADER] {}
// src_LANG [] {}
// PEG: source_src_block <- "src_" LANG ("[" HEADERS? "]")? "{" BODY "}"
pub(crate) fn inline_source_block_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let mut headers_single_expression = Recursive::declare(); // foo / [foo] / [[[foo]]]
    headers_single_expression.define(choice((
        none_of("[]\r\n").repeated().at_least(1).to_slice(),
        headers_single_expression
            .clone()
            .repeated()
            .delimited_by(just('['), just(']'))
            .to_slice(),
    )));
    let headers = headers_single_expression.repeated().at_least(0).to_slice();

    let mut body_single_expression = Recursive::declare(); // foo / {foo} / {{{foo}}}
    body_single_expression.define(choice((
        none_of("{}\r\n").repeated().at_least(1).to_slice(),
        body_single_expression
            .clone()
            .repeated()
            .delimited_by(just('{'), just('}'))
            .to_slice(),
    )));
    let body = body_single_expression.repeated().at_least(1).to_slice();

    object::prev_valid_parser(|c| c.map_or(true, |e| e.is_whitespace()))
        .ignore_then(group((
            just("src_"),
            none_of(" {[\t").repeated().at_least(1).to_slice(),
            headers.delimited_by(just('['), just(']')).or_not(),
            body.delimited_by(just('{'), just('}')),
        )))
        .map(|(src_underscore, lang, maybe_headers, body)| {
            let mut children = Vec::with_capacity(4);
            children.push(crate::token!(OSK::Text, src_underscore));
            children.push(crate::token!(OSK::InlineSourceBlockLang, lang));

            if let Some(headers) = maybe_headers {
                children.push(crate::token!(OSK::LeftSquareBracket, "["));
                if !headers.is_empty() {
                    children.push(crate::token!(OSK::InlineSourceBlockHeaders, headers));
                }
                children.push(crate::token!(OSK::RightSquareBracket, "]"));
            }
            children.push(crate::token!(OSK::LeftCurlyBracket, "{"));
            children.push(crate::token!(OSK::InlineSourceBlockBody, body));
            children.push(crate::token!(OSK::RightCurlyBracket, "}"));

            crate::node!(OSK::InlineSourceBlock, children)
        })
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::compiler::parser::common::{get_parser_output, get_parsers_output};
    use crate::compiler::parser::config::OrgParserConfig;
    use crate::compiler::parser::object;

    #[test]
    fn test_inline_source_block_01_without_header() {
        let input = r##"src_python{print("hello");}"##;
        let expected_output = r##"InlineSourceBlock@0..27
  Text@0..4 "src_"
  InlineSourceBlockLang@4..10 "python"
  LeftCurlyBracket@10..11 "{"
  InlineSourceBlockBody@11..26 "print(\"hello\");"
  RightCurlyBracket@26..27 "}"
"##;
        assert_eq!(
            get_parser_output(inline_source_block_parser::<()>(), input),
            expected_output,
        );
    }

    #[test]
    fn test_inline_source_block_01_empty_header() {
        let input = r##"src_python[]{print("hello");}"##;
        let expected_output = r##"InlineSourceBlock@0..29
  Text@0..4 "src_"
  InlineSourceBlockLang@4..10 "python"
  LeftSquareBracket@10..11 "["
  RightSquareBracket@11..12 "]"
  LeftCurlyBracket@12..13 "{"
  InlineSourceBlockBody@13..28 "print(\"hello\");"
  RightCurlyBracket@28..29 "}"
"##;
        assert_eq!(
            get_parser_output(inline_source_block_parser::<()>(), input),
            expected_output,
        );
    }

    #[test]
    fn test_inline_source_block_02a() {
        let input = r##"
src_python{print("hello");}"##;
        let expected_output = r##"Root@0..28
  Text@0..1 "\n"
  InlineSourceBlock@1..28
    Text@1..5 "src_"
    InlineSourceBlockLang@5..11 "python"
    LeftCurlyBracket@11..12 "{"
    InlineSourceBlockBody@12..27 "print(\"hello\");"
    RightCurlyBracket@27..28 "}"
"##;
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(OrgParserConfig::default()),
                input
            ),
            expected_output,
        );
    }

    #[test]
    fn test_inline_source_block_02b() {
        let input = r##"asrc_python{print("hello");}"##;
        let expected_output = r##"Root@0..28
  Text@0..4 "asrc"
  Subscript@4..11
    Underscore@4..5 "_"
    Text@5..11 "python"
  Text@11..28 "{print(\"hello\");}"
"##;
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(OrgParserConfig::default()),
                input
            ),
            expected_output,
        );
    }

    #[test]
    fn test_inline_source_block_03() {
        let input = r##"src_cpp[:includes <iostream>]{std::cout<< "hi cpp" << std::endl;}"##;
        let expected_output = r##"InlineSourceBlock@0..65
  Text@0..4 "src_"
  InlineSourceBlockLang@4..7 "cpp"
  LeftSquareBracket@7..8 "["
  InlineSourceBlockHeaders@8..28 ":includes <iostream>"
  RightSquareBracket@28..29 "]"
  LeftCurlyBracket@29..30 "{"
  InlineSourceBlockBody@30..64 "std::cout<< \"hi cpp\"  ..."
  RightCurlyBracket@64..65 "}"
"##;
        assert_eq!(
            get_parser_output(inline_source_block_parser::<()>(), input),
            expected_output,
        );
    }
}
