//! Keyword parser
use crate::parser::object::blank_line_parser;
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use phf::phf_set;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;

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

// affliated keyword is NOT a element, it's part of some element.
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

    Parser::boxed(choice((p2a, p2, p1a, p1, p3)))
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

// element_parser: <element with affiliated word>
pub(crate) fn keyword_parser<'a>(
    element_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone
    + 'a,
) -> impl Parser<
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

    // FIXME: better method? element vs blankline?
    // #+KEY: VALUE(string)
    let p1_part1 = just("#+")
        .then(key_parser())
        .then(just(":"))
        .then(object::whitespaces())
        .then(string_without_nl);

    // part + end()
    // part + \n + end()
    // part + \n + blankline*
    // (part + \n) !(element_with_affiliated_keywords)
    let p1 = choice((
        p1_part1.clone().then(end().to(None)),
        p1_part1.clone().then(
            just('\n')
                .map(|c| Some(String::from(c)))
                .then(end())
                .to_slice()
                .to(Some(String::from('\n'))),
        ),
        p1_part1
            .clone()
            .then(just('\n').map(|c| Some(String::from(c))))
            // .map(|s|{println!("dbg: s={s:?}"); s})
            .and_is(
                element_parser
                    .clone()
                    // .map(|s|{println!("dbg@and_is: s={s:?}"); s})
                    .not(),
            ),
        p1_part1
            .clone()
            .then(object::newline_or_ending())
            .then_ignore(blank_line_parser().repeated().at_least(1).rewind()),
    ))
    .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
    // .map(|s| {
    //     println!("keyword_parser@s2={s:?}");
    //     s
    // })
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
    let p1a_part1 = just("#+")
        .then(key_with_objects)
        .then(just(":"))
        .then(object::whitespaces())
        .then(
            objects_parser
                .clone()
                .nested_in(string_without_nl.to_slice()),
        );

    let p1a = choice((
        p1a_part1.clone().then(end().to(None)),
        p1a_part1.clone().then(
            just('\n')
                .map(|c| Some(String::from(c)))
                .then(end())
                .to_slice()
                .to(Some(String::from('\n'))),
        ),
        p1a_part1
            .clone()
            .then(just('\n').map(|c| Some(String::from(c))))
            .and_is(element_parser.clone().not()),
        p1a_part1
            .clone()
            .then(object::newline_or_ending())
            .then_ignore(blank_line_parser().repeated().at_least(1).rewind()),
    ))
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

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Keyword.into(), children))
        },
    );

    Parser::boxed(choice((p1a, p1)))
    // choice((p1a, p1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::{get_parser_output, get_parsers_output};
    use crate::parser::element;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_keyword_01() {
        assert_eq!(
            get_parser_output(
                keyword_parser(element::element_in_keyword_parser()),
                r"#+key: value    "
            ),
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
    fn test_keyword_91() {
        assert_eq!(
            get_parser_output(
                keyword_parser(element::element_in_keyword_parser()),
                r"#+title: org test

"
            ),
            r###"Keyword@0..19
  HashPlus@0..2 "#+"
  KeywordKey@2..7
    Text@2..7 "title"
  Colon@7..8 ":"
  Whitespace@8..9 " "
  KeywordValue@9..17
    Text@9..17 "org test"
  Newline@17..18 "\n"
  BlankLine@18..19 "\n"
"###
        );
    }

    #[test]
    fn test_keyword_02() {
        assert_eq!(
            get_parser_output(
                keyword_parser(element::element_in_keyword_parser()),
                r"#+key:has:colons: value    "
            ),
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

    #[test]
    fn test_affliated_keyword_06() {
        let input = r##"#+caption: export block test
#+begin_export html
<span style="color:green;">hello org</span>
#+end_export
"##;

        assert_eq!(
            get_parsers_output(element::elements_parser(), input),
            r###"Root@0..106
  ExportBlock@0..106
    AffiliatedKeyword@0..29
      HashPlus@0..2 "#+"
      KeywordKey@2..9
        Text@2..9 "caption"
      Colon@9..10 ":"
      Whitespace@10..11 " "
      KeywordValue@11..28
        Text@11..28 "export block test"
      Newline@28..29 "\n"
    BlockBegin@29..49
      Text@29..37 "#+begin_"
      Text@37..43 "EXPORT"
      Whitespace@43..44 " "
      Text@44..48 "html"
      Newline@48..49 "\n"
    BlockContent@49..93
      Text@49..93 "<span style=\"color:gr ..."
    BlockEnd@93..106
      Text@93..99 "#+end_"
      Text@99..105 "EXPORT"
      Newline@105..106 "\n"
"###,
            "<affiliated keyword> is immediately preceding a <export block>"
        );
    }

    #[test]
    fn test_affliated_keyword_07() {
        let input = r##"#+caption: export block test

#+begin_export html
<span style="color:green;">hello org</span>
#+end_export
"##;

        assert_eq!(
            get_parsers_output(element::elements_parser(), input),
            r###"Root@0..107
  Keyword@0..30
    HashPlus@0..2 "#+"
    KeywordKey@2..9
      Text@2..9 "caption"
    Colon@9..10 ":"
    Whitespace@10..11 " "
    KeywordValue@11..28
      Text@11..28 "export block test"
    Newline@28..29 "\n"
    BlankLine@29..30 "\n"
  ExportBlock@30..107
    BlockBegin@30..50
      Text@30..38 "#+begin_"
      Text@38..44 "EXPORT"
      Whitespace@44..45 " "
      Text@45..49 "html"
      Newline@49..50 "\n"
    BlockContent@50..94
      Text@50..94 "<span style=\"color:gr ..."
    BlockEnd@94..107
      Text@94..100 "#+end_"
      Text@100..106 "EXPORT"
      Newline@106..107 "\n"
"###,
            "<affiliated keyword> should be immediately preceding a valid element, or it will be parsed as <keyword>"
        );
    }

    #[test]
    fn test_affliated_keyword_08() {
        let input = r##"#+caption: export block test
a paragraph
"##;

        assert_eq!(
            get_parsers_output(element::elements_parser(), input),
            r###"Root@0..41
  Paragraph@0..41
    AffiliatedKeyword@0..29
      HashPlus@0..2 "#+"
      KeywordKey@2..9
        Text@2..9 "caption"
      Colon@9..10 ":"
      Whitespace@10..11 " "
      KeywordValue@11..28
        Text@11..28 "export block test"
      Newline@28..29 "\n"
    Text@29..41 "a paragraph\n"
"###,
            "<affiliated keyword> is immediately preceding a <paragraph>"
        );
    }

    #[test]
    fn test_affliated_keyword_09() {
        let input = r##"#+caption: export block test
#+key: value
a paragraph
"##;

        assert_eq!(
            get_parsers_output(element::elements_parser(), input),
            r###"Root@0..54
  Keyword@0..29
    HashPlus@0..2 "#+"
    KeywordKey@2..9
      Text@2..9 "caption"
    Colon@9..10 ":"
    Whitespace@10..11 " "
    KeywordValue@11..28
      Text@11..28 "export block test"
    Newline@28..29 "\n"
  Keyword@29..42
    HashPlus@29..31 "#+"
    KeywordKey@31..34
      Text@31..34 "key"
    Colon@34..35 ":"
    Whitespace@35..36 " "
    KeywordValue@36..41
      Text@36..41 "value"
    Newline@41..42 "\n"
  Paragraph@42..54
    Text@42..54 "a paragraph\n"
"###,
            "<keyword> is immediately preceding a <paragraph>"
        );
    }
}
