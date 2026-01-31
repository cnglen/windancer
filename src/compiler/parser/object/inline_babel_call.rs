//! inline babel call
use chumsky::prelude::*;

use crate::compiler::parser::{MyExtra, NT, OSK, object};

// PEG: inline_babel_call <- "call_" NAME ("[" HEADER1 "]")? "(" ARGUMENTS ")" ("[" HEADER2 "]")?
pub(crate) fn inline_babel_call_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let mut header_single_expression = Recursive::declare(); // foo / [foo] / [[[foo]]]
    header_single_expression.define(choice((
        none_of("[]\r\n").repeated().at_least(1).to_slice(),
        header_single_expression
            .clone()
            .repeated()
            .delimited_by(just('['), just(']'))
            .to_slice(),
    )));
    let header = header_single_expression.repeated().at_least(1).to_slice();

    let mut arguments_single_expression = Recursive::declare(); // foo / {foo} / {{{foo}}}
    arguments_single_expression.define(choice((
        none_of("()\r\n").repeated().at_least(1).to_slice(),
        arguments_single_expression
            .clone()
            .repeated()
            .delimited_by(just('('), just(')'))
            .to_slice(),
    )));
    let arguments = arguments_single_expression
        .repeated()
        .at_least(1)
        .to_slice();

    object::prev_valid_parser(|c| c.map_or(true, |e| e.is_whitespace()))
        .ignore_then(group((
            just("call_"),
            none_of(" []()\t").repeated().at_least(1).to_slice(),
            header.clone().delimited_by(just('['), just(']')).or_not(),
            arguments.delimited_by(just('('), just(')')),
            header.clone().delimited_by(just('['), just(']')).or_not(),
        )))
        .map(
            |(call_underscore, name, maybe_header1, arguments, maybe_header2)| {
                let mut children = Vec::with_capacity(4);
                children.push(crate::token!(OSK::Text, call_underscore));
                children.push(crate::token!(OSK::InlineBabelCallName, name));

                if let Some(header1) = maybe_header1 {
                    children.push(crate::token!(OSK::LeftSquareBracket, "["));
                    children.push(crate::token!(OSK::InlineBabelCallHeader1, header1));
                    children.push(crate::token!(OSK::RightSquareBracket, "]"));
                }
                children.push(crate::token!(OSK::LeftRoundBracket, "("));
                children.push(crate::token!(OSK::InlineBabelCallArguments, arguments));
                children.push(crate::token!(OSK::RightRoundBracket, ")"));

                if let Some(header2) = maybe_header2 {
                    children.push(crate::token!(OSK::LeftSquareBracket, "["));
                    children.push(crate::token!(OSK::InlineBabelCallHeader2, header2));
                    children.push(crate::token!(OSK::RightSquareBracket, "]"));
                }

                crate::node!(OSK::InlineBabelCall, children)
            },
        )
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::compiler::parser::common::{get_parser_output, get_parsers_output};
    use crate::compiler::parser::config::OrgParserConfig;
    use crate::compiler::parser::object;

    #[test]
    fn test_inline_babel_call_01() {
        let input = r##"call_hello(arguments)"##;
        let expected_output = r##"InlineBabelCall@0..21
  Text@0..5 "call_"
  InlineBabelCallName@5..10 "hello"
  LeftRoundBracket@10..11 "("
  InlineBabelCallArguments@11..20 "arguments"
  RightRoundBracket@20..21 ")"
"##;
        assert_eq!(
            get_parser_output(inline_babel_call_parser::<()>(), input),
            expected_output,
        );
    }

    #[test]
    fn test_inline_babel_call_02a() {
        let input = r##"
call_hello(arguments)"##;
        let expected_output = r##"Root@0..22
  Text@0..1 "\n"
  InlineBabelCall@1..22
    Text@1..6 "call_"
    InlineBabelCallName@6..11 "hello"
    LeftRoundBracket@11..12 "("
    InlineBabelCallArguments@12..21 "arguments"
    RightRoundBracket@21..22 ")"
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
    fn test_inline_babel_call_02b() {
        let input = r##"acall_hello(arguments)"##;
        let expected_output = r##"Root@0..22
  Text@0..5 "acall"
  Subscript@5..11
    Underscore@5..6 "_"
    Text@6..11 "hello"
  Text@11..22 "(arguments)"
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
    fn test_inline_babel_call_03() {
        let input = r##"call_NAME[HEADER1](ARGUMENTS)[HEADER2]"##;
        let expected_output = r##"InlineBabelCall@0..38
  Text@0..5 "call_"
  InlineBabelCallName@5..9 "NAME"
  LeftSquareBracket@9..10 "["
  InlineBabelCallHeader1@10..17 "HEADER1"
  RightSquareBracket@17..18 "]"
  LeftRoundBracket@18..19 "("
  InlineBabelCallArguments@19..28 "ARGUMENTS"
  RightRoundBracket@28..29 ")"
  LeftSquareBracket@29..30 "["
  InlineBabelCallHeader2@30..37 "HEADER2"
  RightSquareBracket@37..38 "]"
"##;
        assert_eq!(
            get_parser_output(inline_babel_call_parser::<()>(), input),
            expected_output,
        );
    }

    #[test]
    fn test_inline_babel_call_04() {
        let input = r##"call_NAME[HEADER1](ARGUMENTS)"##;
        let expected_output = r##"InlineBabelCall@0..29
  Text@0..5 "call_"
  InlineBabelCallName@5..9 "NAME"
  LeftSquareBracket@9..10 "["
  InlineBabelCallHeader1@10..17 "HEADER1"
  RightSquareBracket@17..18 "]"
  LeftRoundBracket@18..19 "("
  InlineBabelCallArguments@19..28 "ARGUMENTS"
  RightRoundBracket@28..29 ")"
"##;
        assert_eq!(
            get_parser_output(inline_babel_call_parser::<()>(), input),
            expected_output,
        );
    }

    #[test]
    fn test_inline_babel_call_05() {
        let input = r##"call_NAME(ARGUMENTS)[HEADER2]"##;
        let expected_output = r##"InlineBabelCall@0..29
  Text@0..5 "call_"
  InlineBabelCallName@5..9 "NAME"
  LeftRoundBracket@9..10 "("
  InlineBabelCallArguments@10..19 "ARGUMENTS"
  RightRoundBracket@19..20 ")"
  LeftSquareBracket@20..21 "["
  InlineBabelCallHeader2@21..28 "HEADER2"
  RightSquareBracket@28..29 "]"
"##;
        assert_eq!(
            get_parser_output(inline_babel_call_parser::<()>(), input),
            expected_output,
        );
    }
}
