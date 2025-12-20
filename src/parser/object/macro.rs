use crate::parser::ParserState;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// Macro parser
pub(crate) fn macro_parser<'a, C: 'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
    let name = any()
        .filter(|c: &char| c.is_alphabetic())
        .then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '-'))
                .repeated(),
        )
        .to_slice();

    // {{{NAME}}}
    let t1 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>("{{{")
        .then(name)
        .then(just("}}}"))
        .map_with(|((left_3curly, name), right_3curly), e| {
            e.state().prev_char = right_3curly.chars().last();

            let mut children = Vec::with_capacity(3);
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftCurlyBracket3.into(),
                left_3curly,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::MacroName.into(),
                name,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightCurlyBracket3.into(),
                right_3curly,
            )));

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Macro.into(), children))
        });

    // {{{NAME(ARGUMENTS)}}}
    let t2 = just(r"{{{")
        .then(name)
        .then(just("("))
        .then(
            any()
                .and_is(just(")}}}").ignored().not())
                .repeated()
                .collect::<String>(),
        )
        .then(just(")"))
        .then(just("}}}"))
        .map_with(
            |(((((left_3curly, name), left_round), args), right_round), right_3curly), e| {
                e.state().prev_char = right_3curly.chars().last();

                let mut children = Vec::with_capacity(6);
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftCurlyBracket3.into(),
                    left_3curly,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::MacroName.into(),
                    name,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftRoundBracket.into(),
                    &left_round.to_string(),
                )));

                if args.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::MacroArgs.into(),
                        &args,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightRoundBracket.into(),
                    &right_round,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightCurlyBracket3.into(),
                    right_3curly,
                )));

                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Macro.into(), children))
            },
        );

    Parser::boxed(choice((t1, t2)))
}
