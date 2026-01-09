//! citation
// todo: check bibliographies
use crate::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;

fn citation_key_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone {
    any()
        .filter(|c: &char| {
            c.is_alphanumeric()
                || matches!(
                    c,
                    '-' | '.'
                        | ':'
                        | '?'
                        | '!'
                        | '`'
                        | '\''
                        | '/'
                        | '*'
                        | '@'
                        | '+'
                        | '|'
                        | '('
                        | ')'
                        | '{'
                        | '}'
                        | '<'
                        | '>'
                        | '&'
                        | '_'
                        | '^'
                        | '$'
                        | '#'
                        | '%'
                        | '~'
                )
        })
        .repeated()
        .at_least(1)
        .to_slice()
}

fn citation_reference_parser_inner<'a, C: 'a>(
    key_prefix: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
    key_suffix: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let key = citation_key_parser();

    group((key_prefix.or_not(), just("@"), key, key_suffix.or_not()))
        .map(|(maybe_key_prefix, at, key, maybe_key_suffix)| {
            let mut children = Vec::with_capacity(4);
            if let Some(key_prefix) = maybe_key_prefix {
                children.push(crate::node!(OSK::CitationReferenceKeyPrefix, key_prefix));
            }
            children.push(crate::token!(OSK::At, at));
            children.push(crate::token!(OSK::CitationReferenceKey, key));
            if let Some(key_suffix) = maybe_key_suffix {
                children.push(crate::node!(OSK::CitationReferenceKeyPrefix, key_suffix));
            }

            crate::node!(OSK::CitationReference, children)
        })
        .boxed()
}

fn citation_reference_parser<'a, C: 'a>(
    object_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let minimal_objects_parser = object_parser
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<NT>>();

    let key = citation_key_parser();

    let mut prefix_single_expression = Recursive::declare(); // foo / [foo] / [[[foo]]]
    prefix_single_expression.define(choice((
        none_of("[];")
            .and_is(group((just("@"), key.clone())).not())
            .repeated()
            .at_least(1)
            .to_slice(),
        prefix_single_expression
            .clone()
            .repeated()
            .delimited_by(just('['), just(']'))
            .to_slice(),
    )));

    let key_prefix = minimal_objects_parser
        .clone()
        .nested_in(prefix_single_expression.repeated().at_least(1).to_slice());

    let mut suffix_single_expression = Recursive::declare(); // foo / [foo] / [[[foo]]]
    suffix_single_expression.define(choice((
        none_of("[];").repeated().at_least(1).to_slice(),
        suffix_single_expression
            .clone()
            .repeated()
            .delimited_by(just('['), just(']'))
            .to_slice(),
    )));
    let key_suffix = minimal_objects_parser
        .nested_in(suffix_single_expression.repeated().at_least(1).to_slice());

    citation_reference_parser_inner(key_prefix, key_suffix)
}

fn simple_citation_reference_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let key = citation_key_parser();

    let mut prefix_single_expression = Recursive::declare(); // foo / [foo] / [[[foo]]]
    prefix_single_expression.define(choice((
        none_of("[];")
            .and_is(group((just("@"), key.clone())).not())
            .repeated()
            .at_least(1)
            .to_slice(),
        prefix_single_expression
            .clone()
            .repeated()
            .delimited_by(just('['), just(']'))
            .to_slice(),
    )));

    let key_prefix = prefix_single_expression
        .repeated()
        .at_least(1)
        .to_slice()
        .map(|s| vec![crate::token!(OSK::Text, s)]);

    let mut suffix_single_expression = Recursive::declare(); // foo / [foo] / [[[foo]]]
    suffix_single_expression.define(choice((
        none_of("[];").repeated().at_least(1).to_slice(),
        suffix_single_expression
            .clone()
            .repeated()
            .delimited_by(just('['), just(']'))
            .to_slice(),
    )));
    let key_suffix = suffix_single_expression
        .repeated()
        .at_least(1)
        .to_slice()
        .map(|s| vec![crate::token!(OSK::Text, s)]);

    citation_reference_parser_inner(key_prefix, key_suffix)
}

pub(crate) fn citation_parser_inner<'a, C: 'a>(
    global_prefix: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
    reference: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
    global_suffix: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let references = reference
        .separated_by(just(";"))
        .at_least(1)
        .collect::<Vec<_>>();

    let citestyle = group((
        just("/"),
        any()
            .filter(|c: &char| c.is_alphanumeric() || matches!(c, '-' | '_'))
            .repeated()
            .at_least(1),
        just("/")
            .then(
                any()
                    .filter(|c: &char| c.is_alphanumeric() || matches!(c, '-' | '_' | '/'))
                    .repeated()
                    .at_least(1),
            )
            .or_not(),
    ))
    .to_slice();

    group((
        just("cite"),
        citestyle.or_not(),
        just(":"),
        global_prefix.then(just(";")).or_not(),
        references,
        just(";").then(global_suffix).or_not(),
    ))
    .delimited_by(just("["), just("]"))
    .map(
        |(
            cite,
            maybe_citestyle,
            colon,
            maybe_global_prefix_semicolon,
            references,
            maybe_semicolon_global_suffix,
        )| {
            let mut children = Vec::with_capacity(10 + references.len());
            children.push(crate::token!(OSK::LeftSquareBracket, "["));
            children.push(crate::token!(OSK::Text, cite));

            if let Some(citestyle) = maybe_citestyle {
                children.push(crate::token!(OSK::CitationCitestyle, citestyle));
            }

            children.push(crate::token!(OSK::Colon, colon));

            if let Some((global_prefix, semicolon)) = maybe_global_prefix_semicolon {
                children.push(crate::node!(OSK::CitationGlobalPrefix, global_prefix));
                children.push(crate::token!(OSK::Semicolon, semicolon));
            }

            for reference in references[0..(references.len()) - 1].into_iter() {
                children.push(reference.clone());
                children.push(crate::token!(OSK::Semicolon, ";"))
            }
            children.push(references[references.len() - 1].clone());

            if let Some((semicolon, global_suffix)) = maybe_semicolon_global_suffix {
                children.push(crate::token!(OSK::Semicolon, semicolon));
                children.push(crate::node!(OSK::CitationGlobalSuffix, global_suffix));
            }
            children.push(crate::token!(OSK::LeftSquareBracket, "]"));

            crate::node!(OSK::Citation, children)
        },
    )
    .boxed()
}

// citation <- "[" "cite"  CITESTYLE ":" GLOBALPREFIX? REFERENCES GLOBALSUFFIX?  "]"
pub(crate) fn citation_parser<'a, C: 'a>(
    minimal_object_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
    standarset_object_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let reference = citation_reference_parser(minimal_object_parser);
    let key = citation_key_parser();
    let mut prefix_single_expression = Recursive::declare(); // foo / [foo] / [[[foo]
    prefix_single_expression.define(choice((
        none_of("[];")
            .and_is(group((just("@"), key.clone())).not())
            .repeated()
            .at_least(1)
            .to_slice(),
        prefix_single_expression
            .clone()
            .repeated()
            .delimited_by(just('['), just(']'))
            .to_slice(),
    )));
    let standarset_objects_parser = standarset_object_parser
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<NT>>();
    let global_prefix = standarset_objects_parser
        .clone()
        .nested_in(prefix_single_expression.repeated().at_least(1).to_slice());
    let global_suffix = global_prefix.clone();

    citation_parser_inner(global_prefix, reference, global_suffix)
}

pub(crate) fn simple_citation_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let reference = simple_citation_reference_parser();
    let key = citation_key_parser();
    let mut prefix_single_expression = Recursive::declare(); // foo / [foo] / [[[foo]
    prefix_single_expression.define(choice((
        none_of("[];")
            .and_is(group((just("@"), key.clone())).not())
            .repeated()
            .at_least(1)
            .to_slice(),
        prefix_single_expression
            .clone()
            .repeated()
            .delimited_by(just('['), just(']'))
            .to_slice(),
    )));
    let global_prefix = prefix_single_expression
        .repeated()
        .at_least(1)
        .to_slice()
        .map(|s| vec![crate::token!(OSK::Text, s)]);
    let global_suffix = global_prefix.clone();

    citation_parser_inner(global_prefix, reference, global_suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use crate::parser::config::OrgParserConfig;
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_citation_reference_01() {
        let input = r##"see @key p. 123"##;
        let expected_output = r##"CitationReference@0..15
  CitationReferenceKeyPrefix@0..4
    Text@0..4 "see "
  At@4..5 "@"
  CitationReferenceKey@5..8 "key"
  CitationReferenceKeyPrefix@8..15
    Text@8..15 " p. 123"
"##;
        let minimal_set_object =
            object::minimal_set_object_parser::<()>(OrgParserConfig::default());

        assert_eq!(
            get_parser_output(citation_reference_parser(minimal_set_object), input),
            expected_output,
        );
    }

    #[test]
    fn test_citation_01() {
        let input = r##"[cite:@key]"##;
        let expected_output = r##"Citation@0..11
  LeftSquareBracket@0..1 "["
  Text@1..5 "cite"
  Colon@5..6 ":"
  CitationReference@6..10
    At@6..7 "@"
    CitationReferenceKey@7..10 "key"
  LeftSquareBracket@10..11 "]"
"##;
        let minimal_set_object =
            object::minimal_set_object_parser::<()>(OrgParserConfig::default());

        let standard_set_object =
            object::standard_set_object_parser::<()>(OrgParserConfig::default());

        assert_eq!(
            get_parser_output(
                citation_parser(minimal_set_object, standard_set_object),
                input
            ),
            expected_output,
        );
    }

    #[test]
    fn test_citation_02() {
        let input = r##"[cite/t: see;@source1;@source2;by Smith /et al./]"##;
        let expected_output = r##"Citation@0..49
  LeftSquareBracket@0..1 "["
  Text@1..5 "cite"
  CitationCitestyle@5..7 "/t"
  Colon@7..8 ":"
  CitationGlobalPrefix@8..12
    Text@8..12 " see"
  Semicolon@12..13 ";"
  CitationReference@13..21
    At@13..14 "@"
    CitationReferenceKey@14..21 "source1"
  Semicolon@21..22 ";"
  CitationReference@22..30
    At@22..23 "@"
    CitationReferenceKey@23..30 "source2"
  Semicolon@30..31 ";"
  CitationGlobalSuffix@31..48
    Text@31..40 "by Smith "
    Italic@40..48
      Slash@40..41 "/"
      Text@41..47 "et al."
      Slash@47..48 "/"
  LeftSquareBracket@48..49 "]"
"##;
        let minimal_set_object =
            object::minimal_set_object_parser::<()>(OrgParserConfig::default());

        let standard_set_object =
            object::standard_set_object_parser::<()>(OrgParserConfig::default());

        assert_eq!(
            get_parser_output(
                citation_parser(minimal_set_object, standard_set_object),
                input
            ),
            expected_output,
        );
    }

    #[test]
    fn test_citation_03() {
        let input = r##"[cite/t:see;@foo p. 7;@bar pp. 4;by foo]"##;
        let expected_output = r##"Citation@0..40
  LeftSquareBracket@0..1 "["
  Text@1..5 "cite"
  CitationCitestyle@5..7 "/t"
  Colon@7..8 ":"
  CitationGlobalPrefix@8..11
    Text@8..11 "see"
  Semicolon@11..12 ";"
  CitationReference@12..21
    At@12..13 "@"
    CitationReferenceKey@13..16 "foo"
    CitationReferenceKeyPrefix@16..21
      Text@16..21 " p. 7"
  Semicolon@21..22 ";"
  CitationReference@22..32
    At@22..23 "@"
    CitationReferenceKey@23..26 "bar"
    CitationReferenceKeyPrefix@26..32
      Text@26..32 " pp. 4"
  Semicolon@32..33 ";"
  CitationGlobalSuffix@33..39
    Text@33..39 "by foo"
  LeftSquareBracket@39..40 "]"
"##;
        let minimal_set_object =
            object::minimal_set_object_parser::<()>(OrgParserConfig::default());

        let standard_set_object =
            object::standard_set_object_parser::<()>(OrgParserConfig::default());

        assert_eq!(
            get_parser_output(
                citation_parser(minimal_set_object, standard_set_object),
                input
            ),
            expected_output,
        );
    }

    #[test]
    fn test_citation_04() {
        let input =
            r##"[cite/a/f:c.f.;the very important @@atkey @ once;the crucial @baz vol. 3]"##;
        let expected_output = r##"Citation@0..73
  LeftSquareBracket@0..1 "["
  Text@1..5 "cite"
  CitationCitestyle@5..9 "/a/f"
  Colon@9..10 ":"
  CitationGlobalPrefix@10..14
    Text@10..14 "c.f."
  Semicolon@14..15 ";"
  CitationReference@15..48
    CitationReferenceKeyPrefix@15..34
      Text@15..34 "the very important "
    At@34..35 "@"
    CitationReferenceKey@35..41 "@atkey"
    CitationReferenceKeyPrefix@41..48
      Text@41..48 " @ once"
  Semicolon@48..49 ";"
  CitationReference@49..72
    CitationReferenceKeyPrefix@49..61
      Text@49..61 "the crucial "
    At@61..62 "@"
    CitationReferenceKey@62..65 "baz"
    CitationReferenceKeyPrefix@65..72
      Text@65..72 " vol. 3"
  LeftSquareBracket@72..73 "]"
"##;
        let minimal_set_object =
            object::minimal_set_object_parser::<()>(OrgParserConfig::default());

        let standard_set_object =
            object::standard_set_object_parser::<()>(OrgParserConfig::default());

        assert_eq!(
            get_parser_output(
                citation_parser(minimal_set_object, standard_set_object),
                input
            ),
            expected_output,
        );
    }

    #[test]
    fn test_citation_05() {
        let input = r##"[cite/style:common prefix ;prefix @key suffix; prefix @foo suffix ; common suffix]"##;
        let expected_output = r##"Citation@0..82
  LeftSquareBracket@0..1 "["
  Text@1..5 "cite"
  CitationCitestyle@5..11 "/style"
  Colon@11..12 ":"
  CitationGlobalPrefix@12..26
    Text@12..26 "common prefix "
  Semicolon@26..27 ";"
  CitationReference@27..45
    CitationReferenceKeyPrefix@27..34
      Text@27..34 "prefix "
    At@34..35 "@"
    CitationReferenceKey@35..38 "key"
    CitationReferenceKeyPrefix@38..45
      Text@38..45 " suffix"
  Semicolon@45..46 ";"
  CitationReference@46..66
    CitationReferenceKeyPrefix@46..54
      Text@46..54 " prefix "
    At@54..55 "@"
    CitationReferenceKey@55..58 "foo"
    CitationReferenceKeyPrefix@58..66
      Text@58..66 " suffix "
  Semicolon@66..67 ";"
  CitationGlobalSuffix@67..81
    Text@67..81 " common suffix"
  LeftSquareBracket@81..82 "]"
"##;
        let minimal_set_object =
            object::minimal_set_object_parser::<()>(OrgParserConfig::default());

        let standard_set_object =
            object::standard_set_object_parser::<()>(OrgParserConfig::default());

        assert_eq!(
            get_parser_output(
                citation_parser(minimal_set_object, standard_set_object),
                input
            ),
            expected_output,
        );
    }
}
