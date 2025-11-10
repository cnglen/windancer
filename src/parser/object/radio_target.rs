//! radio target parser
use crate::parser::ParserState;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// radio target parser: <<<TARGET>>>
pub(crate) fn radio_target_parser<'a>(
    object_parser: impl Parser<
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
    let minimal_objects_parser = object_parser
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<NodeOrToken<GreenNode, GreenToken>>>();

    let target_onechar = none_of("<>\n \t").map(|c| format!("{c}")).to_slice();
    let target_g2char = none_of("<>\n \t")
        .then(none_of("<>\n").repeated().at_least(1).collect::<String>())
        .try_map_with(|(a, b), e| {
            if b.chars().last().expect("at least 1").is_whitespace() {
                Err(Rich::custom(
                    e.span(),
                    format!("the last char of '{}' can't be whitespace", b),
                ))
            } else {
                Ok(format!("{a}{b}"))
            }
        })
        .to_slice();
    let target = minimal_objects_parser.nested_in(choice((target_g2char, target_onechar)));

    just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, ()>>("<<<")
        .then(target)
        .then(just(">>>"))
        .map_with(|((lbracket3, target), rbracket3), e| {
            e.state().prev_char = rbracket3.chars().last();

            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftAngleBracket3.into(),
                lbracket3,
            )));

            for node in target {
                children.push(node);
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightAngleBracket3.into(),
                rbracket3,
            )));

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::RadioTarget.into(),
                children,
            ))
        })
}

#[cfg(test)]
mod tests {
    use crate::parser::{common::get_parsers_output, object};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_radio_target_01() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "<<<target>>>"),
            r##"Root@0..12
  RadioTarget@0..12
    LeftAngleBracket3@0..3 "<<<"
    Text@3..9 "target"
    RightAngleBracket3@9..12 ">>>"
"##
        );
    }

    #[test]
    fn test_radio_target_02() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "<<<tar\nget>>>"),
            r##"Root@0..13
  Text@0..13 "<<<tar\nget>>>"
"##
        );
    }

    #[test]
    fn test_radio_target_03() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "<<< target>>>"),
            r##"Root@0..13
  Text@0..13 "<<< target>>>"
"##,
            r"TARGET It cannot start or end with a whitespace character."
        );
    }

    #[test]
    fn test_radio_target_04() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "<<<target >>>"),
            r##"Root@0..13
  Text@0..13 "<<<target >>>"
"##,
            r"TARGET It cannot start or end with a whitespace character."
        );
    }

    #[test]
    fn test_radio_target_05() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), "<<<t>>>"),
            r##"Root@0..7
  RadioTarget@0..7
    LeftAngleBracket3@0..3 "<<<"
    Text@3..4 "t"
    RightAngleBracket3@4..7 ">>>"
"##,
            r"TARGET with one char"
        );
    }

    #[test]
    fn test_radio_target_06() {
        assert_eq!(
            get_parsers_output(object::objects_parser(), r"<<<\alpha $a+b$ foo>>>"),
            r##"Root@0..22
  RadioTarget@0..22
    LeftAngleBracket3@0..3 "<<<"
    Entity@3..9
      BackSlash@3..4 "\\"
      EntityName@4..9 "alpha"
    Text@9..10 " "
    LatexFragment@10..15
      Dollar@10..11 "$"
      Text@11..14 "a+b"
      Dollar@14..15 "$"
    Text@15..19 " foo"
    RightAngleBracket3@19..22 ">>>"
"##,
            r"TARGET with minimal objects"
        );
    }
}
