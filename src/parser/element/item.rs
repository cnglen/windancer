//! Item parser
use crate::parser::object;
use crate::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;
use std::ops::Range;

pub(crate) fn item_parser<'a, C: 'a>(
    element_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let item_content_inner = object::line_parser() // no need to test indent since this is the first row of item
        .then(
            object::line_parser()
                .and_is(greater_indent_termination()) // <= indented than the startingl line; including next item since next item is equaly indented of the starting line
                .or(object::blank_line_str_parser())
                .and_is(
                    object::blank_line_parser()
                        .repeated()
                        .at_least(2)
                        .ignored()
                        .not(),
                ) // two consecutive blank lines
                .repeated(),
        )
        .to_slice();

    let item_content_parser = element_parser
        .repeated()
        .collect::<Vec<_>>()
        .nested_in(item_content_inner)
        .map(|children| crate::node!(OSK::ListItemContent, children));

    item_indent_parser()
        .then(item_bullet_parser())
        .then(item_counter_set_parser().or_not())
        .then(item_checkbox_parser().or_not())
        .then(item_tag_parser().or_not())
        .validate(|((((indent, bullet), maybe_counter), maybe_checkbox), maybe_tag), e, emitter|{
            // update item_indent state: push
            let current_indent = usize::from(indent.text_len());
            let state_indent_length = e.state().item_indent.len();

            if state_indent_length > 0 { // indent_length vecntor is not empty
                let last_state = e.state().item_indent[state_indent_length - 1];
                if current_indent < last_state {
                    let error = Rich::custom::<&str>(
                        SimpleSpan::from(Range {
                            start: e.span().start(),
                            end: e.span().end(),
                        }),
                        &format!("item_indent_parser: lesser indernt found: current_indent({current_indent}) < state_indent({last_state})"),
                    );
                    emitter.emit(error)
                } else if current_indent > last_state {
                    e.state().item_indent.push(current_indent); // first item of Non-First list in doc
                } else {
                }
            } else {
                e.state().item_indent.push(current_indent); // update state only at the first item of one list
            }
            ((((indent, bullet), maybe_counter), maybe_checkbox), maybe_tag)
        })
        .then(item_content_parser.clone().or_not())
        .then(object::blank_line_parser().or_not())
        .try_map_with(
            |(
                (((((indent, bullet), maybe_counter), maybe_checkbox), maybe_tag), maybe_content,),
                maybe_blankline,
            ), _e| {
                let mut children = Vec::with_capacity(8);
                children.push(indent);
                children.push(bullet);

                children.extend(maybe_counter.into_iter());
                children.extend(maybe_checkbox.into_iter());
                children.extend(maybe_tag.into_iter());
                children.extend(maybe_content.into_iter());
                children.extend(maybe_blankline);

                Ok(crate::node!(
                  OSK::ListItem,
                  children
                ))
            },
        ).boxed()
}

// only used in lookahead
pub(crate) fn simple_item_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone
{
    let item_content_inner = object::line_parser() // no need to test indent since this is the first row of item
        .then(
            object::line_parser()
                .and_is(greater_indent_termination()) // <= indented than the startingl line; including next item since next item is equaly indented of the starting line
                .or(object::blank_line_str_parser())
                .and_is(
                    object::blank_line_parser()
                        .repeated()
                        .at_least(2)
                        .ignored()
                        .not(),
                ) // two consecutive blank lines
                .repeated(),
        );

    item_indent_parser()
        .then(item_bullet_parser())
        .then(item_counter_set_parser().or_not())
        .then(item_checkbox_parser().or_not())
        .then(item_tag_parser().or_not())
        .validate(|((((indent, bullet), maybe_counter), maybe_checkbox), maybe_tag), e, emitter|{
            // update item_indent state: push
            let current_indent = usize::from(indent.text_len());
            let state_indent_length = e.state().item_indent.len();

            if state_indent_length > 0 { // indent_length vecntor is not empty
                let last_state = e.state().item_indent[state_indent_length - 1];
                if current_indent < last_state {
                    let error = Rich::custom::<&str>(
                        SimpleSpan::from(Range {
                            start: e.span().start(),
                            end: e.span().end(),
                        }),
                        &format!("item_indent_parser: lesser indernt found: current_indent({current_indent}) < state_indent({last_state})"),
                    );
                    emitter.emit(error)
                } else if current_indent > last_state {
                    e.state().item_indent.push(current_indent); // first item of Non-First list in doc
                } else {
                }
            } else {
                e.state().item_indent.push(current_indent); // update state only at the first item of one list
            }
            ((((indent, bullet), maybe_counter), maybe_checkbox), maybe_tag)
        })
        .then(item_content_inner.or_not())
        .then(object::blank_line_parser().or_not())
        .ignored()
        .boxed()
}

/// Item Indent Parser
///
/// 功能: 检测whistespace的个数，构造ListItemIndent Node
///
/// 注意:
///   - 仅当是“任意一个List的第一个item”时才更新state["item_indent"]: push
///   - ItemIndent状态不能在这里更新，避免任意一行content的数据，更新状态，导致状态混乱
///   - 保证ItemIndent状态在item_content前更新
pub(crate) fn item_indent_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone
{
    object::whitespaces().map(|whitespaces| {
        let children = if !whitespaces.is_empty() {
            vec![crate::token!(OSK::Whitespace, whitespaces)]
        } else {
            vec![]
        };
        crate::node!(OSK::ListItemIndent, children)
    })
}

pub(crate) fn counter_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone {
    text::int(10)
        .to_slice()
        .or(one_of("abcdefghijklmnopqrstuvwxyz").to_slice())
}

/// Item Bullet Parser
pub(crate) fn item_bullet_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone
{
    one_of("*+-")
        .to_slice()
        .or(counter_parser().then(one_of(".)")).to_slice())
        .then(object::whitespaces_g1())
        .map(|(bullet, ws)| {
            crate::node!(
                OSK::ListItemBullet,
                vec![
                    crate::token!(OSK::Text, bullet),
                    crate::token!(OSK::Whitespace, &ws),
                ]
            )
        })
}

/// Item Counter Parser
pub(crate) fn item_counter_set_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    just("[@")
        .then(text::int(10))
        .then(just("]"))
        .then(object::whitespaces_g1())
        .map(|(((_lbracket_at, number), rbracket), ws)| {
            crate::node!(
                OSK::ListItemCounter,
                vec![
                    crate::token!(OSK::LeftSquareBracket, "["),
                    crate::token!(OSK::At, "@"),
                    crate::token!(OSK::Text, number),
                    crate::token!(OSK::RightSquareBracket, rbracket),
                    crate::token!(OSK::Whitespace, &ws),
                ]
            )
        })
}

/// Item Checkbox Parser
pub(crate) fn item_checkbox_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    just("[")
        .then(just(" ").or(just("-")).or(just("X")))
        .then(just("]"))
        .then(object::whitespaces_g1())
        .map(|(((lbracket, check), rbracket), ws)| {
            crate::node!(
                OSK::ListItemCheckbox,
                vec![
                    crate::token!(OSK::LeftSquareBracket, lbracket),
                    crate::token!(OSK::Text, check),
                    crate::token!(OSK::RightSquareBracket, rbracket),
                    crate::token!(OSK::Whitespace, ws),
                ]
            )
        })
}

/// Item Tag Parser
pub(crate) fn item_tag_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> {
    none_of(object::CRLF)
        .and_is(
            object::whitespaces_g1()
                .ignore_then(just("::"))
                .ignore_then(object::whitespaces_g1())
                .ignored()
                .not(),
        )
        .repeated()
        .at_least(1)
        .to_slice()
        .then(object::whitespaces_g1())
        .then(just("::"))
        .then(object::whitespaces_g1())
        .map(|(((tag, ws1), double_colon), ws2)| {
            crate::node!(
                OSK::ListItemTag,
                vec![
                    crate::token!(OSK::Text, tag),
                    crate::token!(OSK::Whitespace, ws1),
                    crate::token!(OSK::Colon2, double_colon),
                    crate::token!(OSK::Whitespace, ws2),
                ]
            )
        })
}

/// only used for lookahead
// todo: non-paragrahp elements? inlinetask boundary?
// - CONTENTS (optional) :: A collection of zero or more elements, ending at the first instance of one of the following:
//   - The next item.
//   - The first line less or equally indented than the starting line, not counting lines within other non-paragraph elements or inlinetask boundaries.
//   - Two consecutive blank lines.
fn greater_indent_termination<'a, C: 'a>() -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    // todo: not counting non-paragraph elements or inline task boudaries
    object::whitespaces()
        .try_map_with(|ws, e| {
            // .validate(|ws, e, emitter| {
            let current_indent = ws.len();
            let state_indent_length = e.state().item_indent.len(); // 仅在item_content时调用，必然len>0
            let last_state = e.state().item_indent[state_indent_length - 1];
            if current_indent <= last_state {
                // println!("error: lesser_indent_termination: ws=|{ws}|, error 缩进不足 current_indent({current_indent}) <= state_indent({last_state})");
                let error = Rich::custom::<&str>(
                    SimpleSpan::from(Range {
                        start: e.span().start(),
                        end: e.span().end(),
                    }),
                    &format!("lesser_indent_termination: 缩进不足 current_indent({current_indent}) < state_indent({last_state})"),
                );
                // emitter.emit(error);
                Err(error)
            } else {
                // println!("lesser_indent_termination: ws=|{ws}|, ok current_indent({current_indent}) > state_indent({last_state})");
                Ok(ws)
            }
        })
        .ignored()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use crate::parser::element;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_item_01() {
        let input = r##"+ [@3] [X] tag :: item contents
"##;
        assert_eq!(
            get_parser_output(item_parser(element::element_in_item_parser::<()>()), input),
            r##"ListItem@0..32
  ListItemIndent@0..0
  ListItemBullet@0..2
    Text@0..1 "+"
    Whitespace@1..2 " "
  ListItemCounter@2..7
    LeftSquareBracket@2..3 "["
    At@3..4 "@"
    Text@4..5 "3"
    RightSquareBracket@5..6 "]"
    Whitespace@6..7 " "
  ListItemCheckbox@7..11
    LeftSquareBracket@7..8 "["
    Text@8..9 "X"
    RightSquareBracket@9..10 "]"
    Whitespace@10..11 " "
  ListItemTag@11..18
    Text@11..14 "tag"
    Whitespace@14..15 " "
    Colon2@15..17 "::"
    Whitespace@17..18 " "
  ListItemContent@18..32
    Paragraph@18..32
      Text@18..32 "item contents\n"
"##,
        );
    }

    #[test]
    fn test_item_02() {
        assert_eq!(
            get_parser_output(
                item_parser(element::element_parser::<()>()),
                r##"+ [X] tag :: item contents
"##
            ),
            r##"ListItem@0..27
  ListItemIndent@0..0
  ListItemBullet@0..2
    Text@0..1 "+"
    Whitespace@1..2 " "
  ListItemCheckbox@2..6
    LeftSquareBracket@2..3 "["
    Text@3..4 "X"
    RightSquareBracket@4..5 "]"
    Whitespace@5..6 " "
  ListItemTag@6..13
    Text@6..9 "tag"
    Whitespace@9..10 " "
    Colon2@10..12 "::"
    Whitespace@12..13 " "
  ListItemContent@13..27
    Paragraph@13..27
      Text@13..27 "item contents\n"
"##
        );
    }

    #[test]
    fn test_item_03() {
        assert_eq!(
            get_parser_output(
                item_parser(element::element_in_item_parser::<()>()),
                r##"+ [@3] tag :: item contents
"##
            ),
            r##"ListItem@0..28
  ListItemIndent@0..0
  ListItemBullet@0..2
    Text@0..1 "+"
    Whitespace@1..2 " "
  ListItemCounter@2..7
    LeftSquareBracket@2..3 "["
    At@3..4 "@"
    Text@4..5 "3"
    RightSquareBracket@5..6 "]"
    Whitespace@6..7 " "
  ListItemTag@7..14
    Text@7..10 "tag"
    Whitespace@10..11 " "
    Colon2@11..13 "::"
    Whitespace@13..14 " "
  ListItemContent@14..28
    Paragraph@14..28
      Text@14..28 "item contents\n"
"##
        );
    }

    #[test]
    fn test_item_04() {
        assert_eq!(
            get_parser_output(
                item_parser(element::element_in_item_parser::<()>()),
                r##"+ [@3] [X] item contents
"##
            ),
            r##"ListItem@0..25
  ListItemIndent@0..0
  ListItemBullet@0..2
    Text@0..1 "+"
    Whitespace@1..2 " "
  ListItemCounter@2..7
    LeftSquareBracket@2..3 "["
    At@3..4 "@"
    Text@4..5 "3"
    RightSquareBracket@5..6 "]"
    Whitespace@6..7 " "
  ListItemCheckbox@7..11
    LeftSquareBracket@7..8 "["
    Text@8..9 "X"
    RightSquareBracket@9..10 "]"
    Whitespace@10..11 " "
  ListItemContent@11..25
    Paragraph@11..25
      Text@11..25 "item contents\n"
"##
        );
    }

    #[test]
    fn test_item_05() {
        assert_eq!(
            get_parser_output(
                item_parser(element::element_in_item_parser::<()>()),
                r##"+ [@3] [X] tag :: item contents
"##
            ),
            r##"ListItem@0..32
  ListItemIndent@0..0
  ListItemBullet@0..2
    Text@0..1 "+"
    Whitespace@1..2 " "
  ListItemCounter@2..7
    LeftSquareBracket@2..3 "["
    At@3..4 "@"
    Text@4..5 "3"
    RightSquareBracket@5..6 "]"
    Whitespace@6..7 " "
  ListItemCheckbox@7..11
    LeftSquareBracket@7..8 "["
    Text@8..9 "X"
    RightSquareBracket@9..10 "]"
    Whitespace@10..11 " "
  ListItemTag@11..18
    Text@11..14 "tag"
    Whitespace@14..15 " "
    Colon2@15..17 "::"
    Whitespace@17..18 " "
  ListItemContent@18..32
    Paragraph@18..32
      Text@18..32 "item contents\n"
"##
        );
    }

    #[test]
    fn test_item_06() {
        assert_eq!(
            get_parser_output(
                item_parser(element::element_in_item_parser::<()>()),
                r##"+ 
"##
            ),
            r##"ListItem@0..3
  ListItemIndent@0..0
  ListItemBullet@0..2
    Text@0..1 "+"
    Whitespace@1..2 " "
  BlankLine@2..3 "\n"
"##
        );
    }

    #[test]
    fn test_item_07() {
        assert_eq!(
            get_parser_output(
                item_parser(element::element_in_item_parser::<()>()),
                r##"+ foo"##
            ),
            r##"ListItem@0..5
  ListItemIndent@0..0
  ListItemBullet@0..2
    Text@0..1 "+"
    Whitespace@1..2 " "
  ListItemContent@2..5
    Paragraph@2..5
      Text@2..5 "foo"
"##
        );
    }

    #[test]
    fn test_item_08() {
        assert_eq!(
            get_parser_output(
                item_parser(element::element_in_item_parser::<()>()),
                r##"   + [@3] [X] tag :: item contents
"##
            ),
            r##"ListItem@0..35
  ListItemIndent@0..3
    Whitespace@0..3 "   "
  ListItemBullet@3..5
    Text@3..4 "+"
    Whitespace@4..5 " "
  ListItemCounter@5..10
    LeftSquareBracket@5..6 "["
    At@6..7 "@"
    Text@7..8 "3"
    RightSquareBracket@8..9 "]"
    Whitespace@9..10 " "
  ListItemCheckbox@10..14
    LeftSquareBracket@10..11 "["
    Text@11..12 "X"
    RightSquareBracket@12..13 "]"
    Whitespace@13..14 " "
  ListItemTag@14..21
    Text@14..17 "tag"
    Whitespace@17..18 " "
    Colon2@18..20 "::"
    Whitespace@20..21 " "
  ListItemContent@21..35
    Paragraph@21..35
      Text@21..35 "item contents\n"
"##
        );
    }

    #[test]
    #[should_panic]
    fn test_item_09_content_bad_indent() {
        get_parser_output(
            item_parser(element::element_in_item_parser::<()>()),
            r##"- foo
bar
"##,
        );
    }

    #[test]
    fn test_item_10_content_good_indent() {
        assert_eq!(
            get_parser_output(
                item_parser(element::element_in_item_parser::<()>()),
                r##"- foo
 bar
"##
            ),
            r##"ListItem@0..11
  ListItemIndent@0..0
  ListItemBullet@0..2
    Text@0..1 "-"
    Whitespace@1..2 " "
  ListItemContent@2..11
    Paragraph@2..11
      Text@2..11 "foo\n bar\n"
"##
        );
    }

    #[test]
    fn test_item_11() {
        assert_eq!(
            get_parser_output(
                item_parser(element::element_in_item_parser::<()>()),
                r##"- * not heading"##
            ),
            r##"ListItem@0..15
  ListItemIndent@0..0
  ListItemBullet@0..2
    Text@0..1 "-"
    Whitespace@1..2 " "
  ListItemContent@2..15
    List@2..15
      ListItem@2..15
        ListItemIndent@2..2
        ListItemBullet@2..4
          Text@2..3 "*"
          Whitespace@3..4 " "
        ListItemContent@4..15
          Paragraph@4..15
            Text@4..15 "not heading"
"##
        );
    }

    #[test]
    fn test_item_12() {
        assert_eq!(
            get_parser_output(
                item_parser(element::element_in_item_parser::<()>()),
                r##"- item
  |a|b|

  foo bar
"##
            ),
            r##"ListItem@0..26
  ListItemIndent@0..0
  ListItemBullet@0..2
    Text@0..1 "-"
    Whitespace@1..2 " "
  ListItemContent@2..26
    Paragraph@2..7
      Text@2..7 "item\n"
    Table@7..16
      TableStandardRow@7..15
        Whitespace@7..9 "  "
        Pipe@9..10 "|"
        TableCell@10..12
          Text@10..11 "a"
          Pipe@11..12 "|"
        TableCell@12..14
          Text@12..13 "b"
          Pipe@13..14 "|"
        Newline@14..15 "\n"
      BlankLine@15..16 "\n"
    Paragraph@16..26
      Text@16..26 "  foo bar\n"
"##
        );
    }

    #[test]
    fn test_item_99() {
        let input = r##"+ item contents
"##;
        assert_eq!(
            get_parser_output(item_parser(element::element_in_item_parser::<()>()), input),
            r##"ListItem@0..16
  ListItemIndent@0..0
  ListItemBullet@0..2
    Text@0..1 "+"
    Whitespace@1..2 " "
  ListItemContent@2..16
    Paragraph@2..16
      Text@2..16 "item contents\n"
"##,
        );
    }
}
