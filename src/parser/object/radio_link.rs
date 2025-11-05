//! radio link parser
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::{OrgLanguage, OrgSyntaxKind};

use chumsky::input::InputRef;
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::SyntaxNode;
use rowan::{GreenNode, GreenToken, NodeOrToken};

use chumsky::prelude::*;
use std::ops::Range;
use std::sync::Arc;

#[derive(Clone)]
pub struct DynamicStringParser {
    strings: Arc<Vec<String>>,
}

fn try_match_string<'a>(
    stream: &mut InputRef<
        'a,
        '_,
        &'a str,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
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

fn radio_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    custom::<_, &str, _, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>(
        move |stream| {
            let mut longest_match: Option<(String, usize)> = None;

            let state = stream.state().clone();
            let radio_targets = &state.radio_targets;
            for candidate in radio_targets.iter() {
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
pub(crate) fn radio_link_parser<'a>(
    object_parser: impl Parser<
        'a,
        &'a str,
        S2,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let minimal_objects_parser = object_parser
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<S2>>();

    let radio = minimal_objects_parser.nested_in(radio_parser().to_slice());
    let post = any()
        .filter(|c: &char| !c.is_alphanumeric())
        .or(end().to('x'));

    let backup_prev_char = any::<_, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>()
        .map_with(|s, e| {
            e.state().prev_char_backup = e.state().prev_char;
            s
        })
        .rewind();

    backup_prev_char
        .then(radio)
        .map_with(|s, e| {
            // println!("radio_link_parser: s={s:?}");
            e.state().prev_char = e.state().prev_char_backup; // resume prev_char
            s
        })
        .then_ignore(post.rewind())
        .try_map_with(|(_, radio), e| {
            let pre_valid = e.state().prev_char.map_or(true, |c| !c.is_alphanumeric());

            match pre_valid {
                true => {
                    let mut children = vec![];
                    for node in radio {
                        match node {
                            S2::Single(e) => {
                                children.push(e);
                            }
                            S2::Double(e1, e2) => {
                                children.push(e1);
                                children.push(e2);
                            }
                            _ => {}
                        }
                    }

                    let root = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                        OrgSyntaxKind::Root.into(),
                        children.clone(),
                    ));
                    let syntax_tree: SyntaxNode<OrgLanguage> =
                        SyntaxNode::new_root(root.into_node().expect("xx"));
                    let last_char = syntax_tree
                        .last_token()
                        .map_or(None, |x| x.text().chars().last());
                    e.state().prev_char = last_char;

                    Ok(S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(
                        GreenNode::new(OrgSyntaxKind::RadioLink.into(), children),
                    )))
                }
                false => Err(Rich::custom(
                    e.span(),
                    format!(
                        "radio_link_parser: pre_valid={pre_valid}, PRE={:?} not valid",
                        e.state().prev_char
                    ),
                )),
            }
        })
}

// show test with OrgParser, since RadioTargets should be collected firstly.
