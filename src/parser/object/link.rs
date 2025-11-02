//! link parser, including angle/plain/regular link
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;
use std::ops::Range;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

use phf::phf_set;

pub(crate) static LINK_PROTOCOLS: phf::Set<&'static str> = phf_set! {
    "treemacs", "eww", "rmail", "mhe", "irc", "info", "gnus", "docview", "bibtex", "bbdb", "w3m", "doi", "attachment", "id", "file+sys", "file+emacs", "shell", "news",
    "mailto", "https", "http", "ftp", "help", "file", "elisp"
};

/// PROTOCOL: A string which is one of the link type strings in org-link-parameters
#[allow(unused)]
pub(crate) fn protocol<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    any()
        .filter(|c: &char| matches!(c, 'a'..'z' | '+'))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|e| LINK_PROTOCOLS.contains(e))
}

// pathplain parser
// fixme: parenthesis-wrapped not supported yet
fn path_plain_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    custom::<_, &str, _, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>(|inp| {
        let remaining = inp.slice_from(std::ops::RangeFrom {
            start: &inp.cursor(),
        });

        let content: String = remaining
            .chars()
            .take_while(|c| !matches!(c, ' ' | '\t' | '\n' | '[' | ']' | '<' | '>' | '(' | ')'))
            .collect();

        let maybe_final = content
            .char_indices()
            .rev()
            .find(|(_, c)| !(c.is_ascii_punctuation() || matches!(c, ' ' | '\t' | '\n')));

        let (idx, _) = maybe_final.ok_or_else(|| {
            let n_char = content.chars().count();
            Rich::custom(
                SimpleSpan::from(Range {
                    start: *inp.cursor().inner(),
                    end: (inp.cursor().inner() + n_char),
                }),
                format!(
                    "pathplain must include at least one alphanumeric char: '{}'",
                    content
                ),
            )
        })?;

        let pathplain = content.chars().take(idx + 1).collect::<String>();
        for _ in 0..idx + 1 {
            inp.next();
        }
        Ok(pathplain)
    })
}

/// plain link parser
// todo: parenthesis-wrapped not supported yet
pub(crate) fn plain_link_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let protocol = protocol();
    let post = any()
        .filter(|c: &char| !c.is_alphanumeric())
        .or(end().to('x'));
    protocol
        .then(just(":"))
        .then(path_plain_parser())
        .then_ignore(post.rewind())
        .try_map_with(|((protocol, colon), pathplain), e| {
            let pre_valid = e.state().prev_char.map_or(true, |c| !c.is_alphanumeric());

            match pre_valid {
                true => {
                    e.state().prev_char = pathplain.chars().last();

                    let mut children = vec![];
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &protocol,
                    )));
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Colon.into(),
                        colon,
                    )));
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &pathplain,
                    )));

                    Ok(S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(
                        GreenNode::new(OrgSyntaxKind::PlainLink.into(), children),
                    )))
                }
                false => Err(Rich::custom(
                    e.span(),
                    format!(
                        "plainlink_parser(): pre_valid={pre_valid}, PRE={:?} not valid",
                        e.state().prev_char
                    ),
                )),
            }
        })
}

/// angle link parser
pub(crate) fn angle_link_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let protocol = any()
        .filter(|c: &char| matches!(c, 'a'..'z' | '+'))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|e| LINK_PROTOCOLS.contains(e));
    let path_angle = none_of(">,") // . is permitted: orgmode.org, xx@xx.com
        .repeated()
        .at_least(1)
        .collect::<String>();

    just("<")
        .then(protocol)
        .then(just(":"))
        .then(path_angle)
        .then(just(">"))
        .map(
            |((((left_angle, protocol), colon), path_angle), right_angle)| {
                let mut children = vec![];
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftAngleBracket.into(),
                    left_angle,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &protocol,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    colon,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &path_angle,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightAngleBracket.into(),
                    right_angle,
                )));

                S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::AngleLink.into(),
                    children,
                )))
            },
        )
}

/// regular link parser
pub(crate) fn regular_link_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let pathreg = just("[")
        .then(none_of("]").repeated().collect::<String>())
        .then(just("]"))
        .map(|((lbracket, path), rbracket)| {
            let mut children = vec![];
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                lbracket,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &path,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));
            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::LinkPath.into(),
                children,
            ))
        });

    just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>("[")
        .then(pathreg)
        .then(
            just("[")
                .then(none_of("]").repeated().collect::<String>())
                .then(just("]"))
                .or_not()
                .map(|description| match description {
                    None => None,

                    Some(((lbracket, content), rbracket)) => {
                        let mut children = vec![];
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::LeftSquareBracket.into(),
                            lbracket,
                        )));

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Text.into(),
                            &content,
                        )));

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::RightSquareBracket.into(),
                            rbracket,
                        )));

                        Some(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                            OrgSyntaxKind::LinkDescription.into(),
                            children,
                        )))
                    }
                }),
        )
        .then(just("]"))
        .map_with(|(((lbracket, path), maybe_desc), rbracket), e| {
            // Q: ^ why type annotation needed?  for e:
            e.state().prev_char = rbracket.chars().last();

            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                lbracket,
            )));

            children.push(path);

            match maybe_desc {
                None => {}
                Some(desc) => {
                    children.push(desc);
                }
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            let link = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Link.into(),
                children,
            ));

            S2::Single(link)
        })
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parsers_output;
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_01_plain_link() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), r"https://foo.bar"),
            r###"Root@0..15
  PlainLink@0..15
    Text@0..5 "https"
    Colon@5..6 ":"
    Text@6..15 "//foo.bar"
"###
        );
    }
}
