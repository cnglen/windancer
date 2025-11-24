//! Drawer parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, element, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;

use crate::parser::object::just_case_insensitive;

fn name_parser<'a>()
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

pub(crate) fn node_property_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let name = name_parser();
    let value = none_of("\r\n").repeated().collect::<String>();
    let blank_lines = object::blank_line_parser().repeated().collect::<Vec<_>>();
    object::whitespaces()
        .then(name)
        .then(just("+").or_not())
        .then(just(":"))
        .then(object::whitespaces())
        .then(value)
        .then(just("\n"))
        .then(blank_lines)
        .map_with(
            |(((((((ws0, name), maybe_plus), colon), ws1), value), newline), blank_lines), e| {
                let mut children = vec![];

                if ws0.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws0,
                    )));
                }

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

                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws1,
                    )));
                }

                if value.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &value,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    &newline,
                )));

                for bl in blank_lines {
                    children.push(NodeOrToken::Token(bl));
                }

                NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::NodeProperty.into(),
                    children,
                ))
            },
        )
}

pub(crate) fn property_drawer_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let begin_row = object::whitespaces()
        .then(just_case_insensitive(":properties:"))
        .then(object::whitespaces())
        .then(just("\n"));

    let end_row = object::whitespaces()
        .then(just_case_insensitive(":end:"))
        .then(object::whitespaces())
        .then(just("\n"));

    let blank_lines = object::blank_line_parser().repeated().collect::<Vec<_>>();

    begin_row
        .then(
            node_property_parser()
                .and_is(end_row.clone().not())
                .repeated()
                .collect::<Vec<_>>(),
        )
        .then(end_row)
        .then(blank_lines)
        .map_with(
            |(
                (((((ws1, properties), ws2), nl1), contents), (((ws3, end), ws4), nl2)),
                blank_lines,
            ),
             e| {
                let mut children = vec![];

                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws1,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &properties,
                )));

                if ws2.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws2,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    nl1,
                )));

                for e in contents {
                    children.push(e);
                }

                if ws3.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws3,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &end,
                )));

                if ws4.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws4,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    nl2,
                )));

                for bl in blank_lines {
                    children.push(NodeOrToken::Token(bl));
                }

                NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::PropertyDrawer.into(),
                    children,
                ))
            },
        )
}

pub(crate) fn drawer_parser<'a>(
    element_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
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
        .then(just("\n"))
        .map(|(((((ws1, c1), name), c2), ws2), nl)| {
            // println!(
            //     "drawer begin row: ws1={}, c1={}, name={}, c2={}, ws2={}, nl={}",
            //     ws1, c1, name, c2, ws2, nl
            // );
            let mut tokens = vec![];

            if ws1.len() > 0 {
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
            if ws2.len() > 0 {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
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
            let mut tokens = vec![];

            if ws1.len() > 0 {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }
            tokens.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &end,
            )));
            if ws2.len() > 0 {
                tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }
            match nl {
                Some(_nl) => tokens.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    &_nl,
                ))),
                None => {}
            }
            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::DrawerEnd.into(),
                tokens,
            ))
        });

    let drawer_content_inner = object::line_parser()
        .and_is(drawer_end_row.clone().not())
        .and_is(just("*").not())
        .repeated()
        .to_slice();

    let drawer_content = element_parser
        .repeated()
        .collect::<Vec<_>>()
        .nested_in(drawer_content_inner)
        .map(|s| {
            let mut children = vec![];
            for c in s {
                children.push(c);
            }
            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::DrawerContent.into(),
                children,
            ))
        });
    let blank_lines = object::blank_line_parser().repeated().collect::<Vec<_>>();

    affiliated_keywords
        .then(drawer_name_row)
        .then(drawer_content)
        .then(drawer_end_row)
        .then(blank_lines)
        .map(|((((maybe_keywords, begin), content), end), blank_lines)| {
            let mut children = vec![];

            for keyword in maybe_keywords {
                children.push(keyword);
            }
            children.push(begin);
            children.push(content);
            children.push(end);
            for bl in blank_lines {
                children.push(NodeOrToken::Token(bl));
            }

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::Drawer.into(),
                children,
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use crate::parser::element;
    use crate::parser::element::element_parser;
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_drawer_01() {
        assert_eq!(
            get_parser_output(
                drawer_parser(element::element_in_drawer_parser()),
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
            drawer_parser(element::element_in_drawer_parser()),
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
                drawer_parser(element::element_in_drawer_parser()),
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
                drawer_parser(element::element_in_drawer_parser()),
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
    fn test_node_property_01() {
        assert_eq!(
            get_parser_output(
                node_property_parser(),
                r":header-args:R:          :session *R*
"
            ),
            r###"NodeProperty@0..38
  Text@0..14 ":header-args:R"
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
                node_property_parser(),
                r"    :header-args:R:          :session *R*
"
            ),
            r###"NodeProperty@0..42
  Whitespace@0..4 "    "
  Text@4..18 ":header-args:R"
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
                node_property_parser(),
                r"    :header-args+:R+:          :session *R*
"
            ),
            r###"NodeProperty@0..44
  Whitespace@0..4 "    "
  Text@4..19 ":header-args+:R"
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
                node_property_parser(),
                r"    :header-args:R: 
"
            ),
            r###"NodeProperty@0..21
  Whitespace@0..4 "    "
  Text@4..18 ":header-args:R"
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
                node_property_parser(),
                r":name:
"
            ),
            r###"NodeProperty@0..7
  Text@0..5 ":name"
  Colon@5..6 ":"
  Newline@6..7 "\n"
"###
        );
    }

    #[test]
    fn test_property_drawer_01() {
        assert_eq!(
            get_parser_output(
                property_drawer_parser(),
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
    Text@31..37 ":Title"
    Colon@37..38 ":"
    Whitespace@38..43 "     "
    Text@43..62 "Goldberg Variations"
    Newline@62..63 "\n"
  NodeProperty@63..94
    Whitespace@63..72 "         "
    Text@72..81 ":Composer"
    Colon@81..82 ":"
    Whitespace@82..84 "  "
    Text@84..93 "J.S. Bach"
    Newline@93..94 "\n"
  NodeProperty@94..127
    Whitespace@94..103 "         "
    Text@103..110 ":Artist"
    Colon@110..111 ":"
    Whitespace@111..115 "    "
    Text@115..126 "Glenn Gould"
    Newline@126..127 "\n"
  NodeProperty@127..170
    Whitespace@127..136 "         "
    Text@136..146 ":Publisher"
    Colon@146..147 ":"
    Whitespace@147..148 " "
    Text@148..167 "Deutsche Grammophon"
    Newline@167..168 "\n"
    BlankLine@168..169 "\n"
    BlankLine@169..170 "\n"
  NodeProperty@170..194
    Whitespace@170..179 "         "
    Text@179..186 ":NDisks"
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
