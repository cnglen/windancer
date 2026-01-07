//! Latex fragment parser
use crate::constants::entity::ENTITYNAME_TO_HTML;
use crate::parser::{MyExtra, NT, OSK, object};
use chumsky::prelude::*;

/// Latex Frament parser
pub(crate) fn latex_fragment_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a {
    // latex style:
    // - \(CONTENTS\)
    // - \[CONTENTS\]
    let t_latex = just(r"\")
        .then(choice((
            group((
                just(r"("),
                any().and_is(just(r"\)").not()).repeated().to_slice(),
                just(r"\"),
                just(r")"),
            )),
            group((
                just("["),
                any().and_is(just(r"\]").not()).repeated().to_slice(),
                just(r"\"),
                just("]"),
            )),
        )))
        .map(|(backslash_open, (lb, content, backslash_close, rb))| {
            crate::node!(
                OSK::LatexFragment,
                vec![
                    crate::token!(OSK::BackSlash, backslash_open),
                    match lb {
                        r"(" => crate::token!(OSK::LeftRoundBracket, lb),
                        _ => crate::token!(OSK::LeftSquareBracket, lb),
                    },
                    crate::token!(OSK::Text, content),
                    crate::token!(OSK::BackSlash, backslash_close),
                    match rb {
                        r")" => crate::token!(OSK::RightRoundBracket, rb),
                        _ => crate::token!(OSK::RightSquareBracket, rb),
                    }
                ]
            )
        });

    // $$CONTENTS$$
    let t_tex_display = group((
        just("$$"),
        // any().and_is(just("$$").not()).repeated().to_slice()
        none_of('$')
            .to_slice()
            .or(just('$').then(none_of('$')).to_slice())
            .repeated()
            .to_slice(),
        just("$$"),
    ))
    .map(|(dd_pre, content, dd_post)| {
        crate::node!(
            OSK::LatexFragment,
            vec![
                crate::token!(OSK::Dollar2, dd_pre),
                crate::token!(OSK::Text, content),
                crate::token!(OSK::Dollar2, dd_post),
            ]
        )
    });

    // PRE$CHAR$POST
    // PRE$BORDER1 BODY BORDER2$POST
    let post = any()
        .filter(|c: &char| c.is_ascii_punctuation() || matches!(c, ' ' | '\t' | '\r' | '\n'))
        .or(end().to('x'));
    let border1 = none_of("\r\n \t.,;$");
    let border2 = none_of("\r\n \t.,$");
    let t_tex_inline = object::prev_valid_parser(|c| c.map_or(true, |e| e != '$'))
        .ignore_then(just("$"))
        .then(choice((
            // PRE$CHAR$POST must be in first order, $|pia$, $pi|$
            // $pia$, $pi$ -> $|pia$, $pi|$
            none_of(".,?;\" \t")
                .to_slice()
                .then(just("$"))
                .then_ignore(post.rewind()),
            // PRE$BORDER1 BODY BORDER2$POST
            border1
                .then(
                    any()
                        .and_is(border2.ignore_then(just("$")).ignored().not())
                        .repeated(),
                )
                .then(border2)
                .to_slice()
                .then(just("$"))
                .then_ignore(post.rewind()),
        )))
        .map(|(d_open, (body, d_close))| {
            crate::node!(
                OSK::LatexFragment,
                vec![
                    crate::token!(OSK::Dollar, d_open),
                    crate::token!(OSK::Text, body),
                    crate::token!(OSK::Dollar, d_close),
                ]
            )
        });

    // \NAME BRACKETS
    // NAME := a string consisting of alphabetic characters which does NOT have an association in either `org-entities`` or `org-entities-user`
    let name = any()
        .filter(|c: &char| c.is_alphabetic())
        .repeated()
        .at_least(1)
        .to_slice()
        .filter(|name: &&str| !ENTITYNAME_TO_HTML.contains_key(*name));

    let t_name_brackets = (just(r##"\"##).then(name))
        .then(
            choice((
                // [CONTENTS1]
                group((
                    just("["),
                    none_of("{}[]\r\n").repeated().to_slice(),
                    just(']'),
                )),
                // {CONTENTS2}
                group((
                    just("{"),
                    none_of("{}\r\n").repeated().to_slice(),
                    just('}'),
                )),
            ))
            .or_not(),
        )
        .to_slice()
        .map(|s| crate::node!(OSK::LatexFragment, vec![crate::token!(OSK::Text, s)]));

    Parser::boxed(choice((
        t_latex,
        t_tex_display,
        t_tex_inline,
        t_name_brackets,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    // extern crate test;
    use crate::parser::common::get_parser_output;
    use crate::parser::common::get_parsers_output;
    use crate::parser::config::OrgParserConfig;
    use crate::parser::object::standard_set_object_parser;
    use pretty_assertions::assert_eq;
    // use test::Bencher;

    #[test]
    fn test_latex_fragment_01() {
        assert_eq!(
            get_parser_output(latex_fragment_parser::<()>(), r"\(\alpha\)"),
            r###"LatexFragment@0..10
  BackSlash@0..1 "\\"
  LeftRoundBracket@1..2 "("
  Text@2..8 "\\alpha"
  BackSlash@8..9 "\\"
  RightRoundBracket@9..10 ")"
"###
        );
    }

    #[test]
    fn test_latex_fragment_02() {
        assert_eq!(
            get_parser_output(
                latex_fragment_parser::<()>(),
                r"\enlargethispage{2\baselineskip}"
            ),
            r###"LatexFragment@0..32
  Text@0..32 "\\enlargethispage{2\\ba ..."
"###
        );
    }

    #[test]
    fn test_latex_fragment_03() {
        assert_eq!(
            get_parser_output(latex_fragment_parser::<()>(), r"\enlargethispage"),
            r###"LatexFragment@0..16
  Text@0..16 "\\enlargethispage"
"###
        );
    }

    #[test]
    fn test_latex_fragment_04() {
        assert_eq!(
            get_parser_output(latex_fragment_parser::<()>(), r"$a+b$"),
            r###"LatexFragment@0..5
  Dollar@0..1 "$"
  Text@1..4 "a+b"
  Dollar@4..5 "$"
"###
        );
    }

    #[test]
    fn test_latex_fragment_05() {
        assert_eq!(
            get_parser_output(latex_fragment_parser::<()>(), r"$a$"),
            r###"LatexFragment@0..3
  Dollar@0..1 "$"
  Text@1..2 "a"
  Dollar@2..3 "$"
"###
        );
    }

    #[test]
    fn test_latex_fragment_06() {
        let standard_objects_parser = standard_set_object_parser::<()>(OrgParserConfig::default())
            .repeated()
            .at_least(1)
            .collect::<Vec<_>>();

        assert_eq!(
            get_parsers_output(standard_objects_parser, r"$$a$"),
            r##"Root@0..4
  Text@0..4 "$$a$"
"##
        );
    }

    // #[bench]
    // fn test_latex_fragment_01_bench(b: &mut Bencher) {
    //     let parser = latex_fragment_parser::<()>();
    //     b.iter(|| {
    //         assert!(!parser.parse(r"\(\alpha\)").has_errors());
    //     })
    // }
}
