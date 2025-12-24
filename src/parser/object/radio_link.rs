//! radio link parser
use crate::parser::syntax::{OrgLanguage, OrgSyntaxKind};
use crate::parser::{ParserState, RADIO_TARGETS};
use chumsky::input::InputRef;
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::SyntaxNode;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;

fn try_match_string<'a, C: 'a>(
    stream: &mut InputRef<
        'a,
        '_,
        &'a str,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
    >,
    s: &str,
) -> bool {
    let start = stream.save();
    let mut success = true;

    for expected_char in s.chars() {
        let cc = stream.next();
        match cc {
            Some(actual_char) if actual_char == expected_char => {
                continue;
            }
            _ => {
                success = false;
                break;
            }
        }
    }

    stream.rewind(start);
    success
}

fn radio_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>> + Clone
{
    custom::<_, &str, _, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>>(
        move |stream| {
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
                // println!("radio_parser: {matched_string:?}");
                Ok(matched_string)
            } else {
                Err(Rich::custom(
                    SimpleSpan::from(Range {
                        start: *stream.cursor().inner(),
                        end: (stream.cursor().inner() + 0),
                    }),
                    format!("No radio target matched"),
                ))
            }
        },
    )
}

/// radio link parser: PRE RADIO POST
pub(crate) fn radio_link_parser<'a, C: 'a>(
    object_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
    > + Clone
    + 'a,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
    let minimal_objects_parser = object_parser
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<NodeOrToken<GreenNode, GreenToken>>>();
    let radio = minimal_objects_parser.nested_in(radio_parser().to_slice());
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
        .then(radio)
        .then_ignore(post.rewind())
        .map_with(|(_s, radio), e| {
            // fixme: faster to get radio last char?
            let root = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Root.into(),
                radio.clone(),
            ));
            let syntax_tree: SyntaxNode<OrgLanguage> =
                SyntaxNode::new_root(root.into_node().expect("xx"));
            let last_char = syntax_tree
                .last_token()
                .map_or(None, |x| x.text().chars().last());
            (e.state() as &mut RollbackState<ParserState>).prev_char = last_char;

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::RadioLink.into(),
                radio,
            ))
        })
        .boxed()
}
