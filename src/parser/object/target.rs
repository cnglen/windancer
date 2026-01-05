//! target parser
use crate::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;

/// target parser: <<TARGET>>
pub(crate) fn target_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let target = none_of("<>\n \t")
        .then(
            none_of("<>\n")
                .repeated()
                .at_least(1)
                .to_slice()
                .try_map_with(|ab: &str, e| {
                    if ab.chars().last().expect("at least 1").is_whitespace() {
                        Err(Rich::custom(
                            e.span(),
                            format!("the last char of '{}' can't be whitespace", ab),
                        ))
                    } else {
                        Ok(ab)
                    }
                })
                .or_not(),
        )
        .to_slice();

    just("<<")
        .then(target)
        .then(just(">>"))
        .map(|((lbracket2, target), rbracket2)| {
            let children = vec![
                crate::token!(OSK::LeftAngleBracket2, lbracket2),
                crate::token!(OSK::Text, target),
                crate::token!(OSK::RightAngleBracket2, rbracket2),
            ];

            crate::node!(OSK::Target, children)
        })
        .boxed()
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parser_output;
    use pretty_assertions::assert_eq;

    use super::target_parser;

    #[test]
    fn test_target_01() {
        assert_eq!(
            get_parser_output(target_parser::<()>(), "<<target>>"),
            r##"Target@0..10
  LeftAngleBracket2@0..2 "<<"
  Text@2..8 "target"
  RightAngleBracket2@8..10 ">>"
"##
        );
    }

    #[test]
    #[should_panic]
    fn test_target_02() {
        get_parser_output(target_parser::<()>(), "<<tar\nget>>");
    }

    #[test]
    #[should_panic]
    fn test_target_03() {
        get_parser_output(target_parser::<()>(), "<< target>>");
    }

    #[test]
    #[should_panic]
    fn test_target_04() {
        get_parser_output(target_parser::<()>(), "<<target >>");
    }

    #[test]
    fn test_target_05() {
        assert_eq!(
            get_parser_output(target_parser::<()>(), "<<t>>"),
            r##"Target@0..5
  LeftAngleBracket2@0..2 "<<"
  Text@2..3 "t"
  RightAngleBracket2@3..5 ">>"
"##,
            r"TARGET with one char"
        );
    }
}
