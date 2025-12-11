//! Table parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, element, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::rc::Rc;

fn latex_environment_begin_row_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    object::whitespaces()
        .then(object::just_case_insensitive(r##"\BEGIN{"##))
        .then(
            // name
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '*')
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .then(just("}"))
        .then(object::whitespaces())
        .then(just("\n"))
        .validate(|(((((ws1, begin), name), rcurly), ws2), nl), e, _emitter| {
            // e.state().latex_env_name = name.clone().to_uppercase(); // update state
            e.state().latex_env_name = Rc::from(name.clone().to_uppercase()); // update state

            (ws1, begin, name, rcurly, ws2, nl)
        })
        .map(|(ws1, begin, name, rcurly, ws2, nl)| {
            let mut children = vec![];

            if ws1.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &begin[0..begin.len() - 1],
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftCurlyBracket.into(),
                &begin[begin.len() - 1..],
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &name.to_lowercase(),
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightCurlyBracket.into(),
                rcurly,
            )));

            if ws2.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws2,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Newline.into(),
                &nl,
            )));

            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::LatexEnvironmentBegin.into(),
                children,
            ))
        })
}

fn latex_environment_end_row_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    object::whitespaces()
        .then(object::just_case_insensitive(r##"\END{"##))
        .then(
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric() || *c == '*')
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        // .then(latex_environment_end_row_name_parser())
        .then(just("}"))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .try_map_with(|(((((ws1, end), name), rcurly), ws2), maybe_nl), e| {
            if e.state().latex_env_name.to_uppercase() != name.to_uppercase() {
                Err(Rich::custom(
                    e.span(),
                    format!(
                        "latex env name mismatched {} != {}",
                        e.state().latex_env_name,
                        name
                    ),
                ))
            } else {
                Ok((ws1, end, name, rcurly, ws2, maybe_nl))
            }
        })
        .map(|(ws1, end, name, rcurly, ws2, maybe_nl)| {
            let mut children = vec![];

            if ws1.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &end[0..end.len() - 1],
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftCurlyBracket.into(),
                &end[end.len() - 1..],
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &name.to_lowercase(),
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightCurlyBracket.into(),
                rcurly,
            )));

            if ws2.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws2,
                )));
            }

            match maybe_nl {
                Some(nl) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Newline.into(),
                        &nl,
                    )));
                }
                None => {}
            }

            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::LatexEnvironmentEnd.into(),
                children,
            ))
        })
}

pub(crate) fn latex_environment_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let affiliated_keywords = element::keyword::affiliated_keyword_parser()
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .then(latex_environment_begin_row_parser())
        .then(
            any()
                .and_is(latex_environment_end_row_parser().not())
                .repeated()
                .collect::<String>(),
        )
        .then(latex_environment_end_row_parser())
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(
            |((((keywords, begin_row), content), end_row), blank_lines), e| {
                let mut children = vec![];

                for keyword in keywords {
                    children.push(keyword);
                }

                children.push(begin_row);

                if content.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &content,
                    )));
                }
                children.push(end_row);
                for bl in blank_lines {
                    children.push(NodeOrToken::Token(bl));
                }
                // e.state().latex_env_name = String::new(); // reset state
                e.state().latex_env_name = Rc::from(""); // reset state                
                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::LatexEnvironment.into(),
                    children,
                ))
            },
        )
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parser_output;
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

        let parser = element::latex_environment::latex_environment_parser();
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

        let parser = element::latex_environment::latex_environment_parser();
        assert_eq!(get_parser_output(parser, input), expected_output);
    }
}
