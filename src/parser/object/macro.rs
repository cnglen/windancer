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
    // {{{NAME(ARGUMENTS)}}}
    just(r"{{{")
        .then(name)
        .then(
            just("(")
                .then(
                    any()
                        .and_is(just(")}}}").ignored().not())
                        .repeated()
                        .to_slice(),
                )
                .then(just(")"))
                .or_not(),
        )
        .then(just("}}}"))
        .map_with(
            |(((left_3curly, name), maybe_leftround_args_rightround), right_3curly): (
                ((&_, &_), Option<((&_, &str), &_)>),
                &_,
            ),
             e| {
                let state: &mut RollbackState<ParserState> = e.state();
                state.prev_char = Some('}');

                let mut children = Vec::with_capacity(6);
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftCurlyBracket3.into(),
                    left_3curly,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::MacroName.into(),
                    name,
                )));

                if let Some(((left_round, args), right_round)) = maybe_leftround_args_rightround {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::LeftRoundBracket.into(),
                        left_round,
                    )));

                    if !args.is_empty() {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::MacroArgs.into(),
                            args,
                        )));
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::RightRoundBracket.into(),
                        right_round,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightCurlyBracket3.into(),
                    right_3curly,
                )));

                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Macro.into(), children))
            },
        )
        .boxed()
}
