//! List parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, element, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn plain_list_parser<'a>(
    item_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone
    + 'a,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    item_parser
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(|(items, blanklines), e| {
            let mut children = vec![];

            for item in items.clone() {
                children.push(item);
            }

            for bl in blanklines {
                children.push(NodeOrToken::Token(bl));
            }

            e.state().item_indent.pop();

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::List.into(),
                children,
            ))
        })
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

        let list_parser = element::list::plain_list_parser(element::item::item_parser(
            element::element_parser(),
        ));
        assert_eq!(get_parser_output(list_parser, input), expected_output);
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
    ListItemContent@8..12
      Paragraph@8..12
        Text@8..12 "two\n"
    BlankLine@12..13 "\n"
  ListItem@13..22
    ListItemIndent@13..13
    ListItemBullet@13..15
      Text@13..14 "-"
      Whitespace@14..15 " "
    ListItemContent@15..21
      Paragraph@15..21
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
        let list_parser = element::list::plain_list_parser(element::item::item_parser(
            element::element_parser(),
        ));
        assert_eq!(get_parser_output(list_parser, input), expected_output);
    }

    #[test]
    #[should_panic]
    fn test_list_03() {
        let input = r##"- one
- two


- One again
- Two again
"##;
        let list_parser = element::list::plain_list_parser(element::item::item_parser(
            element::element_parser(),
        ));
        get_parser_output(list_parser, input);
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
            Paragraph@8..12
              Text@8..12 "1.1\n"
            Paragraph@12..44
              Text@12..44 "    a\n         b\n     ..."
    BlankLine@44..45 "\n"
  BlankLine@45..46 "\n"
  BlankLine@46..47 "\n"
"##;
        let list_parser = element::list::plain_list_parser(element::item::item_parser(
            element::element_parser(),
        ));
        assert_eq!(get_parser_output(list_parser, input), expected_output);
    }

    #[test]
    fn test_list_05() {
        let input = r##"- one
     - two"##;
        let list_parser = element::list::plain_list_parser(element::item::item_parser(
            element::element_parser(),
        ));
        assert_eq!(get_parser_output(list_parser, input), r##"List@0..16
  ListItem@0..16
    ListItemIndent@0..0
    ListItemBullet@0..2
      Text@0..1 "-"
      Whitespace@1..2 " "
    ListItemContent@2..16
      Paragraph@2..6
        Text@2..6 "one\n"
      List@6..13
        ListItem@6..13
          ListItemIndent@6..11
            Whitespace@6..11 "     "
          ListItemBullet@11..13
            Text@11..12 "-"
            Whitespace@12..13 " "
      Paragraph@13..16
        Text@13..16 "two"
"##);
    }

    #[test]
    fn test_list_06() {
        let input = r##" - one
    - two
    "##;
        let list_parser = element::list::plain_list_parser(element::item::item_parser(
            element::element_parser(),
        ));
        assert_eq!(get_parser_output(list_parser, input), r#"List@0..21
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
"#);
    }

    //     // FIXME:
    //     #[test]
    //     #[should_panic]
    //     fn test_list_07() {
    //         let input = r##"- one list
    // 1. another list
    // "##;
    //         let list_parser = element::list::plain_list_parser(element::item::item_parser(
    //             element::element_parser(),
    //         ));
    //         get_parser_output(list_parser, input);
    //     }
}
