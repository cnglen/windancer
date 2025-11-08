//! Keyword parser
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use phf::phf_set;
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) static ORG_ELEMENT_AFFILIATED_KEYWORDS: phf::Set<&'static str> = phf_set! {
    "CAPTION",
    "DATA",
    "HEADER",
    "HEADERS",
    "LABEL",
    "NAME",
    "PLOT",
    "RESNAME",
    "RESULT",
    "RESULTS",
    "SOURCE",
    "SRCNAME",
    "TBLNAME"
};

pub(crate) static ORG_ELEMENT_DUAL_KEYWORDS: phf::Set<&'static str> = phf_set! {
    "CAPTION", "RESULTS"
};

pub(crate) static ORG_ELEMENT_PARSED_KEYWORDS: phf::Set<&'static str> = phf_set! {
    "CAPTION"
};

pub(crate) fn affiliated_keyword_parser<'a>(
    object_parser: impl Parser<
        'a,
        &'a str,
        S2,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let key = any()
        .filter(|c: &char| matches!(c, 'a'..'z' | 'A'..'Z'| '0'..'9'))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|name| ORG_ELEMENT_AFFILIATED_KEYWORDS.contains(&name.to_uppercase()));

    let key_with_optvalue = any()
        .filter(|c: &char| matches!(c, 'a'..'z' | 'A'..'Z'| '0'..'9'))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|name| ORG_ELEMENT_DUAL_KEYWORDS.contains(&name.to_uppercase()));

    let key_with_objects = any()
        .filter(|c: &char| matches!(c, 'a'..'z' | 'A'..'Z'| '0'..'9'))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|name| ORG_ELEMENT_PARSED_KEYWORDS.contains(&name.to_uppercase()));

    let backend = any()
        .filter(|c: &char| matches!(c, '-' | '_') || c.is_alphanumeric())
        .repeated()
        .at_least(1)
        .collect::<String>();

    let string_without_nl = none_of("\n\r").repeated().collect::<String>();

    let var = none_of::<&str, &str, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>(
        "[]\r\n",
    )
    .repeated()
    .at_least(1)
    .to_slice();
    let mut single_expression = Recursive::declare(); // foo / (foo) / (((foo)))
    single_expression.define(
        var.or(just("[")
            .then(single_expression.clone().repeated())
            .then(just("]"))
            .to_slice()),
    );
    let optval = single_expression.clone().repeated().to_slice();

    // fixme: without footnote reference
    let objects_parser = object_parser
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<S2>>();

    // #+KEY: VALUE(string)
    let p1 = just("#+")
        .then(key)
        .then(just(":"))
        .then(object::whitespaces())
        .then(string_without_nl)
        .then(object::newline_or_ending())
        .map_with(|(((((hash_plus, key), colon), ws), value), nl), e| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::HashPlus.into(),
                hash_plus,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &key,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                colon,
            )));

            if ws.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &value,
            )));

            match nl {
                Some(newline) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Newline.into(),
                        &newline,
                    )));
                    e.state().prev_char = newline.chars().last();
                }
                None => {
                    e.state().prev_char = value.chars().last();
                }
            }
            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::AffiliatedKeyword.into(),
                children,
            ))
        });

    // #+KEY[OPTVAL]: VALUE(string)
    let p2 = just("#+")
        .then(key_with_optvalue)
        .then(just("["))
        .then(optval.clone())
        .then(just("]"))
        .then(just(":"))
        .then(object::whitespaces())
        .then(string_without_nl)
        .then(object::newline_or_ending())
        .map_with(
            |((((((((hash_plus, key), lsb), optval), rsb), colon), ws), value), nl), e| {
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::HashPlus.into(),
                    hash_plus,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &key,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftSquareBracket.into(),
                    &lsb,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &optval,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightSquareBracket.into(),
                    &rsb,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    colon,
                )));

                if ws.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &value,
                )));

                match nl {
                    Some(newline) => {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Newline.into(),
                            &newline,
                        )));
                        e.state().prev_char = newline.chars().last();
                    }
                    None => {
                        e.state().prev_char = value.chars().last();
                    }
                }
                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::AffiliatedKeyword.into(),
                    children,
                ))
            },
        );

    // #+attr_BACKEND: VALUE
    let p3 = just("#+")
        .then(just("attr_"))
        .then(backend)
        .then(just(":"))
        .then(object::whitespaces())
        .then(string_without_nl)
        .then(object::newline_or_ending())
        .map_with(
            |((((((hash_plus, attr_), backend), colon), ws), value), nl), e| {
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::HashPlus.into(),
                    hash_plus,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &format!("{attr_}{backend}"),
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    colon,
                )));

                if ws.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &value,
                )));

                match nl {
                    Some(newline) => {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Newline.into(),
                            &newline,
                        )));
                        e.state().prev_char = newline.chars().last();
                    }
                    None => {
                        e.state().prev_char = value.chars().last();
                    }
                }
                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::AffiliatedKeyword.into(),
                    children,
                ))
            },
        );

    // #+KEY: VALUE(objects)
    let p1a = just("#+")
        .then(key_with_objects)
        .then(just(":"))
        .then(object::whitespaces())
        .then(
            objects_parser
                .clone()
                .nested_in(string_without_nl.to_slice()),
        )
        .then(object::newline_or_ending())
        .map_with(|(((((hash_plus, key), colon), ws), value), nl), e| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::HashPlus.into(),
                hash_plus,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &key,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                colon,
            )));

            if ws.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )));
            }

            for node in value {
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

            match nl {
                Some(newline) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Newline.into(),
                        &newline,
                    )));
                    e.state().prev_char = newline.chars().last();
                }
                None => {}
            }

            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::AffiliatedKeyword.into(),
                children,
            ))
        });

    // #+KEY[OPTVAL]: VALUE(objects)
    let p2a = just("#+")
        .then(key_with_objects)
        .then(just("["))
        .then(optval.clone())
        .then(just("]"))
        .then(just(":"))
        .then(object::whitespaces())
        .then(
            objects_parser
                .clone()
                .nested_in(string_without_nl.to_slice()),
        )
        .then(object::newline_or_ending())
        .map_with(
            |((((((((hash_plus, key), lsb), optval), rsb), colon), ws), value), nl), e| {
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::HashPlus.into(),
                    hash_plus,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &key,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftSquareBracket.into(),
                    &lsb,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &optval,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightSquareBracket.into(),
                    &rsb,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    colon,
                )));

                if ws.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws,
                    )));
                }

                for node in value {
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

                match nl {
                    Some(newline) => {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Newline.into(),
                            &newline,
                        )));
                        e.state().prev_char = newline.chars().last();
                    }
                    None => {}
                }

                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::AffiliatedKeyword.into(),
                    children,
                ))
            },
        );

    p2a.or(p2).or(p1a).or(p1).or(p3)
}

pub(crate) fn keyword_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    just("#+")
        .then(
            none_of(" \t:")
                .and_is(just("call").not())
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .then(just(":"))
        .then(object::whitespaces())
        .then(none_of("\n\r").repeated().collect::<String>())
        .then(object::newline_or_ending())
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(
            |((((((hash_plus, key), colon), ws), value), nl), blanklines), e| {
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::HashPlus.into(),
                    hash_plus,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &key,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    colon,
                )));

                if ws.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &value,
                )));

                match nl {
                    Some(newline) => {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Newline.into(),
                            &newline,
                        )));
                        e.state().prev_char = newline.chars().last();
                    }
                    None => {
                        e.state().prev_char = value.chars().last();
                    }
                }
                for blankline in blanklines {
                    children.push(NodeOrToken::Token(blankline));
                }

                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Keyword.into(), children))
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_nt_output;
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_keyword_01() {
        assert_eq!(
            get_parser_nt_output(keyword_parser(), r"#+key: value    "),
            r###"Keyword@0..16
  HashPlus@0..2 "#+"
  Text@2..5 "key"
  Colon@5..6 ":"
  Whitespace@6..7 " "
  Text@7..16 "value    "
"###
        );
    }

    #[test]
    fn test_affliated_keyword_01() {
        assert_eq!(
            get_parser_nt_output(
                affiliated_keyword_parser(object::standard_set_object_parser()),
                r"#+caption: value    "
            ),
            r###"AffiliatedKeyword@0..20
  HashPlus@0..2 "#+"
  Text@2..9 "caption"
  Colon@9..10 ":"
  Whitespace@10..11 " "
  Text@11..20 "value    "
"###
        );
    }

    #[test]
    fn test_affliated_keyword_02() {
        assert_eq!(
            get_parser_nt_output(
                affiliated_keyword_parser(object::standard_set_object_parser()),
                r"#+CAPTION[Short caption]: Longer caption."
            ),
            r###"AffiliatedKeyword@0..41
  HashPlus@0..2 "#+"
  Text@2..9 "CAPTION"
  LeftSquareBracket@9..10 "["
  Text@10..23 "Short caption"
  RightSquareBracket@23..24 "]"
  Colon@24..25 ":"
  Whitespace@25..26 " "
  Text@26..41 "Longer caption."
"###
        );
    }

    #[test]
    fn test_affliated_keyword_03() {
        assert_eq!(
            get_parser_nt_output(
                affiliated_keyword_parser(object::standard_set_object_parser()),
                r"#+attr_html: value"
            ),
            r###"AffiliatedKeyword@0..18
  HashPlus@0..2 "#+"
  Text@2..11 "attr_html"
  Colon@11..12 ":"
  Whitespace@12..13 " "
  Text@13..18 "value"
"###
        );
    }
    #[test]
    fn test_affliated_keyword_04() {
        assert_eq!(
            get_parser_nt_output(
                affiliated_keyword_parser(object::standard_set_object_parser()),
                r"#+CAPTION[Short caption]: *Longer* caption."
            ),
            r###"AffiliatedKeyword@0..43
  HashPlus@0..2 "#+"
  Text@2..9 "CAPTION"
  LeftSquareBracket@9..10 "["
  Text@10..23 "Short caption"
  RightSquareBracket@23..24 "]"
  Colon@24..25 ":"
  Whitespace@25..26 " "
  Bold@26..34
    Asterisk@26..27 "*"
    Text@27..33 "Longer"
    Asterisk@33..34 "*"
  Text@34..43 " caption."
"###
        );
    }
}
