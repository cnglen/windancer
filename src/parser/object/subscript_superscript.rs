//! Subscript and Superscript

use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::collections::HashMap;
use std::ops::Range;
type NT = NodeOrToken<GreenNode, GreenToken>;
type OSK = OrgSyntaxKind;

/// chars final parser:
/// - find the longest string consisting of <alphanumeric characters, commas, backslashes, and dots>, whose length>=1
/// - find the last alphnumeric character as FINAL
pub(crate) fn chars_final_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    custom::<_, &str, _, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>>(|inp| {
        let remaining = inp.slice_from(std::ops::RangeFrom {
            start: &inp.cursor(),
        });

        let content: String = remaining
            .chars()
            .take_while(|c| c.is_alphanumeric() || matches!(c, ',' | '\\' | '.'))
            .collect();

        // find last alphanumeric in content
        let maybe_final = content
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_alphanumeric());

        let (idx, _) = maybe_final.ok_or_else(|| {
            let n_char = content.chars().count();
            Rich::custom(
                SimpleSpan::from(Range {
                    start: *inp.cursor().inner(),
                    end: (inp.cursor().inner() + n_char),
                }),
                format!(
                    "superscript must include at least one alphanumeric char: '{}'",
                    content
                ),
            )
        })?;

        let chars_final = content.chars().take(idx + 1).collect::<String>();
        for _ in 0..idx + 1 {
            inp.next();
        }
        Ok(chars_final)
    })
}

pub(crate) fn superscript_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    // ^*
    let t1 = just::<_, _, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>("^")
        .then(just("*"))
        .map_with(|(sup, aes), e| {
            e.state().prev_char = Some('*');

            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::Caret.into(), sup)));
            children.push(NT::Token(GreenToken::new(OSK::Text.into(), aes)));

            S2::Single(NT::Node(GreenNode::new(OSK::Superscript.into(), children)))
        });

    // CHAR^{expression} / CHAR^(EXPRESSION)
    // FIXME: standard objects not supported yet: user standar_object.nested(expression)
    let var = none_of::<&str, &str, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>(
        "{}()\r\n",
    )
    .repeated()
    .at_least(1)
    .to_slice();
    let mut single_expression = Recursive::declare(); // foo / (foo) / (((foo)))
    single_expression.define(
        var.or(just("(")
            .then(single_expression.clone().repeated())
            .then(just(")"))
            .to_slice())
            .or(just("{")
                .then(single_expression.clone().repeated())
                .then(just("}"))
                .to_slice()),
    );
    let expression = single_expression.clone().repeated().to_slice(); // foo(bar){(def)ghi}

    let pairs = HashMap::from([('(', ')'), ('{', '}')]);
    let pair_starts = pairs.keys().copied().collect::<Vec<_>>();
    let pair_ends = pairs.values().copied().collect::<Vec<_>>();
    let t2 = just::<_, _, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>("^")
        .then(one_of(pair_starts))
        .then(expression)
        .then(one_of(pair_ends))
        .try_map_with(
            move |(((sup, lb), expression), rb), e| match e.state().prev_char {
                None => {
                    let error = Rich::custom::<&str>(e.span(), &format!("CHAR is empty"));
                    Err(error)
                }
                Some(c) if c == ' ' || c == '\t' => {
                    let error = Rich::custom::<&str>(e.span(), &format!("CHAR is whitesace"));
                    Err(error)
                }

                _ => {
                    let expected_rb = *pairs.get(&lb).unwrap();

                    if rb != expected_rb {
                        Err(Rich::custom::<&str>(
                            e.span(),
                            &format!("bracket not matched: {lb} {rb}"),
                        ))
                    } else {
                        e.state().prev_char = Some(rb);

                        let mut children = vec![];
                        children.push(NT::Token(GreenToken::new(OSK::Caret.into(), sup)));
                        children.push(NT::Token(GreenToken::new(
                            OSK::LeftCurlyBracket.into(),
                            lb.to_string().as_str(),
                        )));
                        children.push(NT::Token(GreenToken::new(OSK::Text.into(), &expression)));
                        children.push(NT::Token(GreenToken::new(
                            OSK::RightCurlyBracket.into(),
                            rb.to_string().as_str(),
                        )));

                        Ok(S2::Single(NT::Node(GreenNode::new(
                            OSK::Superscript.into(),
                            children,
                        ))))
                    }
                }
            },
        );

    // ^ SIGN CHARS FINAL
    let sign = one_of("+-").or_not();
    let t3 = just("^")
        .then(sign)
        .then(chars_final_parser())
        .try_map_with(|((sup, sign), content), e| match e.state().prev_char {
            None => Err(Rich::custom(e.span(), format!("CHAR is empty"))),
            Some(c) if matches!(c, ' ' | '\t') => {
                Err(Rich::custom(e.span(), format!("CHAR is whitesace")))
            }
            _ => {
                e.state().prev_char = content.chars().last();

                let mut children = vec![];
                children.push(NT::Token(GreenToken::new(OSK::Caret.into(), sup)));
                let text = sign.map_or_else(|| content.clone(), |s| format!("{s}{content}"));
                children.push(NT::Token(GreenToken::new(OSK::Text.into(), &text)));

                Ok(S2::Single(NT::Node(GreenNode::new(
                    OSK::Superscript.into(),
                    children,
                ))))
            }
        });

    t1.or(t3).or(t2)
}

// /// Subscript Parser
// pub(crate) fn subscript_parser<'a>()
//                                    -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
//     S2::Single(NT::Node(
//         GreenNode::new(OSK::Superscript.into(), vec![]),
//     ))
// }
