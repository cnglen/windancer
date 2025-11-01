//! footnote reference parser
use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// Footnote refrence
// - [fn:LABEL]
// - [fn:LABEL:DEFINITION]
// - [fn::DEFINITION]
pub(crate) fn footnote_reference_parser<'a>(
    object_parser: impl Parser<
        'a,
        &'a str,
        S2,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let label = any()
        .filter(|c: &char| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
        .repeated()
        .at_least(1)
        .collect::<String>();

    // defintion must in oneline
    let var =
        none_of::<&str, &str, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>("[]\r\n")
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
    let standard_objects_parser = object_parser.repeated().at_least(1).collect::<Vec<S2>>();
    let definition =
        standard_objects_parser.nested_in(single_expression.clone().repeated().to_slice());

    // [fn:LABEL]
    let t1 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>("[fn:")
        .then(label)
        .then(just("]"))
        .map_with(|((_left_fn_c, label), rbracket), e| {
            e.state().prev_char = rbracket.chars().last();
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
                OrgSyntaxKind::FootnoteReferenceLabel.into(),
                &label,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteReference.into(),
                children,
            )))
        });

    // [fn:LABEL:DEFINITION]
    let t2 = just("[fn:")
        .then(label)
        .then(just(":"))
        .then(definition.clone())
        // .map(|s| {
        //     println!("s2={s:?};");
        //     s
        // })
        .then(just("]"))
        .map_with(
            |((((_left_fn_c, label), colon), definition), rbracket), e| {
                e.state().prev_char = rbracket.chars().last();
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
                    colon,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::FootnoteReferenceLabel.into(),
                    &label,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Colon.into(),
                    ":",
                )));

                let mut defintion_children = vec![];
                for node in definition {
                    match node {
                        S2::Single(e) => {
                            defintion_children.push(e);
                        }
                        S2::Double(e1, e2) => {
                            defintion_children.push(e1);
                            defintion_children.push(e2);
                        }
                        _ => {}
                    }
                }
                children.push(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::FootnoteReferenceDefintion.into(),
                    defintion_children,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::RightSquareBracket.into(),
                    rbracket,
                )));

                S2::Single(NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::FootnoteReference.into(),
                    children,
                )))
            },
        );

    // [fn::DEFINITION]
    let t3 = just("[fn::").then(definition).then(just("]")).map_with(
        |((_left_fn_c_c, definition), rbracket), e| {
            e.state().prev_char = rbracket.chars().last();
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
                OrgSyntaxKind::Colon2.into(),
                "::",
            )));

            let mut defintion_children = vec![];
            for node in definition {
                match node {
                    S2::Single(e) => {
                        defintion_children.push(e);
                    }
                    S2::Double(e1, e2) => {
                        defintion_children.push(e1);
                        defintion_children.push(e2);
                    }
                    _ => {}
                }
            }
            children.push(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteReferenceDefintion.into(),
                defintion_children,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            S2::Single(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::FootnoteReference.into(),
                children,
            )))
        },
    );

    t1.or(t2).or(t3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::{get_parser_output, get_parsers_output};
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_01_fn_label() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "this is a org [fn:1]."),
            r##"Root@0..21
  Text@0..14 "this is a org "
  FootnoteReference@14..20
    LeftSquareBracket@14..15 "["
    Text@15..17 "fn"
    Colon@17..18 ":"
    FootnoteReferenceLabel@18..19 "1"
    RightSquareBracket@19..20 "]"
  Text@20..21 "."
"##
        );
    }

    #[test]
    fn test_02_fn_label_defintion() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "this is a org [fn:1:*bold*]."),
            r##"Root@0..28
  Text@0..14 "this is a org "
  FootnoteReference@14..27
    LeftSquareBracket@14..15 "["
    Text@15..17 "fn"
    Colon@17..18 ":"
    FootnoteReferenceLabel@18..19 "1"
    Colon@19..20 ":"
    FootnoteReferenceDefintion@20..26
      Bold@20..26
        Asterisk@20..21 "*"
        Text@21..25 "bold"
        Asterisk@25..26 "*"
    RightSquareBracket@26..27 "]"
  Text@27..28 "."
"##
        );
    }

    #[test]
    fn test_03_fn_defintion() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser(),
                "this is a org [fn::*org* is a good format]."
            ),
            r##"Root@0..43
  Text@0..14 "this is a org "
  FootnoteReference@14..42
    LeftSquareBracket@14..15 "["
    Text@15..17 "fn"
    Colon2@17..19 "::"
    FootnoteReferenceDefintion@19..41
      Bold@19..24
        Asterisk@19..20 "*"
        Text@20..23 "org"
        Asterisk@23..24 "*"
      Text@24..41 " is a good format"
    RightSquareBracket@41..42 "]"
  Text@42..43 "."
"##
        );
    }
}
