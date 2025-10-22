//! Footnote definition parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use chumsky::prelude::*;
use chumsky::{inspector::SimpleState, text::Char};
use rowan::{GreenNode, GreenToken, NodeOrToken};

// fixme: 简单版本，假设只一行
pub(crate) fn footnote_definition_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    just("[fn:")
        .then(
            // label
            any()
                .filter(|c: &char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .then(just("]"))
        .then(object::whitespaces_g1())
        // .then(element::element_parser().repeated().collect::<Vec<_>>())
        .then(
            any()
                .filter(|c: &char| !c.is_newline())
                .repeated()
                .collect::<String>(),
        )
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(
            // |((((_lfnc, label), rbracket), ws1), contents)| {
            |(((((((_lfnc, label), rbracket), ws1), content), ws2), nl), blanklines)| {
                let mut children = vec![];

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::LeftSquareBracket.into(),
                    "[",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    "fn",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    ":",
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &label,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightSquareBracket.into(),
                    rbracket,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));

                // for content in contents {
                //     children.push(content);
                // }

                if content.len() > 0 {
                    let content_token =
                        NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Text.into(), &content));

                    let content_node = NodeOrToken::Node(GreenNode::new(
                        OrgSyntaxKind::Paragraph.into(),
                        vec![content_token],
                    ));
                    children.push(content_node);
                }

                if ws2.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws2,
                    )));
                }

                match nl {
                    Some(newline) => {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Newline.into(),
                            &newline,
                        )));
                    }
                    None => {}
                }

                for blankline in blanklines {
                    children.push(NodeOrToken::Token(blankline));
                    // children.push(blankline);
                }

                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::FootnoteDefinition.into(),
                    children,
                ))
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::{ParserState, SyntaxNode};

    #[test]
    fn test_footnote_basic() {
        let input = "[fn:1] A short footnote.";
        let mut state = SimpleState(ParserState::default());
        let r = footnote_definition_parser().parse_with_state(input, &mut state);
        assert_eq!(r.has_output(), true);
        let syntax_tree = SyntaxNode::new_root(r.into_result().unwrap().into_node().expect("xxx"));
        let ans = r##"FootnoteDefinition@0..24
  LeftSquareBracket@0..1 "["
  Text@1..3 "fn"
  Colon@3..4 ":"
  Text@4..5 "1"
  RightSquareBracket@5..6 "]"
  Whitespace@6..7 " "
  Paragraph@7..24
    Text@7..24 "A short footnote."
"##;
        println!("{:#?}", syntax_tree);
        assert_eq!(format!("{:#?}", syntax_tree), ans);
    }

    //     #[test]
    //     fn test_footnote_blankline() {

    //         let input = "[fn:2] This is a longer footnote.

    // It even contains a single blank line.
    // ";
    //         let mut state = SimpleState(ParserState::default());
    //         let r = footnote_definition_parser().parse_with_state(input, &mut state);
    //         assert!(r.has_output());
    //         let syntax_tree = SyntaxNode::new_root(r.into_result().unwrap().into_node().expect("xxx"));
    //         println!("{:#?}", syntax_tree);
    //     }

    // assert_eq!(r.has_output(), true);
}
