//! Subscript and Superscript
use crate::parser::ParserState;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

use std::collections::HashMap;
use std::ops::Range;
type NT = NodeOrToken<GreenNode, GreenToken>;
type OSK = OrgSyntaxKind;

// CHARS FINAL parser:
// - find the longest string consisting of <alphanumeric characters, commas, backslashes, and dots>, whose length>=1
// - find the last alphnumeric character as FINAL
fn chars_final_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>> + Clone
{
    custom::<_, &str, _, extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>>(|inp| {
        let remaining = inp.slice_from(std::ops::RangeFrom {
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

fn create_script_parser<'a, C: 'a>(
    script_type: ScriptType,
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
    let (c, syntax_kind) = match script_type {
        ScriptType::Super => ("^", OSK::Superscript),
        ScriptType::Sub => ("_", OSK::Subscript),
    };

    // ^* or _*
    let t1 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>(c)
        .then(just("*"))
        .map_with(move |(sup, aes), e| {
            e.state().prev_char = Some('*');
            let mut children = vec![];
            children.push(NT::Token(GreenToken::new(OSK::Caret.into(), sup)));
            children.push(NT::Token(GreenToken::new(OSK::Text.into(), aes)));
            NT::Node(GreenNode::new(syntax_kind.clone().into(), children))
        });

    // CHAR^{expression} / CHAR^(EXPRESSION)
    let var = none_of::<&str, &str, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>(
        "{}()\r\n",
    )
    .repeated()
    .at_least(1)
    .to_slice();
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
        .collect::<Vec<NodeOrToken<GreenNode, GreenToken>>>();
    let expression =
        standard_objects_parser.nested_in(single_expression.clone().repeated().to_slice()); // foo(bar){(def)ghi}

    let pairs = HashMap::from([('(', ')'), ('{', '}')]);
    let pair_starts = pairs.keys().copied().collect::<Vec<_>>();
    let pair_ends = pairs.values().copied().collect::<Vec<_>>();
    let t2 = just::<_, _, extra::Full<Rich<'_, char>, RollbackState<ParserState>, C>>(c)
        .then(one_of(pair_starts))
        .map_with(|(a, b): (&str, char), e| {
            let tmp = e.state().prev_char;
            e.state().prev_char_backup.push(tmp);

            // println!("t2a: r={:?}", e.state());
            // if e.state().prev_char_backup.len()>3{
            //     println!("subscript: state={:?}", e.state().prev_char_backup.len());
            // }
            e.state().prev_char = Some(b);
            (a, b)
        })
        .then(expression)
        .map_with(|s, e| {
            // println!("t2b: r={:?}", e.state());
            e.state().prev_char = e.state().prev_char_backup.pop().unwrap(); // resume prev_char

            s
        })
        .then(one_of(pair_ends))
        .try_map_with(move |(((sup, lb), expression), rb), e| {
            let pre_valid = e
                .state()
                .prev_char
                .map_or(false, |c| !matches!(c, ' ' | '\t'));
            // println!("pre_valid={pre_valid:?}, expression={expression:?}, prev_char={:?}", e.state().prev_char);
            match pre_valid {
                false => {
                    let error = Rich::custom::<&str>(e.span(), &format!("PRE not valid"));
                    Err(error)
                }

                true => {
                    let expected_rb = *pairs.get(&lb).unwrap();

                    if rb != expected_rb {
                        Err(Rich::custom::<&str>(
                            e.span(),
                            &format!("bracket not matched: {lb} {rb}"),
                        ))
                    } else {
                        e.state().prev_char = Some(rb);

                        let mut children = Vec::with_capacity(expression.len() + 3);
                        children.push(NT::Token(GreenToken::new(OSK::Caret.into(), sup)));
                        children.push(NT::Token(GreenToken::new(
                            OSK::LeftCurlyBracket.into(),
                            lb.to_string().as_str(),
                        )));
                        children.extend(expression);
                        children.push(NT::Token(GreenToken::new(
                            OSK::RightCurlyBracket.into(),
                            rb.to_string().as_str(),
                        )));

                        Ok(NT::Node(GreenNode::new(
                            syntax_kind.clone().into(),
                            children,
                        )))
                    }
                }
            }
        });

    // ^ SIGN CHARS FINAL
    let sign = one_of("+-").or_not();
    let t3 = just(c).then(sign).then(chars_final_parser()).try_map_with(
        move |((sup, sign), content), e| match e.state().prev_char {
            None => Err(Rich::custom(e.span(), format!("CHAR is empty"))),
            Some(c) if matches!(c, ' ' | '\t') => {
                Err(Rich::custom(e.span(), format!("CHAR is whitesace")))
            }
            _ => {
                e.state().prev_char = content.chars().last();

                let mut children = vec![];
                children.push(NT::Token(GreenToken::new(OSK::Caret.into(), sup)));
                let text = sign.map_or_else(|| content.clone(), |s| format!("{s}{content}"));
                children.push(NT::Token(GreenToken::new(OSK::Text.into(), &text)));

                Ok(NT::Node(GreenNode::new(syntax_kind.into(), children)))
            }
        },
    );

    Parser::boxed(choice((t1, t3, t2)))
    // t1.or(t3).or(t2)
}

pub(crate) fn subscript_parser<'a, C: 'a>(
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
    create_script_parser(ScriptType::Sub, object_parser)
}

pub(crate) fn superscript_parser<'a, C: 'a>(
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
    create_script_parser(ScriptType::Super, object_parser)
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
    Caret@3..4 "_"
    Text@4..7 "bar"
"###
        );

        assert_eq!(
            get_parsers_output(object::objects_parser::<()>(), r"fox_{bar}"),
            r###"Root@0..9
  Text@0..3 "fox"
  Subscript@3..9
    Caret@3..4 "_"
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
