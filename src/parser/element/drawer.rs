//! Drawer parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, element, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;

use crate::parser::object::just_case_insensitive;

fn name_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>> + Clone
{
    custom::<_, &str, _, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>>(|inp| {
        let remaining = inp.slice_from(std::ops::RangeFrom {
            start: &inp.cursor(),
        });

        let content: String = remaining
            .chars()
            .take_while(|c| !matches!(c, ' ' | '\t' | '\r' | '\n'))
            .collect();

        let maybe_final = content
            .char_indices()
            .rev()
            .find(|(_, c)| !matches!(c, '+' | ':'));

        let (idx, _) = maybe_final.ok_or_else(|| {
            let n_char = content.chars().count();
            Rich::custom(
                SimpleSpan::from(Range {
                    start: *inp.cursor().inner(),
                    end: (inp.cursor().inner() + n_char),
                }),
                format!("node_property: name_parser error: '{}'", content),
            )
        })?;

        let name = content.chars().take(idx + 1).collect::<String>();
        for _ in 0..idx + 1 {
            inp.next();
        }
        Ok(name)
    })
}

pub(crate) fn node_property_parser<'a, C: 'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
    let name = name_parser();
    let value = none_of(object::CRLF).repeated().to_slice();
    let blank_lines = object::blank_line_parser().repeated().collect::<Vec<_>>();
    object::whitespaces()
        .then(just(":"))
        .then(name)
        .then(just("+").or_not())
        .then(just(":"))
        .then(object::whitespaces())
        .then(value)
        .then(object::newline())
        .then(blank_lines)
        .map(
            |(
                (((((((ws0, colon1), name), maybe_plus), colon), ws1), value), newline),
                blank_lines,
            )| {
                let mut children = Vec::with_capacity(8 + blank_lines.len());
                if !ws0.is_empty() {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        ws0,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    colon1,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &name,
                )));

                if let Some(plus) = maybe_plus {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Plus.into(),
                        plus,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    colon,
                )));

                if !ws1.is_empty() {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        ws1,
                    )));
                }

                if !value.is_empty() {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        value,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    newline,
                )));

                children.extend(blank_lines.into_iter().map(NodeOrToken::Token));

                NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::NodeProperty.into(),
                    children,
                ))
            },
        )
}

pub(crate) fn property_drawer_parser<'a, C: 'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
    let begin_row = object::whitespaces()
        .then(just_case_insensitive(":properties:"))
        .then(object::whitespaces())
        .then(object::newline());

    let end_row = object::whitespaces()
        .then(just_case_insensitive(":end:"))
        .then(object::whitespaces())
        .then(object::newline());

    let blank_lines = object::blank_line_parser().repeated().collect::<Vec<_>>();

    begin_row
        .then(blank_lines.clone())
        .then(
            node_property_parser()
                .and_is(end_row.clone().ignored().not())
                .repeated()
                .collect::<Vec<_>>(),
        )
        .then(end_row)
        .then(blank_lines)
        .map(
            |(
                (
                    (((((ws1, properties), ws2), nl1), start_blank_lines), contents),
                    (((ws3, end), ws4), nl2),
                ),
                blank_lines,
            )| {
                let mut children = Vec::with_capacity(
                    6 + start_blank_lines.len() + contents.len() + blank_lines.len(),
                );
                if !ws1.is_empty() {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        ws1,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    properties,
                )));

                if !ws2.is_empty() {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        ws2,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    nl1,
                )));

                children.extend(start_blank_lines.into_iter().map(NodeOrToken::Token));
                children.extend(contents);

                if !ws3.is_empty() {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        ws3,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    end,
                )));

                if !ws4.is_empty() {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        ws4,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    nl2,
                )));

                children.extend(blank_lines.into_iter().map(NodeOrToken::Token));

                NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::PropertyDrawer.into(),
                    children,
                ))
            },
        )
        .boxed()
}

pub(crate) fn drawer_parser<'a, C: 'a>(
    element_parser: impl Parser<
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
    let affiliated_keywords = element::keyword::affiliated_keyword_parser()
        .repeated()
        .collect::<Vec<_>>();

    let drawer_name_row = object::whitespaces()
        .then(just(":"))
        .then(
            any()
                .filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '-'))
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .then(just(":"))
        .then(object::whitespaces())
        .then(object::newline())
        .map(|(((((ws1, c1), name), c2), ws2), nl)| {
            // println!(
            //     "drawer begin row: ws1={}, c1={}, name={}, c2={}, ws2={}, nl={}",
            //     ws1, c1, name, c2, ws2, nl
            // );
            let mut tokens = Vec::with_capacity(6);
            if !ws1.is_empty() {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }
            tokens.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                c1,
            )));
            tokens.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &name,
            )));
            tokens.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                c2,
            )));
            if !ws2.is_empty() {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws2,
                )));
            }
            tokens.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Newline.into(),
                nl,
            )));

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::DrawerBegin.into(),
                tokens,
            ))
        });

    let drawer_end_row = object::whitespaces()
        .then(object::just_case_insensitive(":end:"))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .map(|(((ws1, end), ws2), nl)| {
            let mut tokens = Vec::with_capacity(4);

            if !ws1.is_empty() {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    ws1,
                )));
            }
            tokens.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &end,
            )));
            if !ws2.is_empty() {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    ws2,
                )));
            }
            if let Some(_nl) = nl {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    _nl,
                )));
            }

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::DrawerEnd.into(),
                tokens,
            ))
        });

    let drawer_content_inner = object::line_parser()
        .or(object::blank_line_str_parser())
        .and_is(drawer_end_row.clone().ignored().not())
        .and_is(just("*").ignored().not()) // fixme: use heading row?
        .repeated()
        .to_slice();

    let drawer_content = element_parser
        .repeated()
        .collect::<Vec<_>>()
        .nested_in(drawer_content_inner)
        .map(|children| {
            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::DrawerContent.into(),
                children,
            ))
        });
    let blank_lines = object::blank_line_parser().repeated().collect::<Vec<_>>();

    affiliated_keywords
        .then(drawer_name_row)
        .then(blank_lines.clone())
        .then(drawer_content)
        .then(drawer_end_row)
        .then(blank_lines)
        .map(
            |(((((keywords, begin), start_blank_lines), content), end), blank_lines)| {
                let mut children = Vec::with_capacity(
                    3 + keywords.len() + start_blank_lines.len() + blank_lines.len(),
                );

                children.extend(keywords);
                children.push(begin);
                children.extend(start_blank_lines.into_iter().map(NodeOrToken::Token));
                children.push(content);
                children.push(end);
                children.extend(blank_lines.into_iter().map(NodeOrToken::Token));

                NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::Drawer.into(),
                    children,
                ))
            },
        )
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use crate::parser::element;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_drawer_01() {
        assert_eq!(
            get_parser_output(
                drawer_parser(element::element_in_drawer_parser::<()>()),
                r##":a:
contents :end:
:end:
"##
            ),
            r###"Drawer@0..25
  DrawerBegin@0..4
    Colon@0..1 ":"
    Text@1..2 "a"
    Colon@2..3 ":"
    Newline@3..4 "\n"
  DrawerContent@4..19
    Paragraph@4..19
      Text@4..19 "contents :end:\n"
  DrawerEnd@19..25
    Text@19..24 ":end:"
    Newline@24..25 "\n"
"###
        );
    }

    #[test]
    #[should_panic]
    fn test_drawer_02() {
        get_parser_output(
            drawer_parser(element::element_in_drawer_parser::<()>()),
            r##":a:
:b:
b
:end:
:end:
"##,
        );
    }

    #[test]
    fn test_drawer_03() {
        assert_eq!(
            get_parser_output(
                drawer_parser(element::element_in_drawer_parser::<()>()),
                r##":a:
#+BEGIN_SRC python
print("hello");
#+END_SRC
:end:
"##
            ),
            r###"Drawer@0..55
  DrawerBegin@0..4
    Colon@0..1 ":"
    Text@1..2 "a"
    Colon@2..3 ":"
    Newline@3..4 "\n"
  DrawerContent@4..49
    SrcBlock@4..49
      BlockBegin@4..23
        Text@4..12 "#+BEGIN_"
        Text@12..15 "SRC"
        Whitespace@15..16 " "
        SrcBlockLanguage@16..22 "python"
        Newline@22..23 "\n"
      BlockContent@23..39
        Text@23..39 "print(\"hello\");\n"
      BlockEnd@39..49
        Text@39..45 "#+END_"
        Text@45..48 "SRC"
        Newline@48..49 "\n"
  DrawerEnd@49..55
    Text@49..54 ":end:"
    Newline@54..55 "\n"
"###
        );
    }

    #[test]
    fn test_drawer_04() {
        assert_eq!(
            get_parser_output(
                drawer_parser(element::element_in_drawer_parser::<()>()),
                r##"#+caption: affiliated keywords in drawer
:a:
foo
:end:
"##
            ),
            r###"Drawer@0..55
  AffiliatedKeyword@0..41
    HashPlus@0..2 "#+"
    KeywordKey@2..9
      Text@2..9 "caption"
    Colon@9..10 ":"
    Whitespace@10..11 " "
    KeywordValue@11..40
      Text@11..40 "affiliated keywords i ..."
    Newline@40..41 "\n"
  DrawerBegin@41..45
    Colon@41..42 ":"
    Text@42..43 "a"
    Colon@43..44 ":"
    Newline@44..45 "\n"
  DrawerContent@45..49
    Paragraph@45..49
      Text@45..49 "foo\n"
  DrawerEnd@49..55
    Text@49..54 ":end:"
    Newline@54..55 "\n"
"###
        );
    }

    #[test]
    fn test_drawer_05() {
        assert_eq!(
            get_parser_output(
                drawer_parser(element::element_in_drawer_parser::<()>()),
                r##":properties:
:add: asd

:dxx: asd
:end:
"##
            ),
            r###"Drawer@0..40
  DrawerBegin@0..13
    Colon@0..1 ":"
    Text@1..11 "properties"
    Colon@11..12 ":"
    Newline@12..13 "\n"
  DrawerContent@13..34
    Paragraph@13..24
      Text@13..23 ":add: asd\n"
      BlankLine@23..24 "\n"
    Paragraph@24..34
      Text@24..34 ":dxx: asd\n"
  DrawerEnd@34..40
    Text@34..39 ":end:"
    Newline@39..40 "\n"
"###
        );
    }

    #[test]
    fn test_node_property_01() {
        assert_eq!(
            get_parser_output(
                node_property_parser::<()>(),
                r":header-args:R:          :session *R*
"
            ),
            r###"NodeProperty@0..38
  Colon@0..1 ":"
  Text@1..14 "header-args:R"
  Colon@14..15 ":"
  Whitespace@15..25 "          "
  Text@25..37 ":session *R*"
  Newline@37..38 "\n"
"###
        );
    }

    #[test]
    fn test_node_property_02() {
        assert_eq!(
            get_parser_output(
                node_property_parser::<()>(),
                r"    :header-args:R:          :session *R*
"
            ),
            r###"NodeProperty@0..42
  Whitespace@0..4 "    "
  Colon@4..5 ":"
  Text@5..18 "header-args:R"
  Colon@18..19 ":"
  Whitespace@19..29 "          "
  Text@29..41 ":session *R*"
  Newline@41..42 "\n"
"###
        );
    }

    #[test]
    fn test_node_property_03() {
        assert_eq!(
            get_parser_output(
                node_property_parser::<()>(),
                r"    :header-args+:R+:          :session *R*
"
            ),
            r###"NodeProperty@0..44
  Whitespace@0..4 "    "
  Colon@4..5 ":"
  Text@5..19 "header-args+:R"
  Plus@19..20 "+"
  Colon@20..21 ":"
  Whitespace@21..31 "          "
  Text@31..43 ":session *R*"
  Newline@43..44 "\n"
"###
        );
    }

    #[test]
    fn test_node_property_04() {
        assert_eq!(
            get_parser_output(
                node_property_parser::<()>(),
                r"    :header-args:R: 
"
            ),
            r###"NodeProperty@0..21
  Whitespace@0..4 "    "
  Colon@4..5 ":"
  Text@5..18 "header-args:R"
  Colon@18..19 ":"
  Whitespace@19..20 " "
  Newline@20..21 "\n"
"###
        );
    }

    #[test]
    fn test_node_property_05() {
        assert_eq!(
            get_parser_output(
                node_property_parser::<()>(),
                r":name:
"
            ),
            r###"NodeProperty@0..7
  Colon@0..1 ":"
  Text@1..5 "name"
  Colon@5..6 ":"
  Newline@6..7 "\n"
"###
        );
    }

    #[test]
    fn test_property_drawer_01() {
        assert_eq!(
            get_parser_output(
                property_drawer_parser::<()>(),
                r"         :PROPERTIES:
         :Title:     Goldberg Variations
         :Composer:  J.S. Bach
         :Artist:    Glenn Gould
         :Publisher: Deutsche Grammophon


         :NDisks:    1

         :END:

"
            ),
            r###"PropertyDrawer@0..210
  Whitespace@0..9 "         "
  Text@9..21 ":PROPERTIES:"
  Newline@21..22 "\n"
  NodeProperty@22..63
    Whitespace@22..31 "         "
    Colon@31..32 ":"
    Text@32..37 "Title"
    Colon@37..38 ":"
    Whitespace@38..43 "     "
    Text@43..62 "Goldberg Variations"
    Newline@62..63 "\n"
  NodeProperty@63..94
    Whitespace@63..72 "         "
    Colon@72..73 ":"
    Text@73..81 "Composer"
    Colon@81..82 ":"
    Whitespace@82..84 "  "
    Text@84..93 "J.S. Bach"
    Newline@93..94 "\n"
  NodeProperty@94..127
    Whitespace@94..103 "         "
    Colon@103..104 ":"
    Text@104..110 "Artist"
    Colon@110..111 ":"
    Whitespace@111..115 "    "
    Text@115..126 "Glenn Gould"
    Newline@126..127 "\n"
  NodeProperty@127..170
    Whitespace@127..136 "         "
    Colon@136..137 ":"
    Text@137..146 "Publisher"
    Colon@146..147 ":"
    Whitespace@147..148 " "
    Text@148..167 "Deutsche Grammophon"
    Newline@167..168 "\n"
    BlankLine@168..169 "\n"
    BlankLine@169..170 "\n"
  NodeProperty@170..194
    Whitespace@170..179 "         "
    Colon@179..180 ":"
    Text@180..186 "NDisks"
    Colon@186..187 ":"
    Whitespace@187..191 "    "
    Text@191..192 "1"
    Newline@192..193 "\n"
    BlankLine@193..194 "\n"
  Whitespace@194..203 "         "
  Text@203..208 ":END:"
  Newline@208..209 "\n"
  BlankLine@209..210 "\n"
"###
        );
    }
}
