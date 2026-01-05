//! plain list parser
use crate::parser::{MyExtra, NT, OSK};
use crate::parser::{element, object};
use chumsky::prelude::*;

// counter <- ([0-9]+ / [a-z])
fn item_counter_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone {
    choice((
        text::int(10),
        any().filter(|c: &char| matches!(c, 'a'..='z')).to_slice(),
    ))
}

// bullet <- ([*-+] / counter) whitespace+
fn item_bullet_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    group((
        choice((
            one_of("*+-").to_slice(),
            item_counter_parser().then(one_of(".)")).to_slice(),
        )),
        object::whitespaces_g1(),
    ))
    .map(|(bullet, whitespaces)| {
        crate::node!(
            OSK::ListItemBullet,
            vec![
                crate::token!(OSK::Text, bullet),
                crate::token!(OSK::Whitespace, whitespaces),
            ]
        )
    })
}

// item_counter_set <- "[@" counter "]"
fn item_counter_set_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    group((
        just("[@"),
        item_counter_parser(),
        just("]"),
        object::whitespaces_g1(),
    ))
    .map(|(leftbracket_at, counter, right_bracket, whitespaces)| {
        crate::node!(
            OSK::ListItemCounter,
            vec![
                crate::token!(OSK::LeftSquareBracket, &leftbracket_at[0..1]),
                crate::token!(OSK::At, &leftbracket_at[1..2]),
                crate::token!(OSK::Text, counter),
                crate::token!(OSK::RightSquareBracket, right_bracket),
                crate::token!(OSK::Whitespace, whitespaces),
            ]
        )
    })
}

// checkbox <- "[" [ -X] "]" whitespace+
fn item_checkbox_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    group((
        just("["),
        just(" ").or(just("-")).or(just("X")),
        just("]"),
        object::whitespaces_g1(),
    ))
    .map(|(left_bracket, check, right_bracket, whitespaces)| {
        crate::node!(
            OSK::ListItemCheckbox,
            vec![
                crate::token!(OSK::LeftSquareBracket, left_bracket),
                crate::token!(OSK::Text, check),
                crate::token!(OSK::RightSquareBracket, right_bracket),
                crate::token!(OSK::Whitespace, whitespaces),
            ]
        )
    })
}

// tag <- !(whitespaces "::" whitespaces) [^CRLF]+ whitespaces+ "::" whitespaces+
fn item_tag_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> {
    group((
        none_of(object::CRLF)
            .and_is(
                object::whitespaces_g1()
                    .then(just("::"))
                    .then(object::whitespaces_g1())
                    .not(),
            )
            .repeated()
            .at_least(1)
            .to_slice(),
        object::whitespaces_g1(),
        just("::"),
        object::whitespaces_g1(),
    ))
    .map(|(tag, whitespaces1, double_colon, whitespaces2)| {
        crate::node!(
            OSK::ListItemTag,
            vec![
                crate::token!(OSK::Text, tag),
                crate::token!(OSK::Whitespace, whitespaces1),
                crate::token!(OSK::Colon2, double_colon),
                crate::token!(OSK::Whitespace, whitespaces2),
            ]
        )
    })
}

// Create ListItemIndent node from `s`
fn create_item_indent_node(s: &str) -> NT {
    crate::node!(
        OSK::ListItemIndent,
        if !s.is_empty() {
            vec![crate::token!(OSK::Whitespace, s)]
        } else {
            vec![]
        }
    )
}

// Create ListItemIndent node from `s`
fn create_item_node<'a, C: 'a + std::default::Default>(
    element_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
    lookahead_only: bool,
    indent: NT,
    bullet: NT,
    maybe_counter_set: Option<NT>,
    maybe_checkbox: Option<NT>,
    maybe_tag: Option<NT>,
    maybe_content: Option<&'a str>,
    maybe_blankline: Option<NT>,
) -> NT {
    let mut children = Vec::with_capacity(7);

    children.push(indent);
    children.push(bullet);
    if let Some(counter_set) = maybe_counter_set {
        children.push(counter_set);
    }
    if let Some(checkbox) = maybe_checkbox {
        children.push(checkbox);
    }
    if let Some(tag) = maybe_tag {
        children.push(tag);
    }

    if let Some(content) = maybe_content {
        let content_node = if lookahead_only {
            crate::node!(
                OSK::ListItemContent,
                vec![crate::token!(OSK::Text, content)]
            )
        } else {
            // Note: use parse() here, if we use nested_in() here, CTX type error
            element_parser
                .repeated()
                .collect::<Vec<_>>()
                .map(|s| crate::node!(OSK::ListItemContent, s))
                .parse(content)
                .into_output()
                .unwrap()
        };
        children.push(content_node);
    }

    if let Some(blankline) = maybe_blankline {
        children.push(blankline);
    }

    crate::node!(OSK::ListItem, children)
}

// PlainList <- AffiliatedKeywords? Item+ BlankLine*
// Item <- whitespaces* Bullet CounterSet? CheckBox? Tag? Contents? blankline?
fn plain_list_parser_inner<'a, C: 'a + std::default::Default>(
    affiliated_keywords_parser: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
    element_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
    lookahead_only: bool,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let item_content_inner = object::line_parser() // no need to test indent since this is the first row of item
        .then(
            object::line_parser()
                .and_is(
                    just(" ")
                        .repeated()
                        .configure(|cfg, ctx: &&str| cfg.at_least((*ctx).len() + 1)),
                )
                .or(object::blank_line_str_parser())
                .and_is(
                    // two consecutive blank lines
                    object::blank_line_parser().repeated().at_least(2).not(),
                )
                .repeated(),
        )
        .to_slice();

    affiliated_keywords_parser
        .then(
            object::whitespaces().then_with_ctx(
                group((
                    // fist item without preceding whitespaces
                    item_bullet_parser(),
                    item_counter_set_parser().or_not(),
                    item_checkbox_parser().or_not(),
                    item_tag_parser().or_not(),
                    item_content_inner.clone().or_not(),
                    object::blank_line_parser().or_not(),
                ))
                .then(
                    // other items
                    group((
                        just(" ")
                            .repeated()
                            .configure(|cfg, ctx: &&str| cfg.exactly((*ctx).len()))
                            .to_slice()
                            .map(create_item_indent_node),
                        item_bullet_parser(),
                        item_counter_set_parser().or_not(),
                        item_checkbox_parser().or_not(),
                        item_tag_parser().or_not(),
                        item_content_inner.or_not(),
                        object::blank_line_parser().or_not(),
                    ))
                    .repeated()
                    .collect::<Vec<_>>(),
                ),
            ),
        )
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(move |((keywords, items), blanklines)| {
            let (
                indent,
                (
                    (
                        bullets,
                        maybe_counter_set,
                        maybe_checkbox,
                        maybe_tag,
                        maybe_content,
                        maybe_blankline,
                    ),
                    other_items,
                ),
            ) = items;

            let mut children =
                Vec::with_capacity(keywords.len() + other_items.len() + blanklines.len() + 1);

            children.extend(keywords);
            let first_item = create_item_node(
                element_parser.clone(),
                lookahead_only,
                create_item_indent_node(indent),
                bullets,
                maybe_counter_set,
                maybe_checkbox,
                maybe_tag,
                maybe_content,
                maybe_blankline,
            );
            children.push(first_item);

            for (
                indent,
                bullets,
                maybe_counter_set,
                maybe_checkbox,
                maybe_tag,
                maybe_content,
                maybe_blankline,
            ) in other_items
            {
                let item = create_item_node(
                    element_parser.clone(),
                    lookahead_only,
                    indent,
                    bullets,
                    maybe_counter_set,
                    maybe_checkbox,
                    maybe_tag,
                    maybe_content,
                    maybe_blankline,
                );
                children.push(item);
            }
            children.extend(blanklines);

            crate::node!(OSK::List, children)
        })
        .boxed()
}

pub(crate) fn plain_list_parser<'a, C: 'a + std::default::Default>(
    element_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let affiliated_keywords_parser = element::keyword::affiliated_keyword_parser()
        .repeated()
        .collect::<Vec<_>>();

    plain_list_parser_inner(affiliated_keywords_parser, element_parser, false)
}

pub(crate) fn simple_plain_list_parser<'a, C: 'a + std::default::Default>()
-> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    let affiliated_keywords_parser = element::keyword::simple_affiliated_keyword_parser()
        .repeated()
        .collect::<Vec<_>>();

    let faded_parser = empty().to(crate::token!(OSK::Text, ""));

    plain_list_parser_inner(affiliated_keywords_parser, faded_parser, true).ignored()
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parser_output;
    use crate::parser::element;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_list_01() {
        let input = r##"- one
+ two
- three
- four
"##;

        let expected_output = r##"List@0..27
  ListItem@0..6
    ListItemIndent@0..0
    ListItemBullet@0..2
      Text@0..1 "-"
      Whitespace@1..2 " "
    ListItemContent@2..6
      Paragraph@2..6
        Text@2..6 "one\n"
  ListItem@6..12
    ListItemIndent@6..6
    ListItemBullet@6..8
      Text@6..7 "+"
      Whitespace@7..8 " "
    ListItemContent@8..12
      Paragraph@8..12
        Text@8..12 "two\n"
  ListItem@12..20
    ListItemIndent@12..12
    ListItemBullet@12..14
      Text@12..13 "-"
      Whitespace@13..14 " "
    ListItemContent@14..20
      Paragraph@14..20
        Text@14..20 "three\n"
  ListItem@20..27
    ListItemIndent@20..20
    ListItemBullet@20..22
      Text@20..21 "-"
      Whitespace@21..22 " "
    ListItemContent@22..27
      Paragraph@22..27
        Text@22..27 "four\n"
"##;

        let list_parser = element::plain_list::plain_list_parser(element::element_parser::<&str>());
        assert_eq!(
            get_parser_output::<&str>(list_parser, input),
            expected_output
        );
    }

    #[test]
    fn test_list_02() {
        let input = r##"- one
- two

- three

- four
"##;
        let expected_output = r##"List@0..29
  ListItem@0..6
    ListItemIndent@0..0
    ListItemBullet@0..2
      Text@0..1 "-"
      Whitespace@1..2 " "
    ListItemContent@2..6
      Paragraph@2..6
        Text@2..6 "one\n"
  ListItem@6..13
    ListItemIndent@6..6
    ListItemBullet@6..8
      Text@6..7 "-"
      Whitespace@7..8 " "
    ListItemContent@8..13
      Paragraph@8..13
        Text@8..12 "two\n"
        BlankLine@12..13 "\n"
  ListItem@13..22
    ListItemIndent@13..13
    ListItemBullet@13..15
      Text@13..14 "-"
      Whitespace@14..15 " "
    ListItemContent@15..22
      Paragraph@15..22
        Text@15..21 "three\n"
        BlankLine@21..22 "\n"
  ListItem@22..29
    ListItemIndent@22..22
    ListItemBullet@22..24
      Text@22..23 "-"
      Whitespace@23..24 " "
    ListItemContent@24..29
      Paragraph@24..29
        Text@24..29 "four\n"
"##;
        let list_parser = element::plain_list::plain_list_parser(element::element_parser::<&str>());
        assert_eq!(
            get_parser_output::<&str>(list_parser, input),
            expected_output
        );
    }

    #[test]
    #[should_panic]
    fn test_list_03() {
        let input = r##"- one
- two


- One again
- Two again
"##;
        let list_parser = element::plain_list::plain_list_parser(element::element_parser::<&str>());
        get_parser_output::<&str>(list_parser, input);
    }

    #[test]
    fn test_list_04() {
        let input = r##"- 1
  - 1.1
    a
         b
             c



"##;

        let expected_output = r##"List@0..47
  ListItem@0..45
    ListItemIndent@0..0
    ListItemBullet@0..2
      Text@0..1 "-"
      Whitespace@1..2 " "
    ListItemContent@2..44
      Paragraph@2..4
        Text@2..4 "1\n"
      List@4..44
        ListItem@4..44
          ListItemIndent@4..6
            Whitespace@4..6 "  "
          ListItemBullet@6..8
            Text@6..7 "-"
            Whitespace@7..8 " "
          ListItemContent@8..44
            Paragraph@8..44
              Text@8..44 "1.1\n    a\n         b\n ..."
    BlankLine@44..45 "\n"
  BlankLine@45..46 "\n"
  BlankLine@46..47 "\n"
"##;
        let list_parser = element::plain_list::plain_list_parser(element::element_parser::<&str>());
        assert_eq!(
            get_parser_output::<&str>(list_parser, input),
            expected_output
        );
    }

    #[test]
    fn test_list_05() {
        let input = r##"- one
     - two"##;
        let list_parser = element::plain_list::plain_list_parser(element::element_parser::<&str>());
        assert_eq!(
            get_parser_output::<&str>(list_parser, input),
            r##"List@0..16
  ListItem@0..16
    ListItemIndent@0..0
    ListItemBullet@0..2
      Text@0..1 "-"
      Whitespace@1..2 " "
    ListItemContent@2..16
      Paragraph@2..6
        Text@2..6 "one\n"
      List@6..16
        ListItem@6..16
          ListItemIndent@6..11
            Whitespace@6..11 "     "
          ListItemBullet@11..13
            Text@11..12 "-"
            Whitespace@12..13 " "
          ListItemContent@13..16
            Paragraph@13..16
              Text@13..16 "two"
"##
        );
    }

    #[test]
    fn test_list_06() {
        let input = r##" - one
    - two
    "##;
        let list_parser = element::plain_list::plain_list_parser(element::element_parser::<&str>());
        assert_eq!(
            get_parser_output::<&str>(list_parser, input),
            r#"List@0..21
  ListItem@0..21
    ListItemIndent@0..1
      Whitespace@0..1 " "
    ListItemBullet@1..3
      Text@1..2 "-"
      Whitespace@2..3 " "
    ListItemContent@3..21
      Paragraph@3..7
        Text@3..7 "one\n"
      List@7..17
        ListItem@7..17
          ListItemIndent@7..11
            Whitespace@7..11 "    "
          ListItemBullet@11..13
            Text@11..12 "-"
            Whitespace@12..13 " "
          ListItemContent@13..17
            Paragraph@13..17
              Text@13..17 "two\n"
      Paragraph@17..21
        Text@17..21 "    "
"#
        );
    }

    #[test]
    fn test_list_07() {
        let input = r##"#+caption: affiliated keywords in list
- one
- two
    "##;
        let list_parser = element::plain_list::plain_list_parser(element::element_parser::<&str>());
        assert_eq!(
            get_parser_output::<&str>(list_parser, input),
            r##"List@0..55
  AffiliatedKeyword@0..39
    HashPlus@0..2 "#+"
    KeywordKey@2..9
      Text@2..9 "caption"
    Colon@9..10 ":"
    Whitespace@10..11 " "
    KeywordValue@11..38
      Text@11..38 "affiliated keywords i ..."
    Newline@38..39 "\n"
  ListItem@39..45
    ListItemIndent@39..39
    ListItemBullet@39..41
      Text@39..40 "-"
      Whitespace@40..41 " "
    ListItemContent@41..45
      Paragraph@41..45
        Text@41..45 "one\n"
  ListItem@45..55
    ListItemIndent@45..45
    ListItemBullet@45..47
      Text@45..46 "-"
      Whitespace@46..47 " "
    ListItemContent@47..55
      Paragraph@47..55
        Text@47..55 "two\n    "
"##
        );
    }
}
