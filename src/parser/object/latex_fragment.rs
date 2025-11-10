//! latex fragment parser
// todo: PRE update state or parse(pre, latex_framengt)?
use crate::parser::ParserState;
use crate::parser::object::entity::ENTITYNAME_TO_HTML;
use crate::parser::syntax::OrgSyntaxKind;
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

type NT = NodeOrToken<GreenNode, GreenToken>;
type OSK = OrgSyntaxKind;

// Latex Frament parser
pub(crate) fn latex_fragment_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    // \(CONTENTS\)
    let t1 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>(r##"\"##)
        .then(just("("))
        .then(
            any()
                .and_is(just(r##"\)"##).not())
                .repeated()
                .collect::<String>(),
        )
        .then(just(r##"\"##))
        .then(just(")"))
        .map_with(|((((dd1, lb), content), dd2), rb), e| {
            e.state().prev_char = rb.chars().last();

            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::BackSlash.into(), dd1)));
            children.push(NT::Token(GreenToken::new(OSK::LeftRoundBracket.into(), lb)));
            children.push(NT::Token(GreenToken::new(OSK::Text.into(), &content)));
            children.push(NT::Token(GreenToken::new(OSK::BackSlash.into(), dd2)));
            children.push(NT::Token(GreenToken::new(
                OSK::RightRoundBracket.into(),
                rb,
            )));

            NT::Node(GreenNode::new(OSK::LatexFragment.into(), children))
        });

    // \[CONTENTS\]
    let t2 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>(r##"\"##)
        .then(just("["))
        .then(
            any()
                .and_is(just(r##"\]"##).not())
                .repeated()
                .collect::<String>(),
        )
        .then(just(r##"\"##))
        .then(just("]"))
        .map_with(|((((dd1, lb), content), dd2), rb), e| {
            e.state().prev_char = dd2.chars().last();

            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::BackSlash.into(), dd1)));
            children.push(NT::Token(GreenToken::new(
                OSK::LeftSquareBracket.into(),
                lb,
            )));
            children.push(NT::Token(GreenToken::new(OSK::Text.into(), &content)));
            children.push(NT::Token(GreenToken::new(OSK::BackSlash.into(), dd2)));
            children.push(NT::Token(GreenToken::new(
                OSK::RightSquareBracket.into(),
                rb,
            )));

            NT::Node(GreenNode::new(OSK::LatexFragment.into(), children))
        });

    // $$CONTENTS$$
    let t3 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>("$$")
        .then(
            any()
                .and_is(just("$$").not())
                .repeated()
                .collect::<String>(),
        )
        .then(just("$$"))
        .map_with(|((dd_pre, content), dd_post), e| {
            e.state().prev_char = dd_post.chars().last();

            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::Dollar2.into(), dd_pre)));
            children.push(NT::Token(GreenToken::new(OSK::Text.into(), &content)));
            children.push(NT::Token(GreenToken::new(OSK::Dollar2.into(), dd_post)));

            NT::Node(GreenNode::new(OSK::LatexFragment.into(), children))
        });

    // v2: use prev_char state
    let post = any()
        .filter(|c: &char| c.is_ascii_punctuation() || matches!(c, ' ' | '\t' | '\r' | '\n'))
        .or(end().to('x'));
    let t4 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>("$")
        .then(none_of(".,?;\" \t"))
        .then(just("$"))
        .then_ignore(post.rewind())
        .try_map_with(|((d_pre, c), d_post), e| match e.state().prev_char {
            Some(c) if c == '$' => Err(Rich::custom::<&str>(e.span(), &format!("prev_char is $"))),

            _ => {
                e.state().prev_char = d_post.chars().last();

                let mut children = vec![];
                children.push(NT::Token(GreenToken::new(OSK::Dollar.into(), d_pre)));
                children.push(NT::Token(GreenToken::new(
                    OSK::Text.into(),
                    &format!("{}", c),
                )));
                children.push(NT::Token(GreenToken::new(OSK::Dollar.into(), d_post)));

                Ok(NT::Node(GreenNode::new(
                    OSK::LatexFragment.into(),
                    children,
                )))
            }
        });

    // PRE$BORDER1 BODY BORDER2$POST
    let border1 = none_of("\r\n \t.,;$");
    let border2 = none_of("\r\n \t.,$");
    let t5 = just("$")
        .then(border1)
        .then(
            any()
                .and_is(border2.then(just("$")).not())
                .repeated()
                .collect::<String>(),
        )
        .then(border2)
        .then(just("$"))
        .then_ignore(post.rewind())
        .try_map_with(|((((d_pre, border1), body), border2), d_post), e| {
            match e.state().prev_char {
                Some(c) if c == '$' => {
                    Err(Rich::custom::<&str>(e.span(), &format!("prev_char is $")))
                }
                _ => {
                    e.state().prev_char = d_post.chars().last();

                    let mut children = vec![];
                    children.push(NT::Token(GreenToken::new(OSK::Dollar.into(), d_pre)));
                    let content = format!("{border1}{body}{border2}");
                    children.push(NT::Token(GreenToken::new(OSK::Text.into(), &content)));
                    children.push(NT::Token(GreenToken::new(OSK::Dollar.into(), d_post)));

                    Ok(NT::Node(GreenNode::new(
                        OSK::LatexFragment.into(),
                        children,
                    )))
                }
            }
        });

    // \NAME [CONTENTS1]
    let name = any()
        .filter(|c: &char| c.is_alphabetic())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|name| !ENTITYNAME_TO_HTML.contains_key(name));
    let t01 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>(r##"\"##)
        .then(name)
        .then(just("["))
        .then(
            none_of("{}[]\r\n")
                .and_is(just("]").not())
                .repeated()
                .collect::<String>(),
        )
        .then(just("]"))
        .map_with(|((((bs, name), lb), content), rb), e| {
            e.state().prev_char = rb.chars().last();

            let mut children = vec![];
            let _content = format!("{bs}{name}{lb}{content}{rb}");
            children.push(NT::Token(GreenToken::new(OSK::Text.into(), &_content)));

            NT::Node(GreenNode::new(OSK::LatexFragment.into(), children))
        });

    // \NAME {CONTENTS2}
    let t02 = just(r##"\"##)
        .then(name)
        .then(just("{"))
        .then(
            none_of("{}\r\n")
                .and_is(just("}").not())
                .repeated()
                .collect::<String>(),
        )
        .then(just("}"))
        .map_with(|((((bs, name), lb), content), rb), e| {
            e.state().prev_char = rb.chars().last();

            let mut children = vec![];
            let _content = format!("{bs}{name}{lb}{content}{rb}");
            children.push(NT::Token(GreenToken::new(OSK::Text.into(), &_content)));

            NT::Node(GreenNode::new(OSK::LatexFragment.into(), children))
        });

    choice((t1, t2, t3, t4, t5, t01, t02))
}
