use chumsky::prelude::*;

use crate::compiler::parser::{MyExtra, NT, OSK};

/// Macro parser
pub(crate) fn macro_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
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
                .then(any().and_is(just(")}}}").not()).repeated().to_slice())
                .then(just(")"))
                .or_not(),
        )
        .then(just("}}}"))
        .map(
            |(((left_3curly, name), maybe_leftround_args_rightround), right_3curly): (
                ((&_, &_), Option<((&_, &str), &_)>),
                &_,
            )| {
                let mut children = Vec::with_capacity(6);
                children.push(crate::token!(OSK::LeftCurlyBracket3, left_3curly));
                children.push(crate::token!(OSK::MacroName, name));
                if let Some(((left_round, args), right_round)) = maybe_leftround_args_rightround {
                    children.push(crate::token!(OSK::LeftRoundBracket, left_round));
                    if !args.is_empty() {
                        children.push(crate::token!(OSK::MacroArgs, args));
                    }
                    children.push(crate::token!(OSK::RightRoundBracket, right_round));
                }
                children.push(crate::token!(OSK::RightCurlyBracket3, right_3curly));

                crate::node!(OSK::Macro, children)
            },
        )
        .boxed()
}
