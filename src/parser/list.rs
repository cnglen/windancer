//! List parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, element, object};
use chumsky::inspector::SimpleState;

use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::ops::Range;

/// Construct List/ListItem/ListItemContent parser
///
/// Note:
///
/// List/ListItem/ListItemContent are Recursive Parsers, Since:
/// - List -> ListItem -> ListItemContent -> List/Other_Element -> ListItem -> ListItemContent ...
pub(crate) fn create_list_item_content_parser<'a>() -> (
    impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
    > + Clone,
    impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
    > + Clone,
    impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
    > + Clone,
) {
    let mut list_parser = Recursive::declare();
    let mut list_item_parser = Recursive::declare();
    let mut item_content_parser = Recursive::declare();

    // find the string of item_content, which is terminated by two blankline or lesser indent first line
    let item_content_inner = object::line_parser()
        .or(object::blank_line_str_parser())
        .and_is(object::blank_line_parser().repeated().at_least(2).not())
        .and_is(lesser_indent_termination().not()) // 覆盖了： next item的结束条件(next_item: 属于lesser_indent)
        .repeated()
        .collect::<Vec<String>>()
        .map(|s| s.join(""))
        .to_slice();

    item_content_parser.define(
        // item content: 注意不从行首开始, 先消费第一行
        any()
            .filter(|c: &char| *c != '\n')
            .repeated()
            .collect::<String>()
            .then(just("\n"))
            .map(|(first_row, nl)| {
                // println!("item content parser(firstline): first={:?}, nl={:?}", first_row, nl);
                let mut first_line = String::new();
                first_line.push_str(&first_row);
                first_line.push_str(nl);

                NodeOrToken::<GreenNode, GreenToken>::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &first_line,
                ))
            })
            .then(
                // FIXME: element_parser doesn't include recursive parser
                list_parser
                    .clone() // element.rs: NOT include list_item
                    .or(element::element_parser())
                    .repeated()
                    .collect::<Vec<_>>()
                    .nested_in(item_content_inner),
            )
            .map(|(first, other_children)| {
                let mut children = vec![];

                let paragraph =
                    NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Paragraph.into(), vec![first]));

                children.push(paragraph);

                for c in other_children {
                    children.push(c);
                }

                NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::ListItemContent.into(),
                    children,
                ))
            }),
    );

    list_item_parser.define(
        item_indent_parser()
            .then(item_bullet_parser())
            .then(item_counter_parser().or_not())
            .then(item_checkbox_parser().or_not())
            .then(item_tag_parser().or_not())
            .validate(|((((indent, bullet), maybe_counter), maybe_checkbox), maybe_tag), e, emitter|{
                // update item_indent state: push
                let current_indent = usize::from(indent.text_len());
                let state_indent_length = e.state().item_indent.len();

                if state_indent_length > 0 {
                    let last_state = e.state().item_indent[state_indent_length - 1];
                    if current_indent < last_state {
                        let error = Rich::custom::<&str>(
                            SimpleSpan::from(Range {
                                start: e.span().start(),
                                end: e.span().end(),
                            }),
                            &format!("item_indent_parser: 缩进不足 current_indent({current_indent}) < state_indent({last_state})"),
                        );
                        emitter.emit(error)
                    } else if current_indent > last_state {
                        // println!("item_indent_parser0:before push ({}>{})@ state={:?}", current_indent, last_state, e.state().item_indent); 
                        e.state().item_indent.push(current_indent); // first item of Non-First list in doc
                        // println!("item_indent_parser0:after  push @ state={:?}", e.state().item_indent); 
                    } else {
                    }
                } else { // 仅当是“任意一个List的第一个item”时才更新state: push
                    // println!("item_indent_parser1:before push @ state={:?}", e.state().item_indent);
                    e.state().item_indent.push(current_indent); // update state
                    // println!("item_indent_parser1:after  push @ state={:?}", e.state().item_indent);                
                }
                ((((indent, bullet), maybe_counter), maybe_checkbox), maybe_tag)
            })
            .then(item_content_parser.clone().or_not())
            .then(object::blank_line_parser().repeated().at_most(1).collect::<Vec<_>>())
            .try_map_with(
                |(
                    (((((indent, bullet), maybe_counter), maybe_checkbox), maybe_tag), maybe_content,),
                    blanklines,
                ), _e| {
                    let mut children = vec![];

                    children.push(indent);
                    children.push(bullet);

                    match maybe_counter {
                        Some(counter) => {
                            children.push(counter);
                        }
                        None => {}
                    }

                    match maybe_checkbox {
                        Some(checkbox) => {
                            children.push(checkbox);
                        }
                        None => {}
                    }

                    match maybe_tag {
                        Some(tag) => {
                            children.push(tag);
                        }
                        None => {}
                    }

                    match maybe_content {
                        Some(content) => {
                            children.push(content);
                        }
                        None => {}
                    }

                    for blankline in blanklines {
                        children.push(NodeOrToken::Token(blankline));
                    }

                    Ok(
                        NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                            OrgSyntaxKind::ListItem.into(),
                            children,
                        ))
                    )
                },
            ),
    );

    list_parser.define(
        list_item_parser
            .clone()
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
            }),
    );

    (list_parser, list_item_parser, item_content_parser)
}

/// Item Indent Parser
///
/// 功能: 检测whistespace的个数，构造ListItemIndent Node
///
/// 注意:
///   - 仅当是“任意一个List的第一个item”时才更新state["item_indent"]: push
///   - ItemIndent状态不能在这里更新，避免任意一行content的数据，更新状态，导致状态混乱
///   - 保证ItemIndent状态在item_content前更新
pub(crate) fn item_indent_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    object::whitespaces().map(|ws| {
        let mut children = vec![];
        if ws.len() > 0 {
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Whitespace.into(),
                &ws,
            )));
        }
        NodeOrToken::Node(GreenNode::new(
            OrgSyntaxKind::ListItemIndent.into(),
            children,
        ))
    })
}

/// Item Bullet Parser
pub(crate) fn item_bullet_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    just("*")
        .to(String::from("*"))
        .or(just("-").to(String::from("-")))
        .or(just("+").to(String::from("+")))
        .or(text::int(10)
            .then(just(".").or(just(")")))
            .map(|(num, pq)| format!("{}{}", num, pq)))
        .then(object::whitespaces_g1())
        .map(|(bullet, ws)| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &bullet,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Whitespace.into(),
                &ws,
            )));

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::ListItemBullet.into(),
                children,
            ))
        })
}

/// Item Counter Parser
pub(crate) fn item_counter_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    just("[@")
        .then(text::int(10))
        .then(just("]"))
        .then(object::whitespaces_g1())
        .map(|(((_lbracket_at, number), rbracket), ws)| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                "[",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::At.into(),
                "@",
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                number,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Whitespace.into(),
                &ws,
            )));

            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::ListItemCounter.into(),
                children,
            ))
        })
}

/// Item Checkbox Parser
pub(crate) fn item_checkbox_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    just("[")
        .then(just(" ").or(just("-")).or(just("X")))
        .then(just("]"))
        .then(object::whitespaces_g1())
        .map(|(((lbracket, check), rbracket), ws)| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::LeftSquareBracket.into(),
                lbracket,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::At.into(),
                check,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::RightSquareBracket.into(),
                rbracket,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Whitespace.into(),
                &ws,
            )));

            NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::ListItemCheckbox.into(),
                children,
            ))
        })
}

/// Item Tag Parser
pub(crate) fn item_tag_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>,
> + Clone {
    any()
        .filter(|c: &char| *c != '\n')
        .and_is(
            object::whitespaces_g1()
                .then(just("::"))
                .then(object::whitespaces_g1())
                .not(),
        )
        .repeated()
        .at_least(1)
        .collect::<String>()
        .then(object::whitespaces_g1())
        .then(just("::"))
        .then(object::whitespaces_g1())
        .map(|(((tag, ws1), double_colon), ws2)| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &tag,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Whitespace.into(),
                &ws1,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon2.into(),
                double_colon,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Whitespace.into(),
                &ws2,
            )));

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::ListItemTag.into(), children))
        })
}

/// 仅用于前瞻否定判定，不用于实际解析
// - CONTENTS (optional) :: A collection of zero or more elements, ending at the first instance of one of the following:
//   - The next item.
//   - The first line less or equally indented than the starting line, not counting lines within other non-paragraph elements or inlinetask boundaries.
//   - Two consecutive blank lines.
fn lesser_indent_termination<'a>()
-> impl Parser<'a, &'a str, (), extra::Full<Rich<'a, char>, SimpleState<ParserState>, ()>> + Clone {
    // todo: not counting non-paragraph elements or inline task boudaries
    object::whitespaces()
        .try_map_with(|ws, e| {
            // .validate(|ws, e, emitter| {
            let current_indent = ws.len();
            let state_indent_length = e.state().item_indent.len(); // 仅在item_content时调用，必然len>0
            let last_state = e.state().item_indent[state_indent_length - 1];
            if current_indent > last_state {
                // println!("lesser_indent_termination: ws=|{ws}|, error 缩进不足 current_indent({current_indent}) < state_indent({last_state})");
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
                // println!("lesser_indent_termination: ws=|{ws}|, ok current_indent({current_indent}) < state_indent({last_state})");
                Ok(ws)
            }
        })
        .ignored()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SyntaxNode;

    #[test]
    fn test_list_item_content() {
        let inputs = vec!["f\n"];

        for (i, input) in inputs.iter().enumerate() {
            let mut state = SimpleState(ParserState::default());
            let (_, _list_item_parser, list_item_content_parser) =
                create_list_item_content_parser();
            let t = list_item_content_parser.parse_with_state(input, &mut state);

            println!("input=|{:?}|", input);
            for e in t.errors() {
                println!("error={:?}", e);
            }

            assert_eq!(t.has_output(), true);

            let syntax_tree =
                SyntaxNode::new_root(t.into_result().unwrap().into_node().expect("xxx"));
            println!("\n\n{i}\ninput=\n{input}\ntree=\n{:#?}", syntax_tree);
        }
    }

    #[test]
    fn test_item_basic() {
        let inputs = vec![
            r##"+ [@3] [X] tag :: item contents
"##,
            r##"+ [X] tag :: item contents
"##,
            r##"+ [@3] tag :: item contents
"##,
            r##"+ [@3] [X] item contents
"##,
            r##"+ [@3] [X] tag :: item contents
"##,
            r##"+ 
"##,
            r##"+ foo
"##,
            r##"   + [@3] [X] tag :: item contents
"##,
        ];

        for (i, input) in inputs.iter().enumerate() {
            let mut state = SimpleState(ParserState::default());
            let (_, list_item_parser, _list_item_content_parser) =
                create_list_item_content_parser();
            let t = list_item_parser.parse_with_state(input, &mut state);

            // println!("input={:?}", input);
            for e in t.errors() {
                println!("error={:?}", e);
            }

            assert_eq!(t.has_output(), true);

            let syntax_tree =
                SyntaxNode::new_root(t.into_result().unwrap().into_node().expect("xxx"));
            println!("\n\n{i}\ninput=\n{input}\ntree=\n{:#?}", syntax_tree);
        }
    }

    #[test]
    fn test_list_4_ok() {
        let inputs = vec![
            r##"- one
+ two
- three
- four
"##,
        ];

        println!("test_list_4_ok\n");
        for (i, input) in inputs.iter().enumerate() {
            println!("input_{:02}={}", i, input);
            let mut state = SimpleState(ParserState::default());
            let (list_parser, _list_item_parser, _) = create_list_item_content_parser();
            let t = list_parser.parse_with_state(input, &mut state);

            for (i, e) in t.clone().errors().enumerate() {
                println!("error_{:02}={:?}", i, e);
            }

            assert_eq!(t.has_output(), true);
            let syntax_tree =
                SyntaxNode::new_root(t.clone().into_result().unwrap().into_node().expect("xxx"));

            println!("final state: item_indent={:?}", state.item_indent);

            println!("syntax_tree=\n{:#?}", syntax_tree);
        }
    }

    #[test]
    fn test_list_4_blankline_ok() {
        let inputs = vec![
            r##"- one
- two

- three

- four
"##,
        ];

        println!("test_list_4_ok\n");
        for (i, input) in inputs.iter().enumerate() {
            println!("input_{:02}={}", i, input);
            let mut state = SimpleState(ParserState::default());
            let (list_parser, _list_item_parser, _) = create_list_item_content_parser();
            let t = list_parser.parse_with_state(input, &mut state);

            for (i, e) in t.clone().errors().enumerate() {
                println!("error_{:02}={:?}", i, e);
            }

            assert_eq!(t.has_output(), true);
            let syntax_tree =
                SyntaxNode::new_root(t.clone().into_result().unwrap().into_node().expect("xxx"));

            println!("syntax_tree=\n{:#?}", syntax_tree);
        }
    }

    #[test]
    fn test_list_deeper_ok() {
        let inputs = vec![
            r##"- 1
  - 1.1
    a
         b
             c



"##,
        ];

        for (i, input) in inputs.iter().enumerate() {
            let mut state = SimpleState(ParserState::default());
            let (list_parser, _list_item_parser, _) = create_list_item_content_parser();
            let t = list_parser.parse_with_state(input, &mut state);

            for e in t.errors() {
                println!("error={:?}", e);
            }

            assert_eq!(t.has_output(), true);
            println!("final state: item_indent={:?}", state.item_indent);

            let syntax_tree =
                SyntaxNode::new_root(t.into_result().unwrap().into_node().expect("xxx"));
            println!("\n\n{i}\ninput=\n{input}\ntree=\n{:#?}", syntax_tree);
        }
    }

    #[test]
    fn test_list_two_lists_bad() {
        let inputs = vec![
            r##"- one
- two


- One again
- Two again
"##,
        ];

        for (_i, input) in inputs.iter().enumerate() {
            let mut state = SimpleState(ParserState::default());
            let (list_parser, _list_item_parser, _) = create_list_item_content_parser();
            let t = list_parser.parse_with_state(input, &mut state);

            for e in t.errors() {
                println!("test_list_two_lists_bad(): error={:?}", e);
            }

            assert_eq!(t.has_errors(), true);
        }
    }

    #[test]
    fn test_list_bad() {
        let inputs = vec![
            r##"- one
 - two"##,
            r##" - one
- two
"##,
        ];

        for (i, input) in inputs.iter().enumerate() {
            let mut state = SimpleState(ParserState::default());
            let (list_parser, _list_item_parser, _) = create_list_item_content_parser();
            let t = list_parser.parse_with_state(input, &mut state);
            for e in t.errors() {
                println!("test_list_bad(): error={:?}", e);
            }
            assert_eq!(t.has_errors(), true);
        }
    }
}
