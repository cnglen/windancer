//! Keyword parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};

use std::ops::Range;

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

pub(crate) fn affiliated_keyword_parser<'a>() -> impl Parser<
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

    let objects_parser = object::object_in_keyword_parser()
        .repeated()
        .at_least(1)
        .collect::<Vec<NodeOrToken<GreenNode, GreenToken>>>();

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

            children.push(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::KeywordKey.into(),
                vec![NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &key,
                ))],
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

            children.push(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::KeywordValue.into(),
                vec![NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &value,
                ))],
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

                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::KeywordKey.into(),
                    vec![NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &key,
                    ))],
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftSquareBracket.into(),
                    &lsb,
                )));

                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::KeywordOptvalue.into(),
                    vec![NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &optval,
                    ))],
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

                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::KeywordValue.into(),
                    vec![NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &value,
                    ))],
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

                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::KeywordKey.into(),
                    vec![NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &format!("{attr_}{backend}"),
                    ))],
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

                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::KeywordValue.into(),
                    vec![NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &value,
                    ))],
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

            children.push(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::KeywordKey.into(),
                vec![NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &key,
                ))],
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

            let mut children_of_value = vec![];
            for node in value {
                children_of_value.push(node);
            }
            if children_of_value.len() > 0 {
                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::KeywordValue.into(),
                    children_of_value,
                )));
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

                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::KeywordKey.into(),
                    vec![NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &key,
                    ))],
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftSquareBracket.into(),
                    &lsb,
                )));

                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::KeywordOptvalue.into(),
                    vec![NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &optval,
                    ))],
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

                let mut children_of_value = vec![];
                for node in value {
                    children_of_value.push(node);
                }
                if children_of_value.len() > 0 {
                    children.push(NodeOrToken::Node(GreenNode::new(
                        OrgSyntaxKind::KeywordValue.into(),
                        children_of_value,
                    )));
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

// find last colon(:), all previous chars are `key`, such as "#+key:with:colon: value"
fn key_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    custom::<_, &str, _, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>(|inp| {
        let remaining = inp.slice_from(std::ops::RangeFrom {
            start: &inp.cursor(),
        });

        let content: String = remaining
            .chars()
            .take_while(|c| !matches!(c, ' ' | '\t' | '\r' | '\n'))
            .collect();

        // last colon
        let last_colon = content.char_indices().rev().find(|(_, c)| matches!(c, ':'));

        let (idx, _) = last_colon.ok_or_else(|| {
            let n_char = content.chars().count();
            Rich::custom(
                SimpleSpan::from(Range {
                    start: *inp.cursor().inner(),
                    end: (inp.cursor().inner() + n_char),
                }),
                format!("keyword must be followd by a colon: '{}'", content),
            )
        })?;

        let key = content.chars().take(idx + 0).collect::<String>();
        for _ in 0..idx + 0 {
            inp.next();
        }
        Ok(key)
    })
}

pub(crate) fn keyword_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    // PEG: !whitespace any()*
    // last if not :

    let string_without_nl = none_of("\n\r").repeated().collect::<String>();
    let key_with_objects = any()
        .filter(|c: &char| matches!(c, 'a'..'z' | 'A'..'Z'| '0'..'9'))
        .repeated()
        .at_least(1)
        .collect::<String>()
        .filter(|name| ORG_ELEMENT_PARSED_KEYWORDS.contains(&name.to_uppercase()));

    let objects_parser = object::object_in_keyword_parser()
        .repeated()
        .at_least(1)
        .collect::<Vec<NodeOrToken<GreenNode, GreenToken>>>();

    // #+KEY: VALUE(string)
    let p1 = just("#+")
        .then(key_parser())
        .then(just(":"))
        .then(object::whitespaces())
        .then(string_without_nl)
        .then(object::newline_or_ending())
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(
            |((((((hash_plus, key), colon), ws), value), nl), blank_lines), e| {
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::HashPlus.into(),
                    hash_plus,
                )));

                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::KeywordKey.into(),
                    vec![NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &key,
                    ))],
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

                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::KeywordValue.into(),
                    vec![NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &value,
                    ))],
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
                for blank_line in blank_lines {
                    children.push(NodeOrToken::Token(blank_line));
                    e.state().prev_char = Some('\n');
                }

                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Keyword.into(), children))
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
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(
            |((((((hash_plus, key), colon), ws), value), nl), blank_lines), e| {
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::HashPlus.into(),
                    hash_plus,
                )));

                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::KeywordKey.into(),
                    vec![NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &key,
                    ))],
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

                let mut children_of_value = vec![];
                for node in value {
                    children_of_value.push(node);
                }
                if children_of_value.len() > 0 {
                    children.push(NodeOrToken::Node(GreenNode::new(
                        OrgSyntaxKind::KeywordValue.into(),
                        children_of_value,
                    )));
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

                for blank_line in blank_lines {
                    children.push(NodeOrToken::Token(blank_line));
                    e.state().prev_char = Some('\n');
                }

                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::AffiliatedKeyword.into(),
                    children,
                ))
            },
        );

    p1a.or(p1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_keyword_01() {
        assert_eq!(
            get_parser_output(keyword_parser(), r"#+key: value    "),
            r###"Keyword@0..16
  HashPlus@0..2 "#+"
  KeywordKey@2..5
    Text@2..5 "key"
  Colon@5..6 ":"
  Whitespace@6..7 " "
  KeywordValue@7..16
    Text@7..16 "value    "
"###
        );
    }

    #[test]
    fn test_keyword_02() {
        assert_eq!(
            get_parser_output(keyword_parser(), r"#+key:has:colons: value    "),
            r###"Keyword@0..27
  HashPlus@0..2 "#+"
  KeywordKey@2..16
    Text@2..16 "key:has:colons"
  Colon@16..17 ":"
  Whitespace@17..18 " "
  KeywordValue@18..27
    Text@18..27 "value    "
"###
        );
    }

    #[test]
    fn test_affliated_keyword_01() {
        assert_eq!(
            get_parser_output(affiliated_keyword_parser(), r"#+caption: value    "),
            r###"AffiliatedKeyword@0..20
  HashPlus@0..2 "#+"
  KeywordKey@2..9
    Text@2..9 "caption"
  Colon@9..10 ":"
  Whitespace@10..11 " "
  KeywordValue@11..20
    Text@11..20 "value    "
"###
        );
    }

    #[test]
    fn test_affliated_keyword_02() {
        assert_eq!(
            get_parser_output(
                affiliated_keyword_parser(),
                r"#+CAPTION[Short caption]: Longer caption."
            ),
            r###"AffiliatedKeyword@0..41
  HashPlus@0..2 "#+"
  KeywordKey@2..9
    Text@2..9 "CAPTION"
  LeftSquareBracket@9..10 "["
  KeywordOptvalue@10..23
    Text@10..23 "Short caption"
  RightSquareBracket@23..24 "]"
  Colon@24..25 ":"
  Whitespace@25..26 " "
  KeywordValue@26..41
    Text@26..41 "Longer caption."
"###
        );
    }

    #[test]
    fn test_affliated_keyword_03() {
        assert_eq!(
            get_parser_output(affiliated_keyword_parser(), r"#+attr_html: value"),
            r###"AffiliatedKeyword@0..18
  HashPlus@0..2 "#+"
  KeywordKey@2..11
    Text@2..11 "attr_html"
  Colon@11..12 ":"
  Whitespace@12..13 " "
  KeywordValue@13..18
    Text@13..18 "value"
"###
        );
    }
    #[test]
    fn test_affliated_keyword_04() {
        assert_eq!(
            get_parser_output(
                affiliated_keyword_parser(),
                r"#+CAPTION[Short caption]: *Longer* caption."
            ),
            r###"AffiliatedKeyword@0..43
  HashPlus@0..2 "#+"
  KeywordKey@2..9
    Text@2..9 "CAPTION"
  LeftSquareBracket@9..10 "["
  KeywordOptvalue@10..23
    Text@10..23 "Short caption"
  RightSquareBracket@23..24 "]"
  Colon@24..25 ":"
  Whitespace@25..26 " "
  KeywordValue@26..43
    Bold@26..34
      Asterisk@26..27 "*"
      Text@27..33 "Longer"
      Asterisk@33..34 "*"
    Text@34..43 " caption."
"###
        );
    }

    #[test]
    fn test_affliated_keyword_05() {
        assert_eq!(
            get_parser_output(affiliated_keyword_parser(), r"#+caption:value: value    "),
            r###"AffiliatedKeyword@0..26
  HashPlus@0..2 "#+"
  KeywordKey@2..9
    Text@2..9 "caption"
  Colon@9..10 ":"
  KeywordValue@10..26
    Text@10..26 "value: value    "
"###
        );
    }
}
