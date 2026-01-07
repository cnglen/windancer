//! latex environment parser
use crate::parser::config::OrgParserConfig;
use crate::parser::{MyExtra, NT, OSK};
use crate::parser::{element, object};
use chumsky::prelude::*;

fn latex_environment_parser_inner<'a, C: 'a>(
    affiliated_keywords_parser: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let begin_row = object::whitespaces()
        .then(object::just_case_insensitive(r##"\BEGIN{"##))
        .then(
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '*')
                .repeated()
                .at_least(1)
                .to_slice(),
        )
        .then(just("}"))
        .then(object::whitespaces())
        .then(object::newline())
        .map(
            |(
                (
                    (((begin_whitespaces1, begin_left_curly), begin_name), begin_right_curly),
                    begin_whitespaces2,
                ),
                begin_newline,
            )| {
                (
                    begin_whitespaces1,
                    begin_left_curly,
                    begin_name,
                    begin_right_curly,
                    begin_whitespaces2,
                    begin_newline,
                )
            },
        );

    let end_row = object::whitespaces()
        .then(object::just_case_insensitive(r##"\END{"##))
        .then(
            just("").configure(|cfg, ctx: &(&str, &str, &str, &str, &str, &str)| cfg.seq((*ctx).2)),
        )
        .then(just("}"))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .map(
            |(
                (
                    (((end_whitespaces1, end_left_curly), end_name), end_right_curly),
                    end_whitespaces2,
                ),
                end_maybe_newline,
            )| {
                (
                    end_whitespaces1,
                    end_left_curly,
                    end_name,
                    end_right_curly,
                    end_whitespaces2,
                    end_maybe_newline,
                )
            },
        );

    let contents = any().and_is(end_row.ignored().not()).repeated().to_slice();

    affiliated_keywords_parser
        .then(begin_row.then_with_ctx(contents.then(end_row)))
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(
            |(
                (
                    keywords,
                    (
                        (
                            begin_whitespaces1,
                            begin_left_curly,
                            begin_name,
                            begin_right_curly,
                            begin_whitespaces2,
                            begin_newline,
                        ),
                        (
                            contents,
                            (
                                end_whitespaces1,
                                end_left_curly,
                                end_name,
                                end_right_curly,
                                end_whitespaces2,
                                end_maybe_newline,
                            ),
                        ),
                    ),
                ),
                blank_lines,
            )| {
                let mut children = Vec::with_capacity(3 + keywords.len() + blank_lines.len());
                children.extend(keywords);

                let begin_row_node = {
                    let mut children = Vec::with_capacity(7);
                    if !begin_whitespaces1.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, begin_whitespaces1));
                    }
                    children.push(crate::token!(
                        OSK::Text,
                        &begin_left_curly[0..begin_left_curly.len() - 1]
                    ));
                    children.push(crate::token!(
                        OSK::LeftCurlyBracket,
                        &begin_left_curly[begin_left_curly.len() - 1..]
                    ));
                    children.push(crate::token!(OSK::Text, begin_name));
                    children.push(crate::token!(OSK::RightCurlyBracket, begin_right_curly));
                    if !begin_whitespaces2.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, begin_whitespaces2));
                    }
                    children.push(crate::token!(OSK::Newline, begin_newline));

                    crate::node!(OSK::LatexEnvironmentBegin, children)
                };
                children.push(begin_row_node);

                if !contents.is_empty() {
                    children.push(crate::token!(OSK::Text, contents));
                }

                let end_row_node = {
                    let mut children = Vec::with_capacity(7);
                    if !end_whitespaces1.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, end_whitespaces1));
                    }
                    children.push(crate::token!(
                        OSK::Text,
                        &end_left_curly[0..end_left_curly.len() - 1]
                    ));
                    children.push(crate::token!(
                        OSK::LeftCurlyBracket,
                        &end_left_curly[end_left_curly.len() - 1..]
                    ));
                    children.push(crate::token!(OSK::Text, end_name));
                    children.push(crate::token!(OSK::RightCurlyBracket, end_right_curly));
                    if !end_whitespaces2.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, end_whitespaces2));
                    }
                    if let Some(newline) = end_maybe_newline {
                        children.push(crate::token!(OSK::Newline, newline));
                    }

                    crate::node!(OSK::LatexEnvironmentEnd, children)
                };
                children.push(end_row_node);

                children.extend(blank_lines);

                crate::node!(OSK::LatexEnvironment, children)
            },
        )
        .boxed()
}

pub(crate) fn latex_environment_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    // let affiliated_keywords_parser = element::keyword::affiliated_keyword_parser()
    //     .repeated()
    //     .collect::<Vec<_>>();
    let affiliated_keywords_parser = element::keyword::affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    latex_environment_parser_inner(affiliated_keywords_parser)
}

pub(crate) fn simple_latex_environment_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    let affiliated_keywords_parser = element::keyword::simple_affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    latex_environment_parser_inner(affiliated_keywords_parser).ignored()
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parser_output;
    use crate::parser::config::OrgParserConfig;
    use crate::parser::element;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_latex_environment_01() {
        let input = r##"\begin{align*}
  2x - 5y &= 8 \\
  3x + 9y &= -12
  \end{align*}
"##;

        let expected_output = r##"LatexEnvironment@0..65
  LatexEnvironmentBegin@0..15
    Text@0..6 "\\begin"
    LeftCurlyBracket@6..7 "{"
    Text@7..13 "align*"
    RightCurlyBracket@13..14 "}"
    Newline@14..15 "\n"
  Text@15..50 "  2x - 5y &= 8 \\\\\n  3 ..."
  LatexEnvironmentEnd@50..65
    Whitespace@50..52 "  "
    Text@52..56 "\\end"
    LeftCurlyBracket@56..57 "{"
    Text@57..63 "align*"
    RightCurlyBracket@63..64 "}"
    Newline@64..65 "\n"
"##;

        let parser =
            element::latex_environment::latex_environment_parser::<()>(OrgParserConfig::default());
        assert_eq!(get_parser_output(parser, input), expected_output);
    }

    #[test]
    fn test_latex_environment_02() {
        let input = r##"#+caption: affiliated keyword in latex environment
  \begin{align*}
  2x - 5y &= 8 \\
  3x + 9y &= -12
  \end{align*}
"##;

        let expected_output = r##"LatexEnvironment@0..118
  AffiliatedKeyword@0..51
    HashPlus@0..2 "#+"
    KeywordKey@2..9
      Text@2..9 "caption"
    Colon@9..10 ":"
    Whitespace@10..11 " "
    KeywordValue@11..50
      Text@11..50 "affiliated keyword in ..."
    Newline@50..51 "\n"
  LatexEnvironmentBegin@51..68
    Whitespace@51..53 "  "
    Text@53..59 "\\begin"
    LeftCurlyBracket@59..60 "{"
    Text@60..66 "align*"
    RightCurlyBracket@66..67 "}"
    Newline@67..68 "\n"
  Text@68..103 "  2x - 5y &= 8 \\\\\n  3 ..."
  LatexEnvironmentEnd@103..118
    Whitespace@103..105 "  "
    Text@105..109 "\\end"
    LeftCurlyBracket@109..110 "{"
    Text@110..116 "align*"
    RightCurlyBracket@116..117 "}"
    Newline@117..118 "\n"
"##;

        let parser =
            element::latex_environment::latex_environment_parser::<()>(OrgParserConfig::default());
        assert_eq!(get_parser_output(parser, input), expected_output);
    }
}
