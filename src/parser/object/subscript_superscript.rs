//! Subscript and Superscript
use crate::parser::{MyExtra, NT, OSK, object};
use chumsky::prelude::*;

use std::ops::Range;

// CHARS FINAL parser:
// - find the longest string consisting of <alphanumeric characters, commas, backslashes, and dots>, whose length>=1
// - find the last alphnumeric character as FINAL
fn chars_final_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, String, MyExtra<'a, C>> + Clone {
    custom(|inp| {
        let remaining: &str = inp.slice_from(std::ops::RangeFrom {
            start: &inp.cursor(),
        });

        let content: String = remaining
            .chars()
            .take_while(|c| c.is_alphanumeric() || matches!(c, ',' | '\\' | '.'))
            .collect();

        let maybe_final = content
            .char_indices()
            .rev()
            .find(|(_, c)| c.is_alphanumeric());

        let (idx, _) = maybe_final.ok_or_else(|| {
            let n_char = content.chars().count();
            Rich::custom(
                SimpleSpan::from(Range {
                    start: *inp.cursor().inner(),
                    end: (inp.cursor().inner() + n_char),
                }),
                format!(
                    "superscript must include at least one alphanumeric char: '{}'",
                    content
                ),
            )
        })?;

        let chars_final = content.chars().take(idx + 1).collect::<String>();
        for _ in 0..idx + 1 {
            inp.next();
        }
        Ok(chars_final)
    })
}

enum ScriptType {
    Super,
    Sub,
}

// inner function for public logic
fn create_script_parser_inner<'a, C: 'a, E>(
    script_type: ScriptType,
    expression_parser: E,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone
where
    E: Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
{
    let (c, syntax_kind) = match script_type {
        ScriptType::Super => ("^", OSK::Superscript),
        ScriptType::Sub => ("_", OSK::Subscript),
    };

    // bracket char to str
    let lb_str = |lb| match lb {
        '(' => "(",
        '{' => "{",
        _ => unreachable!(),
    };
    let rb_str = |rb| match rb {
        ')' => ")",
        '}' => "}",
        _ => unreachable!(),
    };

    let sign = one_of("+-").or_not();
    let t3_t1_t2 = object::prev_valid_parser(|c| c.map_or(false, |c| !c.is_whitespace()))
        .ignore_then(just(c))
        .then(choice((
            // CHAR^ SIGN CHARS FINAL
            sign.then(chars_final_parser())
                .to_slice()
                .map(|sign_content| vec![crate::token!(OSK::Text, sign_content)]),
            // CHAR^* or CHAR_*
            just("*").map(|aes| vec![crate::token!(OSK::Text, aes)]),
            // CHAR^{expression} / CHAR^(EXPRESSION)
            one_of("({")
                .then(expression_parser)
                .then_with_ctx(
                    // ctx type -> expression has ctx type
                    just('a').configure(|cfg, ctx: &(char, _)| {
                        let bracket_close = match (*ctx).0 {
                            '(' => ')',
                            '{' => '}',
                            _ => unreachable!(),
                        };
                        cfg.seq(bracket_close)
                    }),
                )
                .map(move |((lb, expression), rb)| {
                    let mut children = Vec::with_capacity(expression.len() + 2);
                    children.push(crate::token!(
                        match lb {
                            '{' => OSK::LeftCurlyBracket,
                            '(' => OSK::LeftRoundBracket,
                            _ => unreachable!(),
                        },
                        lb_str(lb)
                    ));
                    children.extend(expression);
                    children.push(crate::token!(OSK::RightCurlyBracket, rb_str(rb)));
                    children
                }),
        )))
        .map(move |(sup, others)| {
            let mut children = vec![];

            let token = match sup {
                "^" => crate::token!(OSK::Caret, sup),
                "_" => crate::token!(OSK::Underscore, sup),
                _ => unreachable!(),
            };
            children.push(token);
            children.extend(others);

            crate::node!(syntax_kind.clone(), children)
        })
        .boxed();

    // let t3_t1_t2 = just(c)
    //     .try_map_with(|s, e| {
    //         // check PRE
    //         let pre_valid = (e.state() as &mut RollbackState<ParserState>)
    //             .prev_char
    //             .map_or(false, |c| !c.is_whitespace());

    //         match pre_valid {
    //             true => Ok(s),
    //             false => Err(Rich::<char>::custom(
    //                 e.span(),
    //                 format!(
    //                     "sub/sup script parser: pre_valid={pre_valid}, PRE={:?} not valid",
    //                     (e.state() as &mut RollbackState<ParserState>).prev_char
    //                 ),
    //             )),
    //         }
    //     })
    //     .then(choice((
    //         // CHAR^ SIGN CHARS FINAL
    //         sign.then(chars_final_parser())
    //             .to_slice()
    //             .map_with(|sign_content, e| {
    //                 (e.state() as &mut RollbackState<ParserState>).prev_char =
    //                     sign_content.chars().last();

    //                 vec![crate::token!(OSK::Text, sign_content)]
    //             }),
    //         // CHAR^* or CHAR_*
    //         just("*").map_with(|aes, e| {
    //             (e.state() as &mut RollbackState<ParserState>).prev_char = Some('*');
    //             vec![crate::token!(OSK::Text, aes)]
    //         }),
    //         // CHAR^{expression} / CHAR^(EXPRESSION)
    //         one_of("({")
    //             .map_with(|s: char, e| {
    //                 // update state for next expression parser: nest + state + ctx
    //                 (e.state() as &mut RollbackState<ParserState>).prev_char = Some(s);
    //                 s
    //             })
    //             .then(expression_parser)
    //             .then_with_ctx(
    //                 // ctx type -> expression has ctx type
    //                 just('a').configure(|cfg, ctx: &(char, _)| {
    //                     let bracket_close = match (*ctx).0 {
    //                         '(' => ')',
    //                         '{' => '}',
    //                         _ => unreachable!(),
    //                     };
    //                     cfg.seq(bracket_close)
    //                 }),
    //             )
    //             .map_with(move |((lb, expression), rb), e| {
    //                 (e.state() as &mut RollbackState<ParserState>).prev_char = Some(rb);

    //                 let mut children = Vec::with_capacity(expression.len() + 2);
    //                 children.push(crate::token!(
    //                     match lb {
    //                         '{' => OSK::LeftCurlyBracket,
    //                         '(' => OSK::LeftRoundBracket,
    //                         _ => unreachable!(),
    //                     },
    //                     lb_str(lb)
    //                 ));
    //                 children.extend(expression);
    //                 children.push(crate::token!(OSK::RightCurlyBracket, rb_str(rb)));

    //                 children
    //             }),
    //     )))
    //     .map(move |(sup, others)| {
    //         let mut children = vec![];

    //         let token = match sup {
    //             "^" => crate::token!(OSK::Caret, sup),
    //             "_" => crate::token!(OSK::Underscore, sup),
    //             _ => unreachable!(),
    //         };
    //         children.push(token);
    //         children.extend(others);

    //         crate::node!(syntax_kind.clone(), children)
    //     })
    //     .boxed();

    t3_t1_t2
}

fn create_simple_script_parser<'a, C: 'a>(
    script_type: ScriptType,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let var = none_of("{}()\r\n").repeated().at_least(1).to_slice();
    let mut single_expression = Recursive::declare(); // foo / (foo) / (((foo)))
    single_expression.define(
        var.or(just("(")
            .then(single_expression.clone().repeated())
            .then(just(")"))
            .to_slice())
            .or(just("{")
                .then(single_expression.clone().repeated())
                .then(just("}"))
                .to_slice()),
    );

    let expression_parser = single_expression
        .clone()
        .repeated()
        .to_slice()
        .map(|text: &str| vec![crate::token!(OSK::Text, text)]);

    create_script_parser_inner(script_type, expression_parser)
}

fn create_script_parser<'a, C: 'a>(
    script_type: ScriptType,
    object_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let var = none_of("{}()\r\n").repeated().at_least(1).to_slice();
    let mut single_expression = Recursive::declare(); // foo / (foo) / (((foo)))
    single_expression.define(
        var.or(just("(")
            .then(single_expression.clone().repeated())
            .then(just(")"))
            .to_slice())
            .or(just("{")
                .then(single_expression.clone().repeated())
                .then(just("}"))
                .to_slice()),
    );

    let standard_objects_parser = object_parser
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<NT>>();

    let expression_parser =
        standard_objects_parser.nested_in(single_expression.clone().repeated().to_slice());

    create_script_parser_inner(script_type, expression_parser)
}

pub(crate) fn subscript_parser<'a, C: 'a>(
    object_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    create_script_parser(ScriptType::Sub, object_parser)
}

pub(crate) fn superscript_parser<'a, C: 'a>(
    object_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    create_script_parser(ScriptType::Super, object_parser)
}

pub(crate) fn simple_subscript_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    create_simple_script_parser(ScriptType::Sub)
}

pub(crate) fn simple_superscript_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    create_simple_script_parser(ScriptType::Super)
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parsers_output;
    use crate::parser::object;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_subscript_01() {
        // fox_bar
        // 否定前瞻过程中, markup解析，中间步骤解析standard_objects.nested(content)成功, 会更新prev_char, 后续marker_end解析失败，但状态不恢复，导致状态混乱
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"fox_bar"),
            r###"Root@0..7
  Text@0..3 "fox"
  Subscript@3..7
    Underscore@3..4 "_"
    Text@4..7 "bar"
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"fox_{bar}"),
            r###"Root@0..9
  Text@0..3 "fox"
  Subscript@3..9
    Underscore@3..4 "_"
    LeftCurlyBracket@4..5 "{"
    Text@5..8 "bar"
    RightCurlyBracket@8..9 "}"
"###
        );
    }

    #[test]
    fn test_subscript_02_bad() {
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"fo _{bar}"),
            r###"Root@0..9
  Text@0..9 "fo _{bar}"
"###
        );
    }

    #[test]
    fn test_subscript_03() {
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"a {*b_foo*}"),
            r###"Root@0..11
  Text@0..3 "a {"
  Bold@3..10
    Asterisk@3..4 "*"
    Text@4..5 "b"
    Subscript@5..9
      Underscore@5..6 "_"
      Text@6..9 "foo"
    Asterisk@9..10 "*"
  Text@10..11 "}"
"###
        );
    }

    #[test]
    fn test_subscript_04() {
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"a {*_foo*}"),
            r###"Root@0..10
  Text@0..3 "a {"
  Bold@3..9
    Asterisk@3..4 "*"
    Text@4..8 "_foo"
    Asterisk@8..9 "*"
  Text@9..10 "}"
"###
        );
    }

    #[test]
    fn test_superscript_01_bold() {
        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"a^{*bold*}"),
            r###"Root@0..10
  Text@0..1 "a"
  Superscript@1..10
    Caret@1..2 "^"
    LeftCurlyBracket@2..3 "{"
    Bold@3..9
      Asterisk@3..4 "*"
      Text@4..8 "bold"
      Asterisk@8..9 "*"
    RightCurlyBracket@9..10 "}"
"###
        );
    }
}
