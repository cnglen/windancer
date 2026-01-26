//! radio target parser
use crate::compiler::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;

/// radio target parser: <<<TARGET>>>
fn radio_target_parser_inner<'a, C: 'a>(
    target_parser: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    just("<<<")
        .then(target_parser)
        .then(just(">>>"))
        .map(|((lbracket3, target), rbracket3)| {
            let mut children = Vec::with_capacity(2 + target.len());
            children.push(crate::token!(OSK::LeftAngleBracket3, lbracket3));
            children.extend(target);
            children.push(crate::token!(OSK::RightAngleBracket3, rbracket3));

            crate::node!(OSK::RadioTarget, children)
        })
        .boxed()
}

pub(crate) fn radio_target_parser<'a, C: 'a>(
    object_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let minimal_objects_parser = object_parser
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<NT>>();

    let target_inner = none_of("<>\n \t") // starting with a non-whitespce character
        .then(
            none_of("<>\n")
                .repeated()
                .at_least(1)
                .to_slice()
                .try_map_with(|s: &str, e| {
                    if s.chars().last().expect("at least 1").is_whitespace() {
                        Err(Rich::custom(
                            e.span(),
                            format!("the last char of '{}' can't be whitespace", s),
                        ))
                    } else {
                        Ok(s)
                    }
                })
                .or_not(),
        )
        .to_slice();
    let target_parser = minimal_objects_parser.nested_in(target_inner);

    radio_target_parser_inner(target_parser)
}

pub(crate) fn simple_radio_target_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let target_inner = none_of("<>\n \t") // starting with a non-whitespce character
        .then(
            none_of("<>\n")
                .repeated()
                .at_least(1)
                .to_slice()
                .try_map_with(|s: &str, e| {
                    if s.chars().last().expect("at least 1").is_whitespace() {
                        Err(Rich::custom(
                            e.span(),
                            format!("the last char of '{}' can't be whitespace", s),
                        ))
                    } else {
                        Ok(s)
                    }
                })
                .or_not(),
        )
        .to_slice()
        .map(|s: &str| vec![crate::token!(OSK::Text, s)]);
    radio_target_parser_inner(target_inner)
}

#[cfg(test)]
mod tests {
    use crate::compiler::parser::config::OrgParserConfig;
    use crate::compiler::parser::{common::get_parsers_output, object};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_radio_target_01() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(OrgParserConfig::default()),
                "<<<target>>>"
            ),
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
            get_parsers_output(
                object::objects_parser::<()>(OrgParserConfig::default()),
                "<<<tar\nget>>>"
            ),
            r##"Root@0..13
  Text@0..13 "<<<tar\nget>>>"
"##
        );
    }

    #[test]
    fn test_radio_target_03() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(OrgParserConfig::default()),
                "<<< target>>>"
            ),
            r##"Root@0..13
  Text@0..13 "<<< target>>>"
"##,
            r"TARGET It cannot start or end with a whitespace character."
        );
    }

    #[test]
    fn test_radio_target_04() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(OrgParserConfig::default()),
                "<<<target >>>"
            ),
            r##"Root@0..13
  Text@0..13 "<<<target >>>"
"##,
            r"TARGET It cannot start or end with a whitespace character."
        );
    }

    #[test]
    fn test_radio_target_05() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(OrgParserConfig::default()),
                "<<<t>>>"
            ),
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
            get_parsers_output(
                object::objects_parser::<()>(OrgParserConfig::default()),
                r"<<<\alpha $a+b$ foo>>>"
            ),
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
