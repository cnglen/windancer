//! angle link parser
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

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

// /// plain link parser
// pub(crate) fn plain_link_parser<'a>()
// -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone {
//     let protocol = protocol();
//     let path_plain = none_of(" \t()[]<>,") // . is permitted: orgmode.org, xx@xx.com
//         .repeated()
//         .at_least(1)
//         .collect::<String>();

//     just("<")
//         .then(protocol)
//         .then(just(":"))
//         .then(path_angle)
//         .then(just(">"))
//         .map(
//             |((((left_angle, protocol), colon), path_angle), right_angle)| {
//                 let mut children = vec![];
//                 children.push(NodeOrToken::Token(GreenToken::new(
//                     OrgSyntaxKind::LeftAngleBracket.into(),
//                     left_angle,
//                 )));

//                 children.push(NodeOrToken::Token(GreenToken::new(
//                     OrgSyntaxKind::Text.into(),
//                     &protocol,
//                 )));

//                 children.push(NodeOrToken::Token(GreenToken::new(
//                     OrgSyntaxKind::Colon.into(),
//                     colon,
//                 )));

//                 children.push(NodeOrToken::Token(GreenToken::new(
//                     OrgSyntaxKind::Text.into(),
//                     &path_angle,
//                 )));

//                 children.push(NodeOrToken::Token(GreenToken::new(
//                     OrgSyntaxKind::RightAngleBracket.into(),
//                     right_angle,
//                 )));

//                 S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
//                     OrgSyntaxKind::AngleBracketLink.into(),
//                     children,
//                 )))
//             },
//         )
// }

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
