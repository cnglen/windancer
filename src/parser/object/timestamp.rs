//! timestamp parser
use crate::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;

use super::whitespaces_g1;

/// timestamp parser: <<TIMESTAMP>>
pub(crate) fn timestamp_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone
{
    let yyyymmdd = one_of("0123456789")
        .repeated()
        .at_least(4)
        .at_most(4)
        .then(just("-"))
        .then(one_of("0123456789").repeated().at_least(2).at_most(2))
        .then(just("-"))
        .then(one_of("0123456789").repeated().at_least(2).at_most(2));
    let daytime = none_of(" \t+-]>0123456789\n").repeated().at_least(1);

    let date = yyyymmdd.then(whitespaces_g1().then(daytime).or_not());
    let time = one_of("0123456789")
        .repeated()
        .at_least(1)
        .at_most(2)
        .then(just(":"))
        .then(one_of("0123456789").repeated().at_least(2).at_most(2));

    let repeater_or_delay = just("++")
        .or(just(".+"))
        .or(just("+"))
        .or(just("--"))
        .or(just("-"))
        .then(one_of("0123456789").repeated().at_least(1))
        .then(one_of("hdwmy"));

    let p1a = just("<")
        .then(date.clone())
        .then(whitespaces_g1().then(time).or_not())
        .then(
            whitespaces_g1()
                .then(repeater_or_delay)
                .repeated()
                .at_most(2),
        )
        .then(just(">"))
        .to_slice()
        .map(|s| crate::node!(OSK::Timestamp, vec![crate::token!(OSK::Text, s)]));

    let p1b = just("[")
        .then(date.clone())
        .then(whitespaces_g1().then(time).or_not())
        .then(
            whitespaces_g1()
                .then(repeater_or_delay)
                .repeated()
                .at_most(2),
        )
        .then(just("]"))
        .to_slice()
        .map(|s| crate::node!(OSK::Timestamp, vec![crate::token!(OSK::Text, s)]));

    let p2a = p1a
        .clone()
        .then(just("--"))
        .then(p1a.clone())
        .to_slice()
        .map(|s| crate::node!(OSK::Timestamp, vec![crate::token!(OSK::Text, s)]));

    let p2b = p1b
        .clone()
        .then(just("--"))
        .then(p1b.clone())
        .to_slice()
        .map(|s| crate::node!(OSK::Timestamp, vec![crate::token!(OSK::Text, s)]));

    let p3a = just("<")
        .then(date.clone())
        .then(whitespaces_g1().then(time).then(just("-").then(time)))
        .then(
            whitespaces_g1()
                .then(repeater_or_delay)
                .repeated()
                .at_most(2),
        )
        .then(just(">"))
        .to_slice()
        .map(|s| crate::node!(OSK::Timestamp, vec![crate::token!(OSK::Text, s)]));

    let p3b = just("[")
        .then(date.clone())
        .then(whitespaces_g1().then(time).then(just("-").then(time)))
        .then(
            whitespaces_g1()
                .then(repeater_or_delay)
                .repeated()
                .at_most(2),
        )
        .then(just("]"))
        .to_slice()
        .map(|s| crate::node!(OSK::Timestamp, vec![crate::token!(OSK::Text, s)]));

    choice((p2a, p2b, p3a, p3b, p1a, p1b)).boxed()
    // Parser::boxed(choice((p2a, p2b, p3a, p3b, p1a, p1b)))
    // p2a.or(p2b).or(p3a).or(p3b).or(p1a).or(p1b)
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parsers_output;
    use crate::parser::config::OrgParserConfig;
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_timestamp_01() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(OrgParserConfig::default()),
                r"[2004-08-24 Tue]--[2004-08-26 Thu]"
            ),
            r##"Root@0..34
  Timestamp@0..34
    Text@0..34 "[2004-08-24 Tue]--[20 ..."
"##
        );
    }

    #[test]
    fn test_timestamp_02() {
        assert_eq!(
            get_parsers_output(
                object::objects_parser::<()>(OrgParserConfig::default()),
                r"<2030-10-05 Sat +1m -3d>"
            ),
            r##"Root@0..24
  Timestamp@0..24
    Text@0..24 "<2030-10-05 Sat +1m -3d>"
"##
        );
    }
}
