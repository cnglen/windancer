//! Table parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

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
            e.state().latex_env_name = name.clone().to_uppercase(); // update state
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
    latex_environment_begin_row_parser()
        .then(
            any()
                .and_is(latex_environment_end_row_parser().not())
                .repeated()
                .collect::<String>(),
        )
        .then(latex_environment_end_row_parser())
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(|(((begin_row, content), end_row), blank_lines), e| {
            let mut children = vec![];
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
            e.state().latex_env_name = String::new(); // reset state
            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::LatexEnvironment.into(),
                children,
            ))
        })
}
