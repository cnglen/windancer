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
pub(crate) fn latex_fragment_parser<'a, C: 'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone
+ 'a {
    // t1 <- \(CONTENTS\)
    // CONTENTS <- !("\(")
    let t1 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>(r"\")
        .then(just(r"("))
        .then(
            // any().and_is(just(r"\)").not()).repeated().to_slice() // slow version
            none_of('\\')
                .to_slice()
                .or(just('\\').then(none_of(')')).to_slice())
                .repeated()
                .to_slice(),
        )
        .then(just(r"\"))
        .then(just(r")"))
        .map_with(|((((dd1, lb), content), dd2), rb), e| {
            e.state().prev_char = rb.chars().last();

            NT::Node(GreenNode::new(
                OSK::LatexFragment.into(),
                vec![
                    NT::Token(GreenToken::new(OSK::BackSlash.into(), dd1)),
                    NT::Token(GreenToken::new(OSK::LeftRoundBracket.into(), lb)),
                    NT::Token(GreenToken::new(OSK::Text.into(), content)),
                    NT::Token(GreenToken::new(OSK::BackSlash.into(), dd2)),
                    NT::Token(GreenToken::new(OSK::RightRoundBracket.into(), rb)),
                ],
            ))
        });

    // \[CONTENTS\]
    let t2 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>(r##"\"##)
        .then(just("["))
        .then(
            // any().and_is(just(r##"\]"##).not()).repeated().to_slice()
            none_of('\\')
                .to_slice()
                .or(just('\\').then(none_of(']')).to_slice())
                .repeated()
                .to_slice(),
        )
        .then(just(r##"\"##))
        .then(just("]"))
        .map_with(|((((dd1, lb), content), dd2), rb), e| {
            e.state().prev_char = dd2.chars().last();

            NT::Node(GreenNode::new(
                OSK::LatexFragment.into(),
                vec![
                    NT::Token(GreenToken::new(OSK::BackSlash.into(), dd1)),
                    NT::Token(GreenToken::new(OSK::LeftSquareBracket.into(), lb)),
                    NT::Token(GreenToken::new(OSK::Text.into(), content)),
                    NT::Token(GreenToken::new(OSK::BackSlash.into(), dd2)),
                    NT::Token(GreenToken::new(OSK::RightSquareBracket.into(), rb)),
                ],
            ))
        });

    // $$CONTENTS$$
    let t3 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>("$$")
        .then(
            // any().and_is(just("$$").not()).repeated().to_slice()
            none_of('$')
                .to_slice()
                .or(just('$').then(none_of('$')).to_slice())
                .repeated()
                .to_slice(),
        )
        .then(just("$$"))
        .map_with(|((dd_pre, content), dd_post), e| {
            e.state().prev_char = dd_post.chars().last();

            NT::Node(GreenNode::new(
                OSK::LatexFragment.into(),
                vec![
                    NT::Token(GreenToken::new(OSK::Dollar2.into(), dd_pre)),
                    NT::Token(GreenToken::new(OSK::Text.into(), content)),
                    NT::Token(GreenToken::new(OSK::Dollar2.into(), dd_post)),
                ],
            ))
        });

    // v2: use prev_char state
    let post = any()
        .filter(|c: &char| c.is_ascii_punctuation() || matches!(c, ' ' | '\t' | '\r' | '\n'))
        .or(end().to('x'));
    let t4 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>("$")
        .then(none_of(".,?;\" \t"))
        .then(just("$"))
        .then_ignore(post.rewind())
        .try_map_with(|((d_pre, c), d_post), e| match e.state().prev_char {
            Some(c) if c == '$' => Err(Rich::custom::<&str>(e.span(), &format!("prev_char is $"))),

            _ => {
                e.state().prev_char = d_post.chars().last();
                Ok(NT::Node(GreenNode::new(
                    OSK::LatexFragment.into(),
                    vec![
                        NT::Token(GreenToken::new(OSK::Dollar.into(), d_pre)),
                        NT::Token(GreenToken::new(OSK::Text.into(), &format!("{}", c))),
                        NT::Token(GreenToken::new(OSK::Dollar.into(), d_post)),
                    ],
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
                .to_slice(),
        )
        .then(border2)
        .then(just("$"))
        .then_ignore(post.rewind()) // todo
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
    let t01 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>(r##"\"##)
        .then(name)
        .then(just("["))
        .then(none_of("{}[]\r\n").repeated().to_slice())
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
        .then(none_of("{}\r\n").repeated().to_slice())
        .then(just("}"))
        .map_with(|((((bs, name), lb), content), rb), e| {
            e.state().prev_char = rb.chars().last();

            let mut children = vec![];
            let _content = format!("{bs}{name}{lb}{content}{rb}");
            children.push(NT::Token(GreenToken::new(OSK::Text.into(), &_content)));

            NT::Node(GreenNode::new(OSK::LatexFragment.into(), children))
        });

    Parser::boxed(choice((t1, t2, t3, t4, t5, t01, t02)))
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate test;
    use crate::parser::common::get_parser_output;
    use pretty_assertions::assert_eq;
    use test::Bencher;

    #[test]
    fn test_latex_fragment_01() {
        assert_eq!(
            get_parser_output(latex_fragment_parser::<()>(), r"\(\alpha\)"),
            r###"LatexFragment@0..10
  BackSlash@0..1 "\\"
  LeftRoundBracket@1..2 "("
  Text@2..8 "\\alpha"
  BackSlash@8..9 "\\"
  RightRoundBracket@9..10 ")"
"###
        );
    }

    #[bench]
    fn test_latex_fragment_01_bench(b: &mut Bencher) {
        let parser = latex_fragment_parser::<()>();
        b.iter(|| {
            assert!(!parser.parse(r"\(\alpha\)").has_errors());
        })
    }
}
