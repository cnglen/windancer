//! link parser, including angle/plain/regular link
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use std::ops::Range;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

use phf::phf_set;

pub(crate) static LINK_PROTOCOLS: phf::Set<&'static str> = phf_set! {
    "treemacs", "eww", "rmail", "mhe", "irc", "info", "gnus", "docview",
    "bibtex", "bbdb", "w3m", "doi", "attachment", "id", "file+sys",
    "file+emacs", "shell", "news", "mailto", "https", "http", "ftp",
    "help", "file", "elisp"
};

/// PROTOCOL: A string which is one of the link type strings in org-link-parameters
// - Consume only verified to reduce # of backtracks: rollbackstate.on_save 1761022 -> 1661398 (94.3%)
// #[allow(unused)]
use chumsky::input::InputRef;
pub(crate) fn protocol<'a, C: 'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>> + Clone
{
    custom(
        |inp: &mut InputRef<
            'a,
            '_,
            &'a str,
            extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
        >| {
            let before = &inp.cursor();
            let remaining = inp.slice_from(std::ops::RangeFrom {
                start: &inp.cursor(),
            });

            let protocol: String = remaining
                .chars()
                .take_while(|c| c.is_ascii_lowercase() || *c == '+')
                .collect();

            if protocol.is_empty() || !LINK_PROTOCOLS.contains(protocol.as_str()) {
                return Err(Rich::custom(
                    inp.span_since(before),
                    format!("invalid protocol: '{}'", protocol),
                ));
            }

            for _ in 0..protocol.len() {
                inp.next();
            }

            Ok(protocol)
        },
    )
}
// pub(crate) fn protocol_old<'a, C: 'a>()
// -> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>> + Clone
// {
//     any()
//         .filter(|c: &char| matches!(c, 'a'..'z' | '+'))
//         .repeated()
//         .at_least(1)
//         .collect::<String>()
//         .filter(|e| LINK_PROTOCOLS.contains(e))
// }

// pathplain parser
fn path_plain_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>> + Clone
{
    custom::<_, &str, _, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>>(|inp| {
        let remaining = inp.slice_from(std::ops::RangeFrom {
            start: &inp.cursor(),
        });

        let content_: String = remaining
            .chars()
            // .take_while(|c| !matches!(c, ' ' | '\t' | '\n' | '[' | ']' | '<' | '>' | '(' | ')'))
            .take_while(|c| !matches!(c, ' ' | '\t' | '\n' | '[' | ']' | '<' | '>')) // () is allowed for parenthesis-wrapped
            .collect();

        let mut content_chars_ = content_.chars();
        let mut content = String::new();
        let mut paren_depth = 0;
        let mut max_paren_depth = 0;
        while let Some(c) = content_chars_.next() {
            match c {
                // 遇到左括号，增加深度
                '(' => {
                    paren_depth += 1;
                    max_paren_depth = max_paren_depth.max(paren_depth);

                    // 检查嵌套深度
                    if paren_depth > 2 {
                        return Err(Rich::custom(
                            SimpleSpan::from(Range {
                                start: *inp.cursor().inner() - 1,
                                end: *inp.cursor().inner(),
                            }),
                            "parentheses nesting depth exceeds 2 levels",
                        ));
                    }
                    content.push(c);
                }
                // 遇到右括号，减少深度
                ')' => {
                    if paren_depth == 0 {
                        // 不匹配的右括号，停止解析
                        break;
                    }
                    paren_depth -= 1;
                    content.push(c);
                }
                // 遇到其他终止字符（在括号外）
                c if paren_depth == 0
                    && matches!(c, ' ' | '\t' | '\n' | '[' | ']' | '<' | '>' | ')') =>
                {
                    // 回退这个字符，停止解析
                    break;
                }
                // 普通字符
                _ => {
                    content.push(c);
                }
            }
        }

        let n_char = content.chars().count();

        // 检查括号是否匹配
        if paren_depth != 0 {
            return Err(Rich::custom(
                SimpleSpan::from(Range {
                    start: *inp.cursor().inner(),
                    end: (inp.cursor().inner() + n_char),
                }),
                "unclosed parentheses",
            ));
        }

        // 检查内容是否为空
        if content.is_empty() {
            return Err(Rich::custom(
                SimpleSpan::from(Range {
                    start: *inp.cursor().inner(),
                    end: (inp.cursor().inner() + n_char),
                }),
                "empty path plain",
            ));
        }

        let maybe_final = content.char_indices().rev().find(|(_, c)| {
            matches!(c, ')') || (!(c.is_ascii_punctuation() || matches!(c, ' ' | '\t' | '\n')))
        });
        // .find(|(_, c)| !(c.is_ascii_punctuation() || matches!(c, ' ' | '\t' | '\n')));

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
    .boxed()
}

/// plain link parser
pub(crate) fn plain_link_parser<'a, C: 'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
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

                    Ok(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                        OrgSyntaxKind::PlainLink.into(),
                        vec![
                            NodeOrToken::Token(GreenToken::new(
                                OrgSyntaxKind::Text.into(),
                                &protocol,
                            )),
                            NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Colon.into(), colon)),
                            NodeOrToken::Token(GreenToken::new(
                                OrgSyntaxKind::Text.into(),
                                &pathplain,
                            )),
                        ],
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
        .boxed()
}

/// angle link parser
pub(crate) fn angle_link_parser<'a, C: 'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
    let path_angle = none_of(">") // . is permitted: orgmode.org, xx@xx.com
        .repeated()
        .at_least(1)
        .to_slice();

    just("<")
        .then(protocol())
        .then(just(":"))
        .then(path_angle)
        .then(just(">"))
        .map(
            |((((left_angle, protocol), colon), path_angle), right_angle)| {
                NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::AngleLink.into(),
                    vec![
                        NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::LeftAngleBracket.into(),
                            left_angle,
                        )),
                        NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Text.into(), &protocol)),
                        NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Colon.into(), colon)),
                        NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Text.into(), path_angle)),
                        NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::RightAngleBracket.into(),
                            right_angle,
                        )),
                    ],
                ))
            },
        )
        .boxed()
}

/// regular link parser
pub(crate) fn regular_link_parser<'a, C: 'a>(
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
    let minimal_and_other_objects_parser = object_parser
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<NodeOrToken<GreenNode, GreenToken>>>();

    let description = just("[")
        .then(
            minimal_and_other_objects_parser.nested_in(
                // any().and_is(just("]]").not()) // slow version
                none_of("]")
                    .to_slice()
                    .or(just("]").then(none_of("]")).to_slice())
                    .repeated()
                    .to_slice(),
            ),
        )
        .then(just("]"))
        .or_not()
        .map(|description| match description {
            None => None,

            Some(((lbracket, content), rbracket)) => {
                Some(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::LinkDescription.into(),
                    {
                        let mut children = Vec::with_capacity(2 + content.len());
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::LeftSquareBracket.into(),
                            lbracket,
                        )));
                        children.extend(content);
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::RightSquareBracket.into(),
                            rbracket,
                        )));
                        children
                    },
                )))
            }
        });

    let normal_char = none_of("[]\\");
    let escape_char = just('\\').then(one_of("[]\\")).map(|(_, c)| c);
    let string_without_brackets = choice((normal_char, escape_char))
        .repeated()
        .at_least(1)
        .to_slice();

    let filename = (just("./").or(just("/")))
        .then(
            any()
                .filter(|c: &char| {
                    (c.is_alphanumeric() || matches!(c, '.' | '/' | ':' | '@'))
                        && (*c != '[')
                        && (*c != ']')
                })
                .repeated()
                .at_least(1),
        )
        .to_slice();

    let protocol_pathinner = protocol()
        .then(just(":"))
        .then(just("//").or_not())
        .then(string_without_brackets)
        .to_slice();

    let id = just("id:")
        .then(one_of("0123456789abcdef-").repeated().at_least(1))
        .to_slice();

    let custom_id = just("#").then(string_without_brackets).to_slice();

    let codef_ref = just("(")
        .then(string_without_brackets)
        .then(just(")"))
        .to_slice();

    let fuzzy = string_without_brackets;

    let pathreg = just("[")
        .then(choice((
            filename,
            protocol_pathinner,
            id,
            custom_id,
            codef_ref,
            fuzzy,
        )))
        .then(just("]"))
        .map(|((lbracket, path), rbracket)| {
            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::LinkPath.into(),
                vec![
                    NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::LeftSquareBracket.into(),
                        lbracket,
                    )),
                    NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Text.into(), path)),
                    NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::RightSquareBracket.into(),
                        rbracket,
                    )),
                ],
            ))
        });

    just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>("[")
        .then(pathreg)
        .then(description)
        .then(just("]"))
        .then(object::newline().or_not())
        .map_with(
            |((((lbracket, path), maybe_desc), rbracket), maybe_newline), e| {
                let mut children = Vec::new();
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftSquareBracket.into(),
                    lbracket,
                )));

                children.push(path);

                children.extend(maybe_desc.into_iter());

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightSquareBracket.into(),
                    rbracket,
                )));

                e.state().prev_char = rbracket.chars().last();
                children.extend(maybe_newline.into_iter().map(|nl| {
                    e.state().prev_char = nl.chars().last();
                    NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Newline.into(), &nl))
                }));

                NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::Link.into(),
                    children,
                ))
            },
        )
        .boxed()
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parsers_output;
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_plain_link_01_http() {
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"https://foo.bar"),
            r###"Root@0..15
  PlainLink@0..15
    Text@0..5 "https"
    Colon@5..6 ":"
    Text@6..15 "//foo.bar"
"###
        );
    }

    #[test]
    fn test_plain_link_02_http() {
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"https://foo...bar"),
            r###"Root@0..17
  PlainLink@0..17
    Text@0..5 "https"
    Colon@5..6 ":"
    Text@6..17 "//foo...bar"
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"https://bar..."),
            r###"Root@0..14
  PlainLink@0..11
    Text@0..5 "https"
    Colon@5..6 ":"
    Text@6..11 "//bar"
  Text@11..14 "..."
"###
        );
    }

    #[test]
    fn test_plain_link_03_http() {
        // 1 layer nested
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"https://foo(bar)"),
            r###"Root@0..16
  PlainLink@0..16
    Text@0..5 "https"
    Colon@5..6 ":"
    Text@6..16 "//foo(bar)"
"###
        );

        // 2 layer nested
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"https://foo(bar(dfk))"),
            r###"Root@0..21
  PlainLink@0..21
    Text@0..5 "https"
    Colon@5..6 ":"
    Text@6..21 "//foo(bar(dfk))"
"###
        );

        // 3 layer nested
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"https://f(b(d(q)))"),
            r###"Root@0..18
  Text@0..18 "https://f(b(d(q)))"
"###
        );

        // not matched round bracket
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"https://foo(bar"),
            r###"Root@0..15
  Text@0..15 "https://foo(bar"
"###
        );
    }

    #[test]
    fn test_regular_link_01_http() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"[[https://orgmode.org][The Org project homepage]]
[[file:orgmanual.org]]
[[Regular links]]
"
            ),
            r###"Root@0..91
  Link@0..50
    LeftSquareBracket@0..1 "["
    LinkPath@1..22
      LeftSquareBracket@1..2 "["
      Text@2..21 "https://orgmode.org"
      RightSquareBracket@21..22 "]"
    LinkDescription@22..48
      LeftSquareBracket@22..23 "["
      Text@23..47 "The Org project homepage"
      RightSquareBracket@47..48 "]"
    RightSquareBracket@48..49 "]"
    Newline@49..50 "\n"
  Link@50..73
    LeftSquareBracket@50..51 "["
    LinkPath@51..71
      LeftSquareBracket@51..52 "["
      Text@52..70 "file:orgmanual.org"
      RightSquareBracket@70..71 "]"
    RightSquareBracket@71..72 "]"
    Newline@72..73 "\n"
  Link@73..91
    LeftSquareBracket@73..74 "["
    LinkPath@74..89
      LeftSquareBracket@74..75 "["
      Text@75..88 "Regular links"
      RightSquareBracket@88..89 "]"
    RightSquareBracket@89..90 "]"
    Newline@90..91 "\n"
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"[[http://orgmode.org/]]"),
            r##"Root@0..23
  Link@0..23
    LeftSquareBracket@0..1 "["
    LinkPath@1..22
      LeftSquareBracket@1..2 "["
      Text@2..21 "http://orgmode.org/"
      RightSquareBracket@21..22 "]"
    RightSquareBracket@22..23 "]"
"##
        );
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"[[https://orgmode.org/]]"),
            r##"Root@0..24
  Link@0..24
    LeftSquareBracket@0..1 "["
    LinkPath@1..23
      LeftSquareBracket@1..2 "["
      Text@2..22 "https://orgmode.org/"
      RightSquareBracket@22..23 "]"
    RightSquareBracket@23..24 "]"
"##
        );
    }
    #[test]
    fn test_regular_link_02_doi() {
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"[[doi:10.1000/182]]"),
            r##"Root@0..19
  Link@0..19
    LeftSquareBracket@0..1 "["
    LinkPath@1..18
      LeftSquareBracket@1..2 "["
      Text@2..17 "doi:10.1000/182"
      RightSquareBracket@17..18 "]"
    RightSquareBracket@18..19 "]"
"##
        );
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"[[file:/home/dominik/images/jupiter.jpg]]"
            ),
            r##"Root@0..41
  Link@0..41
    LeftSquareBracket@0..1 "["
    LinkPath@1..40
      LeftSquareBracket@1..2 "["
      Text@2..39 "file:/home/dominik/im ..."
      RightSquareBracket@39..40 "]"
    RightSquareBracket@40..41 "]"
"##
        );
    }

    #[test]
    fn test_regular_link_03_file() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"[[/home/dominik/images/jupiter.jpg]]"
            ),
            r##"Root@0..36
  Link@0..36
    LeftSquareBracket@0..1 "["
    LinkPath@1..35
      LeftSquareBracket@1..2 "["
      Text@2..34 "/home/dominik/images/ ..."
      RightSquareBracket@34..35 "]"
    RightSquareBracket@35..36 "]"
"##
        );

        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"[[file:papers/last.pdf]]"),
            r##"Root@0..24
  Link@0..24
    LeftSquareBracket@0..1 "["
    LinkPath@1..23
      LeftSquareBracket@1..2 "["
      Text@2..22 "file:papers/last.pdf"
      RightSquareBracket@22..23 "]"
    RightSquareBracket@23..24 "]"
"##
        );
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"[[./papers/last.pdf]]"),
            r##"Root@0..21
  Link@0..21
    LeftSquareBracket@0..1 "["
    LinkPath@1..20
      LeftSquareBracket@1..2 "["
      Text@2..19 "./papers/last.pdf"
      RightSquareBracket@19..20 "]"
    RightSquareBracket@20..21 "]"
"##
        );
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"[[file:/ssh:me@some.where:papers/last.pdf]]"
            ),
            r##"Root@0..43
  Link@0..43
    LeftSquareBracket@0..1 "["
    LinkPath@1..42
      LeftSquareBracket@1..2 "["
      Text@2..41 "file:/ssh:me@some.whe ..."
      RightSquareBracket@41..42 "]"
    RightSquareBracket@42..43 "]"
"##
        );
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"[[/ssh:me@some.where:papers/last.pdf]]"
            ),
            r##"Root@0..38
  Link@0..38
    LeftSquareBracket@0..1 "["
    LinkPath@1..37
      LeftSquareBracket@1..2 "["
      Text@2..36 "/ssh:me@some.where:pa ..."
      RightSquareBracket@36..37 "]"
    RightSquareBracket@37..38 "]"
"##
        );
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"[[file:sometextfile::NNN]]"
            ),
            r##"Root@0..26
  Link@0..26
    LeftSquareBracket@0..1 "["
    LinkPath@1..25
      LeftSquareBracket@1..2 "["
      Text@2..24 "file:sometextfile::NNN"
      RightSquareBracket@24..25 "]"
    RightSquareBracket@25..26 "]"
"##
        );
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"[[file:projects.org]]"),
            r##"Root@0..21
  Link@0..21
    LeftSquareBracket@0..1 "["
    LinkPath@1..20
      LeftSquareBracket@1..2 "["
      Text@2..19 "file:projects.org"
      RightSquareBracket@19..20 "]"
    RightSquareBracket@20..21 "]"
"##
        );
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"[[file:projects.org::some words]]"
            ),
            r##"Root@0..33
  Link@0..33
    LeftSquareBracket@0..1 "["
    LinkPath@1..32
      LeftSquareBracket@1..2 "["
      Text@2..31 "file:projects.org::so ..."
      RightSquareBracket@31..32 "]"
    RightSquareBracket@32..33 "]"
"##
        );

        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"[[file:projects.org::*task title]]"
            ),
            r##"Root@0..34
  Link@0..34
    LeftSquareBracket@0..1 "["
    LinkPath@1..33
      LeftSquareBracket@1..2 "["
      Text@2..32 "file:projects.org::*t ..."
      RightSquareBracket@32..33 "]"
    RightSquareBracket@33..34 "]"
"##
        );
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"[[file:projects.org::#custom-id]]"
            ),
            r##"Root@0..33
  Link@0..33
    LeftSquareBracket@0..1 "["
    LinkPath@1..32
      LeftSquareBracket@1..2 "["
      Text@2..31 "file:projects.org::#c ..."
      RightSquareBracket@31..32 "]"
    RightSquareBracket@32..33 "]"
"##
        );
    }
    #[test]
    fn test_regular_link_04_attachment() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"[[attachment:projects.org]]"
            ),
            r##"Root@0..27
  Link@0..27
    LeftSquareBracket@0..1 "["
    LinkPath@1..26
      LeftSquareBracket@1..2 "["
      Text@2..25 "attachment:projects.org"
      RightSquareBracket@25..26 "]"
    RightSquareBracket@26..27 "]"
"##
        );
        //         assert_eq!(
        //             get_parsers_output(
        //                 object::objects_parser::<()>(),
        //                 r"[[attachment:projects.org::some words]]"
        //             ),
        //             r##"
        // "##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[docview:papers/last.pdf::NNN]]"),
        //             r##"
        // "##
        //         );
        //         assert_eq!(
        //             get_parsers_output(
        //                 object::objects_parser::<()>(),
        //                 r"[[id:B7423F4D-2E8A-471B-8810-C40F074717E9]]"
        //             ),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(
        //                 object::objects_parser::<()>(),
        //                 r"[[id:B7423F4D-2E8A-471B-8810-C40F074717E9::*task]]"
        //             ),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[news:comp.emacs]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[mailto:adent@galaxy.net]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[mhe:folder]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[mhe:folder#id]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[rmail:folder]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[rmail:folder#id]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[gnus:group]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[gnus:group#id]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[bbdb:R.*Stallman]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[irc:/irc.com/#emacs/bob]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[help:org-store-link]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[info:org#External links]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[shell:ls *.org]]"),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(
        //                 object::objects_parser::<()>(),
        //                 r##"[[elisp:(find-file "Elisp.org")]]"##
        //             ),
        //             r##""##
        //         );
        //         assert_eq!(
        //             get_parsers_output(object::objects_parser::<()>(), r"[[elisp:org-agenda]]"),
        //             r##""##
        //         );
    }

    #[test]
    fn test_regular_link_97_escape() {
        // allow ] in DESCRIPTION
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"[[http://foo.bar][jac]k]]"),
            r##"Root@0..25
  Link@0..25
    LeftSquareBracket@0..1 "["
    LinkPath@1..17
      LeftSquareBracket@1..2 "["
      Text@2..16 "http://foo.bar"
      RightSquareBracket@16..17 "]"
    LinkDescription@17..24
      LeftSquareBracket@17..18 "["
      Text@18..23 "jac]k"
      RightSquareBracket@23..24 "]"
    RightSquareBracket@24..25 "]"
"##
        );
    }

    #[test]
    fn test_regular_link_98_escape() {
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"[[http://foo.ba\]r]]"),
            r##"Root@0..20
  Link@0..20
    LeftSquareBracket@0..1 "["
    LinkPath@1..19
      LeftSquareBracket@1..2 "["
      Text@2..18 "http://foo.ba\\]r"
      RightSquareBracket@18..19 "]"
    RightSquareBracket@19..20 "]"
"##
        );
    }

    #[test]
    fn test_regular_link_99_objects() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(),
                r"[[http://foo.bar][\alpha $a+b$ the_subscript *foo* bar {{{title}}} https:://foo.bar <https://angle.bar> <<not-supported-target>> txt]]"
            ),
            r##"Root@0..134
  Link@0..134
    LeftSquareBracket@0..1 "["
    LinkPath@1..17
      LeftSquareBracket@1..2 "["
      Text@2..16 "http://foo.bar"
      RightSquareBracket@16..17 "]"
    LinkDescription@17..133
      LeftSquareBracket@17..18 "["
      Entity@18..24
        BackSlash@18..19 "\\"
        EntityName@19..24 "alpha"
      Text@24..25 " "
      LatexFragment@25..30
        Dollar@25..26 "$"
        Text@26..29 "a+b"
        Dollar@29..30 "$"
      Text@30..34 " the"
      Subscript@34..44
        Caret@34..35 "_"
        Text@35..44 "subscript"
      Text@44..45 " "
      Bold@45..50
        Asterisk@45..46 "*"
        Text@46..49 "foo"
        Asterisk@49..50 "*"
      Text@50..55 " bar "
      Macro@55..66
        LeftCurlyBracket3@55..58 "{{{"
        MacroName@58..63 "title"
        RightCurlyBracket3@63..66 "}}}"
      Text@66..67 " "
      PlainLink@67..83
        Text@67..72 "https"
        Colon@72..73 ":"
        Text@73..83 "://foo.bar"
      Text@83..84 " "
      AngleLink@84..103
        LeftAngleBracket@84..85 "<"
        Text@85..90 "https"
        Colon@90..91 ":"
        Text@91..102 "//angle.bar"
        RightAngleBracket@102..103 ">"
      Text@103..132 " <<not-supported-targ ..."
      RightSquareBracket@132..133 "]"
    RightSquareBracket@133..134 "]"
"##
        );
    }
}
