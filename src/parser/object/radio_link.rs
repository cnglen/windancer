//! radio link parser
use crate::parser::syntax::OrgLanguage;
use crate::parser::{MyExtra, NT, OSK};
use crate::parser::{ParserState, RADIO_TARGETS};
use chumsky::container::Seq;
use chumsky::input::InputRef;
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::SyntaxNode;

fn try_match_string<'a, C: 'a>(
    stream: &mut InputRef<'a, '_, &'a str, MyExtra<'a, C>>,
    s: &str,
) -> bool {
    let mut remaining = stream.slice_from(&stream.cursor()..).seq_iter();
    let mut success = true;
    for expected_char in s.chars() {
        match remaining.next() {
            Some(c) if c == expected_char => {
                continue;
            }
            _ => {
                success = false;
                break;
            }
        }
    }
    success
}

fn radio_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, String, MyExtra<'a, C>> + Clone {
    custom(move |stream| {
        let before = stream.cursor();

        let mut longest_match: Option<(String, usize)> = None;
        if let Some(radio_targets) = RADIO_TARGETS.get() {
            for candidate in radio_targets {
                if try_match_string(stream, candidate) {
                    let match_len = candidate.len();
                    if longest_match
                        .as_ref()
                        .map_or(true, |(_, len)| match_len > *len)
                    {
                        longest_match = Some((candidate.clone(), match_len));
                    }
                }
            }
        }

        if let Some((matched_string, len)) = longest_match {
            for _ in 0..len {
                stream.next();
            }
            Ok(matched_string)
        } else {
            Err(Rich::custom(
                stream.span_since(&before),
                "No radio target matched",
            ))
        }
    })
}

pub(crate) fn radio_link_parser_inner<'a, C: 'a, E>(
    radio_parser_slice_or_object: E,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone
where
    E: Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
{
    let post = any()
        .filter(|c: &char| !c.is_alphanumeric())
        .or(end().to('x'));

    any()
        .try_map_with(|_s, e| {
            // check with PRE
            let pre_valid = (e.state() as &mut RollbackState<ParserState>)
                .prev_char
                .map_or(true, |c| !c.is_alphanumeric());

            match pre_valid {
                true => Ok(()),
                false => Err(Rich::<char>::custom(
                    e.span(),
                    format!(
                        "radio_link_parser: pre_valid={pre_valid}, PRE={:?} not valid",
                        (e.state() as &mut RollbackState<ParserState>).prev_char
                    ),
                )),
            }
        })
        .rewind()
        .then(radio_parser_slice_or_object)
        .then_ignore(post.rewind())
        .map_with(|(_s, radio), e| {
            // fixme: faster to get radio last char?
            let root = crate::node!(OSK::Root, radio.clone());
            let syntax_tree: SyntaxNode<OrgLanguage> =
                SyntaxNode::new_root(root.into_node().expect("xx"));
            let last_char = syntax_tree
                .last_token()
                .map_or(None, |x| x.text().chars().last());
            (e.state() as &mut RollbackState<ParserState>).prev_char = last_char;

            crate::node!(OSK::RadioLink, radio)
        })
        .boxed()
}

/// radio link parser: PRE RADIO POST
pub(crate) fn radio_link_parser<'a, C: 'a>(
    object_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let minimal_objects_parser = object_parser
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<NT>>();
    let radio_parser_object = minimal_objects_parser.nested_in(radio_parser().to_slice());

    radio_link_parser_inner(radio_parser_object)
}

pub(crate) fn simple_radio_link_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let radio_parser_slice = radio_parser()
        .to_slice()
        .map(|s: &str| vec![crate::token!(OSK::Text, s)]);
    radio_link_parser_inner(radio_parser_slice)
}
