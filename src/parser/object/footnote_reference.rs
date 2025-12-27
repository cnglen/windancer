//! footnote reference parser
use crate::parser::ParserState;
use crate::parser::{MyExtra, NT, OSK};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken};

// - [fn:LABEL]
// - [fn:LABEL:DEFINITION]
// - [fn::DEFINITION]
// PEG: footnote <- "[fn:" ((LABEL (":" DEFINITION)?) / (":" DEFINITION)) "]"
pub(crate) fn footnote_reference_parser_inner<'a, C: 'a>(
    definition_parser: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let label = any()
        .filter(|c: &char| c.is_alphanumeric() || matches!(c, '_' | '-'))
        .repeated()
        .at_least(1)
        .to_slice();

    just("[")
        .then(just("fn"))
        .then(just(":"))
        .then(choice((
            // [fn:LABEL]
            // [fn:LABEL:DEFINITION]
            label
                .then(just(":").then(definition_parser.clone()).or_not())
                .then(just("]"))
                .map(|((label, maybe_colon_defintion), right_bracket)| {
                    let mut children = Vec::with_capacity(4);
                    children.push(NT::Token(GreenToken::new(
                        OSK::FootnoteReferenceLabel.into(),
                        label,
                    )));

                    if let Some((colon, definition)) = maybe_colon_defintion {
                        children.push(NT::Token(GreenToken::new(OSK::Colon.into(), colon)));
                        children.push(NT::Node(GreenNode::new(
                            OSK::FootnoteReferenceDefintion.into(),
                            definition,
                        )));
                    }
                    children.push(NT::Token(GreenToken::new(
                        OSK::RightSquareBracket.into(),
                        right_bracket,
                    )));

                    children
                }),
            // [fn::DEFINITION]
            just(":").then(definition_parser).then(just("]")).map(
                |((colon, definition), right_bracket)| {
                    vec![
                        NT::Token(GreenToken::new(OSK::Colon.into(), colon)),
                        NT::Node(GreenNode::new(
                            OSK::FootnoteReferenceDefintion.into(),
                            definition,
                        )),
                        NT::Token(GreenToken::new(
                            OSK::RightSquareBracket.into(),
                            right_bracket,
                        )),
                    ]
                },
            ),
        )))
        .map_with(|(((lbracket, fn_text), colon), others), e| {
            e.state().prev_char = Some(']');

            let mut children = Vec::with_capacity(3 + others.len());
            children.extend(vec![
                NT::Token(GreenToken::new(OSK::LeftSquareBracket.into(), lbracket)),
                NT::Token(GreenToken::new(OSK::Text.into(), fn_text)),
                NT::Token(GreenToken::new(OSK::Colon.into(), colon)),
            ]);

            children.extend(others);

            NT::Node(GreenNode::new(OSK::FootnoteReference.into(), children))
        })
        .boxed()
}

pub(crate) fn footnote_reference_parser<'a, C: 'a>(
    object_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
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
    let standard_objects_parser = object_parser.repeated().at_least(1).collect::<Vec<NT>>();
    let definition_parser =
        standard_objects_parser.nested_in(single_expression.clone().repeated().to_slice());

    footnote_reference_parser_inner(definition_parser)
}

pub(crate) fn simple_footnote_reference_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
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
    let definition_parser = single_expression
        .clone()
        .repeated()
        .to_slice()
        .map(|s: &str| vec![NT::Token(GreenToken::new(OSK::Text.into(), s))]);

    footnote_reference_parser_inner(definition_parser)
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
    Colon@17..18 ":"
    Colon@18..19 ":"
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
