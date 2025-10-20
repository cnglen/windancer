//! Section parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserResult, ParserState, element, list, object};
use chumsky::inspector::SimpleState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;

use crate::parser::paragraph::simple_heading_row_parser;

/// Section解析器，返回包含`GreenNode`的ParserResult
///
/// 实现要点:
/// - 结尾满足下面条件之一:
///   - \n + HeadingRow: 避免把`This is a * faked_title`部分识别为HeadingRow
///   - end()
///     - \n + end()
///     - end()
/// - 开头不能以`* Text`开头, 否则部分标题会被识别为Section

// block_parser
// blank_line
// other_parser
// S2? 是否合适?
pub(crate) fn section_parser<'a>()
-> impl Parser<'a, &'a str, ParserResult, extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>>
+ Clone {
    // elements: children
    // 每个element实现时可通过前缀快速终止
    let list_parser = list::create_list_item_content_parser().0;
    list_parser
        .or(element::element_parser())
        // element::element_parser()
        .and_is(simple_heading_row_parser().then(any().repeated()).not()) // Section不能以<* title>开头，避免HeadingSurbtree被识别为Section
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .labelled("section parse")
        // .map_with(|(s, nl), e| {
        .map_with(|children, e| {
            let span: SimpleSpan = e.span();

            ParserResult {
                green: NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Section.into(), children)),
                text: "todo".to_string(),
                span: Range {
                    start: span.start,
                    end: span.end,
                },
            }
        })
}

pub(crate) fn section_unknown_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    none_of("*")
        .then(
            any()
                .and_is(just('\n').then(simple_heading_row_parser()).not().or(end())) // Section的结尾是\n+HeadingRow或End(), 其中\n会被后续的子parser消费掉
                .repeated()
                .collect::<String>(),
        )
        .then(object::newline_or_ending()) // section的最后必须是\n或end() //? \n/end已经被上一个element消费掉！！
        .map(|((c, s), nl)| {
            println!("section_unknown_parser: first_character={:#?}, other_character={:#?}, maybe_newline={:#?}", c, s, nl);
            let mut text = String::new();
            text.push(c);
            text.push_str(&s);

            match nl {
                Some(_nl) => {
                    text.push_str(&_nl);
                }
                None => {}
            }
            let token = NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Text.into(), &text));
            let node = NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::SectionUnknown.into(),
                vec![token],
            ));

            println!("  text={:#?}", text);
            node
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_section_end_with_heading() {
        let input = "section content
* heading";
        let mut state = SimpleState(ParserState::default());
        assert_eq!(
            section_parser()
                .parse_with_state(input, &mut state)
                .has_errors(),
            true
        );
    }

    //     #[test]
    //     fn test_section_with_end() {
    //         let input = "0123456789";
    //         let mut state = SimpleState(ParserState::default());
    //         let s = section_parser()
    //             .parse_with_state(input, &mut state)
    //             .into_result()
    //             .unwrap()
    //             .syntax();
    //         println!("{:#?}", s);
    //         assert_eq!(
    //             format!("{:#?}", s),
    //             r##"Section@0..10
    //   Paragraph@0..10
    //     Text@0..10 "0123456789"
    // "##
    //         );
    //     }

    #[test]
    fn test_section_with_newline_end() {
        let input = "0123456789\n";
        let mut state = SimpleState(ParserState::default());
        let s = section_parser()
            .parse_with_state(input, &mut state)
            .into_result()
            .unwrap()
            .syntax();
        println!("{}", format!("{:#?}", s));
        assert_eq!(
            format!("{:#?}", s),
            r##"Section@0..11
  Paragraph@0..11
    Text@0..11 "0123456789\n"
"##
        );
    }

    #[test]
    fn test_section_fakedtitle() {
        let input = "0123456789 * faked_title";
        let mut state = SimpleState(ParserState::default());
        let result = section_parser().parse_with_state(input, &mut state);

        // for e in result.errors() {
        //     println!("error={:?}", e);
        // }

        //         let s = result
        //             .into_result()
        //             .unwrap()
        //             .syntax();

        //         println!("syntax_tree:{}", format!("{:#?}", s));
        //         assert_eq!(
        //             format!("{:#?}", s),
        //             r##"Section@0..24
        //   Paragraph@0..24
        //     Text@0..24 "0123456789 * faked_title"
        // "##
        //         );
    }

    #[test]
    fn test_section_vs_heading_subtree() {
        let input = "* title\n asf\n";
        let mut state = SimpleState(ParserState::default());

        assert!(
            section_parser()
                .parse_with_state(input, &mut state)
                .has_errors()
        );

        let parser = simple_heading_row_parser().then(any().repeated());
        assert_eq!(
            parser
                .parse_with_state(input, &mut SimpleState(ParserState::default()))
                .has_errors(),
            false
        );
    }
}
