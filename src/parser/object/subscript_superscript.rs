//! Subscript and Superscript

use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::input::InputRef;
use chumsky::input::MapExtra;
use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;

pub(crate) fn chars_final_parser_v2a<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    any()
        .filter(|c: &char| c.is_alphanumeric() || matches!(c, '\\' | '.' | ','))
        .repeated()
        .at_least(1)
        .to_slice()
        .validate(|s: &str, e, emit| {
            if !s.chars().last().expect("at_least(1)").is_alphanumeric() {
                emit.emit(Rich::custom(
                    e.span(),
                    format!(
                        "the `char final` '{}' must end in an alphanumeric character",
                        s
                    ),
                ));
            }

            s.to_string()
        })
}

pub(crate) fn chars_final_parser_v2b<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    any()
        .filter(|c: &char| c.is_alphanumeric() || matches!(c, '\\' | '.' | ','))
        .repeated()
        .at_least(1)
        .to_slice()
        .try_map_with(|s: &str, e| {
            if !s.chars().last().expect("at_least(1)").is_alphanumeric() {
                Err(Rich::custom(
                    e.span(),
                    format!(
                        "the `char final` '{}' must end in an alphanumber character",
                        s
                    ),
                ))
            } else {
                Ok(s.to_string())
            }
        })
}

/// Superscript Parser
pub(crate) fn chars_final_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    custom(
        |inp: &mut InputRef<
            'a,
            '_,
            &'a str,
            extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
        >| {
            let remaining = inp.slice_from(std::ops::RangeFrom {
                start: &inp.cursor(),
            });

            let mut content = String::new();
            for c in remaining.chars() {
                if c.is_alphanumeric() || c == ',' || c == '\\' || c == '.' {
                    content.push(c);
                } else {
                    break;
                }
            }

            // find last alphanumeric in content
            let maybe_final = content
                .char_indices()
                .rev()
                .find(|(_, c)| c.is_alphanumeric());

            match maybe_final {
                None => {
                    let error = Rich::custom::<&str>(
                        SimpleSpan::from(Range {
                            start: *inp.cursor().inner(),
                            end: (inp.cursor().inner() + content.chars().count()),
                        }),
                        "必须包含至少一个alpha_numeric",
                    );
                    return Err(error);
                }

                Some((idx, _)) => {
                    content = content.chars().take(idx + 1).collect::<String>();
                    for _ in 0..idx + 1 {
                        inp.next();
                    }

                    Ok(content)
                }
            }
        },
    )
}

pub(crate) fn superscript_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    let t1 = just("^").then(just("*")).map_with(
        |(sup, aes),
         e: &mut MapExtra<
            '_,
            '_,
            &str,
            extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>,
        >| {
            e.state().prev_char = Some('*');

            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Caret.into(),
                sup,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                aes,
            )));

            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Superscript.into(),
                children,
            )))
        },
    );

    // - standard objects not supported
    // - nested brackets not supported
    let simplified_expressiona = any().and_is(just("}").not()).repeated().collect::<String>();
    let t2a = just("^{")
        .then(simplified_expressiona)
        .then(just("}"))
        .try_map_with(
            |((sup_lb, expression), rb),
             e: &mut MapExtra<
                '_,
                '_,
                &str,
                extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>,
            >| match e.state().prev_char {
                None => {
                    let error = Rich::custom::<&str>(
                        SimpleSpan::from(Range {
                            start: e.span().start(),
                            end: e.span().end(),
                        }),
                        &format!("CHAR is empty"),
                    );
                    Err(error)
                }
                Some(c) if c == ' ' || c == '\t' => {
                    let error = Rich::custom::<&str>(
                        SimpleSpan::from(Range {
                            start: e.span().start(),
                            end: e.span().end(),
                        }),
                        &format!("CHAR is whitesace"),
                    );
                    Err(error)
                }
                _ => {
                    e.state().prev_char = rb.chars().last();

                    let mut children = vec![];

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Caret.into(),
                        &sup_lb.chars().nth(0).unwrap().to_string(),
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::LeftCurlyBracket.into(),
                        &sup_lb.chars().last().unwrap().to_string(),
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &expression,
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::RightCurlyBracket.into(),
                        &sup_lb.chars().last().unwrap().to_string(),
                    )));

                    Ok(S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(
                        GreenNode::new(OrgSyntaxKind::Superscript.into(), children),
                    )))
                }
            },
        );

    let simplified_expressionb = any().and_is(just(")").not()).repeated().collect::<String>();
    let t2b = just("^(")
        .then(simplified_expressionb)
        .then(just(")"))
        .try_map_with(
            |((sup_lb, expression), rb),
             e: &mut MapExtra<
                '_,
                '_,
                &str,
                extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>,
            >| match e.state().prev_char {
                None => {
                    let error = Rich::custom::<&str>(
                        SimpleSpan::from(Range {
                            start: e.span().start(),
                            end: e.span().end(),
                        }),
                        &format!("CHAR is empty"),
                    );
                    Err(error)
                }
                Some(c) if c == ' ' || c == '\t' => {
                    let error = Rich::custom::<&str>(
                        SimpleSpan::from(Range {
                            start: e.span().start(),
                            end: e.span().end(),
                        }),
                        &format!("CHAR is whitesace"),
                    );
                    Err(error)
                }
                _ => {
                    e.state().prev_char = rb.chars().last();

                    let mut children = vec![];

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Caret.into(),
                        &sup_lb.chars().nth(0).unwrap().to_string(),
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::LeftRoundBracket.into(),
                        &sup_lb.chars().last().unwrap().to_string(),
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &expression,
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::RightRoundBracket.into(),
                        &sup_lb.chars().last().unwrap().to_string(),
                    )));

                    Ok(S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(
                        GreenNode::new(OrgSyntaxKind::Superscript.into(), children),
                    )))
                }
            },
        );

    // ^ SIGN CHARS FINAL
    // chars final实现注意:
    // - 如果chars.then(final), chars包含final, 因为chumsky贪心实现，永远不会匹配
    // - chars.then_ignore(final_.rewind()): 会提前终止，也错误
    let sign = one_of("+-").or_not();
    // let chars = any()
    //     .filter(|c: &char| c.is_alphanumeric() || matches!(c, '\\' | '.' | ','))
    //     .repeated()
    //     .collect::<String>()
    //     ;
    // let final_ = any().filter(|c: &char| c.is_alphanumeric());

    // let chars_final = any()
    //     .filter(|c: &char| c.is_alphanumeric() || matches!(c, '\\' | '.' | ','))
    //     .repeated()
    //     .at_least(1)
    //     .collect::<String>()
    //     .filter(|s| s.chars().last().unwrap().is_alphanumeric());

    // 123,,,,,,4021\s
    //            ^
    // and_is
    //

    // III:
    let t3 = just("^")
        .then(sign)
        .then(chars_final_parser())
        .try_map_with(|((sup, sign), content), e| match e.state().prev_char {
            None => {
                let error = Rich::custom::<&str>(
                    SimpleSpan::from(Range {
                        start: e.span().start(),
                        end: e.span().end(),
                    }),
                    &format!("CHAR is empty"),
                );
                Err(error)
            }
            Some(c) if c == ' ' || c == '\t' => {
                let error = Rich::custom::<&str>(
                    SimpleSpan::from(Range {
                        start: e.span().start(),
                        end: e.span().end(),
                    }),
                    &format!("CHAR is whitesace"),
                );
                Err(error)
            }
            _ => {
                e.state().prev_char = content.chars().last();

                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Caret.into(),
                    sup,
                )));

                let text = match sign {
                    Some(s) => format!("{s}{content}"),
                    None => format!("{content}"),
                };

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &text,
                )));

                Ok(S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(
                    GreenNode::new(OrgSyntaxKind::Superscript.into(), children),
                )))
            }
        });

    // // II:
    // let t3 = just("^")
    //     .then(sign)
    //     .then(chars_final)
    //     .map(|s| {
    //         println!("s={s:?}");
    //         s
    //     })
    //     .try_map_with(|s, e| {
    //         let mut children = vec![];
    //         Ok(S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(
    //             GreenNode::new(OrgSyntaxKind::Superscript.into(), children),
    //         )))
    //     });

    // // I:  如果chars.then(final_), chars包含final_, 因为chumsky贪心实现，永远不会匹配
    // let t3 = just("^")
    //     .then(sign)
    //     .then(chars)
    //     .then(final_)
    //     .map(|s|{println!("s={s:?}"); s})
    //     .try_map_with(
    //         |s, e| {
    //             let mut children = vec![];
    //             Ok(S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(
    //                 GreenNode::new(OrgSyntaxKind::Superscript.into(), children),
    //             )))
    //         });

    // let chars_final = any()
    //     .filter(|c: &char| c.is_alphanumeric() || matches!(c, '\\' | '.' | ','))
    //     .then_ignore(
    //         final_.rewind(),
    //         // chars.then(final_).rewind()
    //     )
    //     .repeated()
    //     .at_least(1)
    //     .collect::<String>();

    // let t3 = just("^").then(sign).then(chars_final)
    //     .map(|s|{println!("s={s:?}"); s})
    //     .try_map_with(
    //     |((sup, sign), chars),

    //      e: &mut MapExtra<
    //         '_,
    //         '_,
    //         &str,
    //         extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>,
    //     >| {
    //         let last_char = chars.chars().last().expect("todo");
    //         if !last_char.is_alphanumeric() {
    //             let error = Rich::custom::<&str>(
    //                 SimpleSpan::from(Range {
    //                     start: e.span().start(),
    //                     end: e.span().end(),
    //                 }),
    //                 &format!("FINAL is \\ not matched: '{last_char}' NOT alphanumeric"),
    //             );
    //             Err(error)
    //         } else {
    //             e.state().prev_char = chars.chars().last();
    //             let mut children = vec![];
    //             children.push(NodeOrToken::Token(GreenToken::new(
    //                 OrgSyntaxKind::Caret.into(),
    //                 sup,
    //             )));

    //             let text = match sign {
    //                 Some(s) => format!("{s}{chars}"),
    //                 None => format!("{chars}"),
    //             };

    //             children.push(NodeOrToken::Token(GreenToken::new(
    //                 OrgSyntaxKind::Text.into(),
    //                 &text,
    //             )));
    //             Ok(S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(
    //                 GreenNode::new(OrgSyntaxKind::Superscript.into(), children),
    //             )))
    //         }
    //     },
    // );

    t1.or(t3).or(t2a)
}

// /// Subscript Parser
// pub(crate) fn subscript_parser<'a>()
//                                    -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
//     S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(
//         GreenNode::new(OrgSyntaxKind::Superscript.into(), vec![]),
//     ))
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::syntax::OrgLanguage;
    use rowan::SyntaxNode;

    // #[test]
    // fn test_name() {
    //     let s = "a^-12,889,78.3\\a";

    //     superscript_parser().parse(s).unwrap()

    // }
}
