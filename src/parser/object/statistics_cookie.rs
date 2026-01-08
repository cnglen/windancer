//! statistics cookie
use crate::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;

// PEG: statistics_cookie <- "[" ((PERCENT? "%") / (NUM1? "/" NUM2?)) "]"
pub(crate) fn statistics_cookie_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let num = text::int(10);

    group((
        just("["),
        choice((
            group((num.clone().or_not(), just("%"))).to_slice(),
            group((num.clone().or_not(), just("/"), num.clone().or_not())).to_slice(),
        )),
        just("]"),
    ))
    .map(|(left_bracket, value, right_bracket)| {
        crate::node!(
            OSK::StatisticsCookie,
            vec![
                crate::token!(OSK::LeftSquareBracket, left_bracket),
                crate::token!(OSK::Text, value),
                crate::token!(OSK::RightSquareBracket, right_bracket),
            ]
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_statistics_cookies_01() {
        let parser = statistics_cookie_parser::<()>();
        let input = r##"[3/4]"##;
        let expected_output = r##"StatisticsCookie@0..5
  LeftSquareBracket@0..1 "["
  Text@1..4 "3/4"
  RightSquareBracket@4..5 "]"
"##;
        assert_eq!(get_parser_output(parser, input), expected_output,);
    }

    #[test]
    fn test_statistics_cookies_02() {
        let parser = statistics_cookie_parser::<()>();
        let input = r##"[34%]"##;
        let expected_output = r##"StatisticsCookie@0..5
  LeftSquareBracket@0..1 "["
  Text@1..4 "34%"
  RightSquareBracket@4..5 "]"
"##;
        assert_eq!(get_parser_output(parser, input), expected_output,);
    }

    #[test]
    fn test_statistics_cookies_03() {
        let parser = statistics_cookie_parser::<()>();
        let input = r##"[%]"##;
        let expected_output = r##"StatisticsCookie@0..3
  LeftSquareBracket@0..1 "["
  Text@1..2 "%"
  RightSquareBracket@2..3 "]"
"##;
        assert_eq!(get_parser_output(parser, input), expected_output,);
    }

    #[test]
    fn test_statistics_cookies_04() {
        let parser = statistics_cookie_parser::<()>();
        let input = r##"[/]"##;
        let expected_output = r##"StatisticsCookie@0..3
  LeftSquareBracket@0..1 "["
  Text@1..2 "/"
  RightSquareBracket@2..3 "]"
"##;
        assert_eq!(get_parser_output(parser, input), expected_output,);
    }
}
