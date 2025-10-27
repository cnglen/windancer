//! Subscript and Superscript

use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::input::InputRef;
use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;
type NT = NodeOrToken<GreenNode, GreenToken>;
type OSK = OrgSyntaxKind;

/// chars final parser
pub(crate) fn chars_final_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone
{
    custom::<_, &str, _, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>>(|inp| {
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
                let error = Rich::custom(
                    SimpleSpan::from(Range {
                        start: *inp.cursor().inner(),
                        end: (inp.cursor().inner() + content.chars().count()),
                    }),
                    "must include at least one alphanumeric char",
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
    })
}

pub(crate) fn superscript_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    let t1 = just::<_, _, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>("^")
        .then(just("*"))
        .map_with(|(sup, aes), e| {
            e.state().prev_char = Some('*');

            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::Caret.into(), sup)));
            children.push(NT::Token(GreenToken::new(OSK::Text.into(), aes)));

            S2::Single(NT::Node(GreenNode::new(OSK::Superscript.into(), children)))
        });

    // - standard objects not supported
    // - nested brackets not supported
    let simplified_expressiona = any().and_is(just("}").not()).repeated().collect::<String>();
    let t2a = just::<_, _, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>("^{")
        .then(simplified_expressiona)
        .then(just("}"))
        .try_map_with(|((sup_lb, expression), rb), e| match e.state().prev_char {
            None => {
                let error = Rich::custom::<&str>(e.span(), &format!("CHAR is empty"));
                Err(error)
            }
            Some(c) if c == ' ' || c == '\t' => {
                let error = Rich::custom::<&str>(e.span(), &format!("CHAR is whitesace"));
                Err(error)
            }
            _ => {
                e.state().prev_char = rb.chars().last();

                let mut children = vec![];
                children.push(NT::Token(GreenToken::new(
                    OSK::Caret.into(),
                    &sup_lb.chars().nth(0).unwrap().to_string(),
                )));
                children.push(NT::Token(GreenToken::new(
                    OSK::LeftCurlyBracket.into(),
                    &sup_lb.chars().last().unwrap().to_string(),
                )));
                children.push(NT::Token(GreenToken::new(OSK::Text.into(), &expression)));
                children.push(NT::Token(GreenToken::new(
                    OSK::RightCurlyBracket.into(),
                    &rb.chars().last().unwrap().to_string(),
                )));

                Ok(S2::Single(NT::Node(GreenNode::new(
                    OSK::Superscript.into(),
                    children,
                ))))
            }
        });

    let simplified_expressionb = any().and_is(just(")").not()).repeated().collect::<String>();
    let t2b = just::<_, _, extra::Full<Rich<'_, char>, SimpleState<ParserState>, ()>>("^(")
        .then(simplified_expressionb)
        .then(just(")"))
        .try_map_with(|((sup_lb, expression), rb), e| match e.state().prev_char {
            None => {
                let error = Rich::custom::<&str>(e.span(), &format!("CHAR is empty"));
                Err(error)
            }
            Some(c) if c == ' ' || c == '\t' => {
                let error = Rich::custom::<&str>(e.span(), &format!("CHAR is whitesace"));
                Err(error)
            }
            _ => {
                e.state().prev_char = rb.chars().last();

                let mut children = vec![];
                children.push(NT::Token(GreenToken::new(
                    OSK::Caret.into(),
                    &sup_lb.chars().nth(0).unwrap().to_string(),
                )));
                children.push(NT::Token(GreenToken::new(
                    OSK::LeftRoundBracket.into(),
                    &sup_lb.chars().last().unwrap().to_string(),
                )));
                children.push(NT::Token(GreenToken::new(OSK::Text.into(), &expression)));
                children.push(NT::Token(GreenToken::new(
                    OSK::RightRoundBracket.into(),
                    &rb.chars().last().unwrap().to_string(),
                )));

                Ok(S2::Single(NT::Node(GreenNode::new(
                    OSK::Superscript.into(),
                    children,
                ))))
            }
        });

    // ^ SIGN CHARS FINAL
    let sign = one_of("+-").or_not();
    let t3 = just("^")
        .then(sign)
        .then(chars_final_parser())
        .try_map_with(|((sup, sign), content), e| match e.state().prev_char {
            None => {
                let error = Rich::custom::<&str>(e.span(), &format!("CHAR is empty"));
                Err(error)
            }
            Some(c) if c == ' ' || c == '\t' => {
                let error = Rich::custom::<&str>(e.span(), &format!("CHAR is whitesace"));
                Err(error)
            }
            _ => {
                e.state().prev_char = content.chars().last();

                let mut children = vec![];
                children.push(NT::Token(GreenToken::new(OSK::Caret.into(), sup)));
                let text = match sign {
                    Some(s) => format!("{s}{content}"),
                    None => format!("{content}"),
                };
                children.push(NT::Token(GreenToken::new(OSK::Text.into(), &text)));

                Ok(S2::Single(NT::Node(GreenNode::new(
                    OSK::Superscript.into(),
                    children,
                ))))
            }
        });

    t1.or(t3).or(t2a).or(t2b)
}

// /// Subscript Parser
// pub(crate) fn subscript_parser<'a>()
//                                    -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
//     S2::Single(NT::Node(
//         GreenNode::new(OSK::Superscript.into(), vec![]),
//     ))
// }
