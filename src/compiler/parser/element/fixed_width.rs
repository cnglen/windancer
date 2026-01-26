//! Fixed width parser
use crate::compiler::parser::config::OrgParserConfig;
use crate::compiler::parser::object::whitespaces_g1;
use crate::compiler::parser::{MyExtra, NT, OSK};
use crate::compiler::parser::{element, object};
use chumsky::prelude::*;

pub(crate) fn fixed_width_parser_inner<'a, C: 'a>(
    affiliated_keywords_parser: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let fixed_width_line = object::whitespaces()
        .then(just(":"))
        .then(
            whitespaces_g1()
                .then(none_of(object::CRLF).repeated())
                .to_slice()
                .or_not(),
        )
        .then(object::newline_or_ending())
        .map(
            |(((ws, colon), maybe_content), eol_or_eof): (((&str, _), _), _)| {
                let mut children = Vec::with_capacity(4);

                if !ws.is_empty() {
                    children.push(crate::token!(OSK::Whitespace, ws));
                }

                children.push(crate::token!(OSK::Colon, colon));

                if let Some(content) = maybe_content {
                    children.push(crate::token!(OSK::Text, content));
                }

                if let Some(newline) = eol_or_eof {
                    children.push(crate::token!(OSK::Newline, newline));
                }

                crate::node!(OSK::FixedWidthLine, children)
            },
        );

    let blank_lines = object::blank_line_parser().repeated().collect::<Vec<_>>();

    affiliated_keywords_parser
        .then(fixed_width_line.repeated().at_least(1).collect::<Vec<_>>())
        .then(blank_lines)
        .map(|((keywords, lines), blank_lines)| {
            let mut children = Vec::with_capacity(keywords.len() + lines.len() + blank_lines.len());
            children.extend(keywords);
            children.extend(lines);
            children.extend(blank_lines);

            crate::node!(OSK::FixedWidth, children)
        })
        .boxed()
}

pub(crate) fn fixed_width_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    // let affiliated_keywords_parser = element::keyword::affiliated_keyword_parser()
    //     .repeated()
    //     .collect::<Vec<_>>();

    let affiliated_keywords_parser = element::keyword::affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    fixed_width_parser_inner(affiliated_keywords_parser)
}

pub(crate) fn simple_fixed_width_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    let affiliated_keywords_parser = element::keyword::simple_affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    fixed_width_parser_inner(affiliated_keywords_parser).ignored()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::parser::common::get_parser_output;
    use crate::compiler::parser::config::OrgParserConfig;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_fixed_width_01() {
        let input = r##": this is a
: fixed width area
"##;
        let parser = fixed_width_parser::<()>(OrgParserConfig::default());
        assert_eq!(
            get_parser_output(parser, input),
            r##"FixedWidth@0..31
  FixedWidthLine@0..12
    Colon@0..1 ":"
    Text@1..11 " this is a"
    Newline@11..12 "\n"
  FixedWidthLine@12..31
    Colon@12..13 ":"
    Text@13..30 " fixed width area"
    Newline@30..31 "\n"
"##
        );
    }

    #[test]
    fn test_fixed_width_02() {
        let input = r##": this is a
   : foo
: fixed width area
:
: 
: a   



"##;
        let parser = fixed_width_parser::<()>(OrgParserConfig::default());
        assert_eq!(
            get_parser_output(parser, input),
            r##"FixedWidth@0..55
  FixedWidthLine@0..12
    Colon@0..1 ":"
    Text@1..11 " this is a"
    Newline@11..12 "\n"
  FixedWidthLine@12..21
    Whitespace@12..15 "   "
    Colon@15..16 ":"
    Text@16..20 " foo"
    Newline@20..21 "\n"
  FixedWidthLine@21..40
    Colon@21..22 ":"
    Text@22..39 " fixed width area"
    Newline@39..40 "\n"
  FixedWidthLine@40..42
    Colon@40..41 ":"
    Newline@41..42 "\n"
  FixedWidthLine@42..45
    Colon@42..43 ":"
    Text@43..44 " "
    Newline@44..45 "\n"
  FixedWidthLine@45..52
    Colon@45..46 ":"
    Text@46..51 " a   "
    Newline@51..52 "\n"
  BlankLine@52..53 "\n"
  BlankLine@53..54 "\n"
  BlankLine@54..55 "\n"
"##
        );
    }

    #[test]
    #[should_panic]
    fn test_fixed_width_03() {
        let input = r##": this is a
:bad
"##;
        let parser = fixed_width_parser::<()>(OrgParserConfig::default());
        get_parser_output(parser, input);
    }
}
