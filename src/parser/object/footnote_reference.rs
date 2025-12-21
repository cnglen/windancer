//! footnote reference parser
use crate::parser::ParserState;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// Footnote refrence
/// Helper function to build common footnote reference token structure
fn build_footnote_reference(
    lbracket: &str,
    fn_text: &str,
    rbracket: &str,
    middle_tokens: Vec<NodeOrToken<GreenNode, GreenToken>>,
) -> NodeOrToken<GreenNode, GreenToken> {
    let mut tokens = Vec::with_capacity(3 + middle_tokens.len());
    tokens.extend(vec![
        NodeOrToken::Token(GreenToken::new(
            OrgSyntaxKind::LeftSquareBracket.into(),
            lbracket,
        )),
        NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Text.into(), fn_text)),
    ]);
    tokens.extend(middle_tokens);
    tokens.push(NodeOrToken::Token(GreenToken::new(
        OrgSyntaxKind::RightSquareBracket.into(),
        rbracket,
    )));

    NodeOrToken::Node(GreenNode::new(
        OrgSyntaxKind::FootnoteReference.into(),
        tokens,
    ))
}

// - [fn:LABEL]
// - [fn:LABEL:DEFINITION]
// - [fn::DEFINITION]
pub(crate) fn footnote_reference_parser<'a, C: 'a>(
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
    let label = any()
        .filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '-'))
        .repeated()
        .at_least(1)
        .to_slice();

    // defintion must in oneline
    let var =
        none_of::<&str, &str, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>("[]\r\n")
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
    let standard_objects_parser = object_parser
        .repeated()
        .at_least(1)
        .collect::<Vec<NodeOrToken<GreenNode, GreenToken>>>();
    let definition =
        standard_objects_parser.nested_in(single_expression.clone().repeated().to_slice());

    // [fn:LABEL]
    // [fn:LABEL:DEFINITION]
    let t1_or_t2 = just("[")
        .then(just("fn"))
        .then(just(":"))
        .then(label)
        .then(just(":").then(definition.clone()).or_not())
        .then(just("]"))
        .map_with(
            |(((((lbracket, fn_text), colon1), label), maybe_colon2_definition), rbracket), e| {
                e.state().prev_char = Some(']');

                match maybe_colon2_definition {
                    Some((colon2, definition)) => build_footnote_reference(
                        lbracket,
                        fn_text,
                        rbracket,
                        vec![
                            NodeOrToken::Token(GreenToken::new(
                                OrgSyntaxKind::Colon.into(),
                                colon1,
                            )),
                            NodeOrToken::Token(GreenToken::new(
                                OrgSyntaxKind::FootnoteReferenceLabel.into(),
                                label,
                            )),
                            NodeOrToken::Token(GreenToken::new(
                                OrgSyntaxKind::Colon.into(),
                                colon2,
                            )),
                            NodeOrToken::Node(GreenNode::new(
                                OrgSyntaxKind::FootnoteReferenceDefintion.into(),
                                definition,
                            )),
                        ],
                    ),
                    None => build_footnote_reference(
                        lbracket,
                        fn_text,
                        rbracket,
                        vec![
                            NodeOrToken::Token(GreenToken::new(
                                OrgSyntaxKind::Colon.into(),
                                colon1,
                            )),
                            NodeOrToken::Token(GreenToken::new(
                                OrgSyntaxKind::FootnoteReferenceLabel.into(),
                                label,
                            )),
                        ],
                    ),
                }
            },
        );

    // [fn::DEFINITION]
    let t3 = just("[")
        .then(just("fn"))
        .then(just("::"))
        .then(definition)
        .then(just("]"))
        .map_with(
            |((((lbracket, fn_text), colon_colon), definition), rbracket), e| {
                e.state().prev_char = rbracket.chars().last();

                build_footnote_reference(
                    lbracket,
                    fn_text,
                    rbracket,
                    vec![
                        NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Colon2.into(),
                            colon_colon,
                        )),
                        NodeOrToken::Node(GreenNode::new(
                            OrgSyntaxKind::FootnoteReferenceDefintion.into(),
                            definition,
                        )),
                    ],
                )
            },
        );

    Parser::boxed(choice((t1_or_t2, t3)))
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parsers_output;
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_01_fn_label() {
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), "this is a org [fn:1]."),
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
            get_parsers_output(
                object::objects_parser::<()>(),
                "this is a org [fn:1:*bold*]."
            ),
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
                object::objects_parser::<()>(),
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
