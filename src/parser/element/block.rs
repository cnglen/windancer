//! Block parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, S2, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::collections::HashSet;
type NT = NodeOrToken<GreenNode, GreenToken>;
type OSK = OrgSyntaxKind;

pub(crate) fn block_begin_row_parser_with_type<'a>(
    block_type: &'a str,
) -> impl Parser<'a, &'a str, NT, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    object::whitespaces()
        .then(object::just_case_insensitive("#+BEGIN_"))
        .then(object::just_case_insensitive(block_type))
        .then(
            object::whitespaces_g1()
                .then(none_of("\n").repeated().collect::<String>())
                .or_not(),
        )
        .then(just("\n"))
        .validate(
            |((((ws, begin), block_type), parameters), nl), e, _emitter| {
                // println!("dbg@validate@begin: type@state={:?}, type@current={}", e.state().block_type, block_type.to_uppercase());
                e.state().block_type = block_type.clone().to_uppercase(); // update state
                (ws, begin, block_type, parameters, nl)
            },
        )
        .map_with(|(ws, begin, block_type, parameters, nl), e| {
            // println!("dbg@map_with: type={:?}", block_type.to_uppercase());
            let mut children = vec![];

            if ws.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &begin,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &block_type.to_uppercase(),
            )));

            // println!("begin_end_row={:?}", block_type);
            e.state().block_type = block_type.clone().to_uppercase(); // update state

            match parameters {
                None => {}
                Some((ws, p)) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws,
                    )));
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &p,
                    )));
                }
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Newline.into(),
                &nl,
            )));

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockBegin.into(), children))
        })
}

pub(crate) fn block_begin_row_parser<'a>()
-> impl Parser<'a, &'a str, NT, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let block_type = any()
        .filter(|c: &char| !c.is_whitespace())
        .repeated()
        .at_least(1)
        .collect::<String>();

    object::whitespaces()
        .then(object::just_case_insensitive("#+BEGIN_"))
        .then(block_type)
        .then(
            object::whitespaces_g1()
                .then(none_of("\n").repeated().collect::<String>())
                .or_not(),
        )
        .then(just("\n"))
        .validate(
            |((((ws, begin), block_type), parameters), nl), e, _emitter| {
                // println!("dbg@validate@begin: type@state={:?}, type@current={}", e.state().block_type, block_type.to_uppercase());
                e.state().block_type = block_type.clone().to_uppercase(); // update state
                (ws, begin, block_type, parameters, nl)
            },
        )
        .map_with(|(ws, begin, block_type, parameters, nl), e| {
            // println!("dbg@map_with: type={:?}", block_type.to_uppercase());
            let mut children = vec![];

            if ws.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &begin,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &block_type.to_uppercase(),
            )));

            // println!("begin_end_row={:?}", block_type);
            e.state().block_type = block_type.clone().to_uppercase(); // update state

            match parameters {
                None => {}
                Some((ws, p)) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws,
                    )));
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &p,
                    )));
                }
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Newline.into(),
                &nl,
            )));

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockBegin.into(), children))
        })
}

pub(crate) fn block_end_row_parser<'a>()
-> impl Parser<'a, &'a str, NT, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    object::whitespaces()
        .then(object::just_case_insensitive("#+END_"))
        .then(
            any()
                .filter(|c: &char| !c.is_whitespace())
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .try_map_with(|((((ws1, end), block_type), ws2), nl), e| {
            // Not using validate, use try_map_with to halt when an error is generated instead of continuing
            // Not using map_with, which is not executed in and_is(block_rend_row_parser().not())
            // println!("dbg@try_map_with@end: type@state={:?}, type@current={}", e.state().block_type, block_type.to_uppercase());
            if e.state().block_type.to_uppercase() != block_type.to_uppercase() {
                // println!("block type mismatched {} != {}", e.state().block_type, block_type);
                // todo: how to display this error?
                Err(Rich::custom(
                    e.span(),
                    &format!(
                        "block type mismatched {} != {}",
                        e.state().block_type,
                        block_type
                    ),
                ))
            } else {
                Ok((ws1, end, block_type, ws2, nl))
            }
        })
        .map(|(ws1, end, btype, ws2, nl)| {
            let mut children = vec![];
            if ws1.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &end,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &btype.to_uppercase(),
            )));
            if ws2.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws2,
                )));
            }
            match nl {
                Some(_nl) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Newline.into(),
                        &_nl,
                    )));
                }
                None => {}
            }
            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockEnd.into(), children))
        })
}

pub(crate) fn block_end_row_parser_with_type<'a>(
    block_type: &'a str,
) -> impl Parser<'a, &'a str, NT, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    object::whitespaces()
        .then(object::just_case_insensitive("#+END_"))
        .then(object::just_case_insensitive(block_type))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .try_map_with(|((((ws1, end), block_type), ws2), nl), e| {
            // Not using validate, use try_map_with to halt when an error is generated instead of continuing
            // Not using map_with, which is not executed in and_is(block_rend_row_parser().not())
            // println!("dbg@try_map_with@end: type@state={:?}, type@current={}", e.state().block_type, block_type.to_uppercase());
            if e.state().block_type.to_uppercase() != block_type.to_uppercase() {
                // println!("block type mismatched {} != {}", e.state().block_type, block_type);
                // todo: how to display this error?
                Err(Rich::custom(
                    e.span(),
                    &format!(
                        "block type mismatched {} != {}",
                        e.state().block_type,
                        block_type
                    ),
                ))
            } else {
                Ok((ws1, end, block_type, ws2, nl))
            }
        })
        .map(|(ws1, end, btype, ws2, nl)| {
            let mut children = vec![];
            if ws1.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &end,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Text.into(),
                &btype.to_uppercase(),
            )));
            if ws2.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws2,
                )));
            }
            match nl {
                Some(_nl) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Newline.into(),
                        &_nl,
                    )));
                }
                None => {}
            }
            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockEnd.into(), children))
        })
}

/// export block
pub(crate) fn export_block_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let data = none_of("\n \t").repeated().at_least(1).collect::<String>();
    let begin_row = object::whitespaces()
        .then(object::just_case_insensitive("#+BEGIN_"))
        .then(object::just_case_insensitive("EXPORT")) // name
        .then(object::whitespaces_g1())
        .then(data)
        .then(object::whitespaces())
        .then(just("\n"))
        .map(|((((((ws1, begin), block_type), ws2), data), ws3), nl)| {
            (ws1, begin, block_type, ws2, data, ws3, nl)
        });

    let end_row = object::whitespaces()
        .then(object::just_case_insensitive("#+END_"))
        .then(object::just_case_insensitive("EXPORT"))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .map(|((((ws1, end), block_type), ws2), nl)| (ws1, end, block_type, ws2, nl));

    // No line may start with #+end_NAME.
    // Lines beginning with an asterisk must be quoted by a comma (,*) and lines beginning with #+ may be quoted by a comma when necessary (#+).
    let content = object::line_parser()
        .and_is(end_row.clone().not()) // No line may start with #+end_NAME.
        .and_is(just("*").not())
        .repeated()
        .collect::<Vec<String>>()
        .map(|s| s.join(""));

    begin_row
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(|(((begin_row, content), end_row), blank_lines), e| {
            let mut children = vec![];

            let block_begin_node = {
                let (ws1, begin, block_type, ws2, data, ws3, nl) = begin_row;
                let mut children = vec![];
                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws1,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &begin,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &block_type.to_uppercase(),
                )));

                if ws2.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws2,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &data,
                )));

                if ws3.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws3,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    &nl,
                )));
                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockBegin.into(), children))
            };
            children.push(block_begin_node);

            if content.len() > 0 {
                let mut c_children = vec![];
                c_children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &content,
                )));
                let node = NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::BlockContent.into(),
                    c_children,
                ));
                children.push(node);
            }

            let block_end_node = {
                let (ws1, end, block_type, ws2, nl) = end_row;

                let mut children = vec![];
                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws1,
                    )));
                }
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &end,
                )));
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &block_type.to_uppercase(),
                )));

                if ws2.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws2,
                    )));
                }
                match nl {
                    Some(_nl) => {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Newline.into(),
                            &_nl,
                        )));
                    }
                    None => {}
                }
                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockEnd.into(), children))
            };
            children.push(block_end_node);

            for bl in blank_lines {
                children.push(NodeOrToken::Token(bl));
            }

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::ExportBlock.into(), children))
        })
}

/// src block
pub(crate) fn src_block_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let language = none_of(" \t\n").repeated().at_least(1).collect::<String>();
    let switch_p1 = just("-l")
        .then(object::whitespaces_g1())
        .then(none_of("\n\" \t").repeated())
        .to_slice();
    let switch_p2 = one_of("+-")
        .then(any().filter(|c: &char| c.is_alphanumeric()))
        .to_slice();
    let switches = switch_p1.or(switch_p2).repeated().to_slice();
    let arguments = none_of("\n").repeated().collect::<String>();
    let begin_row = object::whitespaces()
        .then(object::just_case_insensitive("#+BEGIN_"))
        .then(object::just_case_insensitive("SRC")) // name
        .then(object::whitespaces_g1())
        .then(language)
        .then(object::whitespaces_g1().then(switches).or_not())
        .then(object::whitespaces_g1().then(arguments).or_not())
        .then(just("\n"))
        .map(
            |(
                (
                    (((((ws1, begin), block_type), ws2), language), maybe_ws3_switches),
                    maybe_ws4_arguments,
                ),
                nl,
            )| {
                (
                    ws1,
                    begin,
                    block_type,
                    ws2,
                    language,
                    maybe_ws3_switches,
                    maybe_ws4_arguments,
                    nl,
                )
            },
        );

    let end_row = object::whitespaces()
        .then(object::just_case_insensitive("#+END_"))
        .then(object::just_case_insensitive("SRC"))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .map(|((((ws1, end), block_type), ws2), nl)| (ws1, end, block_type, ws2, nl));

    // No line may start with #+end_NAME.
    // Lines beginning with an asterisk must be quoted by a comma (,*) and lines beginning with #+ may be quoted by a comma when necessary (#+).
    let content = object::line_parser()
        .and_is(end_row.clone().not()) // No line may start with #+end_NAME.
        .and_is(just("*").not())
        .repeated()
        .collect::<Vec<String>>()
        .map(|s| s.join(""));

    begin_row
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(|(((begin_row, content), end_row), blank_lines), e| {
            let mut children = vec![];

            let block_begin_node = {
                let (
                    ws1,
                    begin,
                    block_type,
                    ws2,
                    language,
                    maybe_ws3_switches,
                    maybe_ws4_arguments,
                    nl,
                ) = begin_row;
                let mut children = vec![];
                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws1,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &begin,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &block_type.to_uppercase(),
                )));

                if ws2.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws2,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::SrcBlockLanguage.into(),
                    &language,
                )));

                if let Some((ws3, switches)) = maybe_ws3_switches {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws3,
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::SrcBlockSwitches.into(),
                        switches,
                    )));
                }

                if let Some((ws4, arguments)) = maybe_ws4_arguments {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws4,
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::SrcBlockHeaderArguments.into(),
                        &arguments,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    &nl,
                )));
                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockBegin.into(), children))
            };
            children.push(block_begin_node);

            if content.len() > 0 {
                let mut c_children = vec![];
                c_children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &content,
                )));
                let node = NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::BlockContent.into(),
                    c_children,
                ));
                children.push(node);
            }

            let block_end_node = {
                let (ws1, end, block_type, ws2, nl) = end_row;

                let mut children = vec![];
                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws1,
                    )));
                }
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &end,
                )));
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &block_type.to_uppercase(),
                )));

                if ws2.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws2,
                    )));
                }
                match nl {
                    Some(_nl) => {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Newline.into(),
                            &_nl,
                        )));
                    }
                    None => {}
                }
                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockEnd.into(), children))
            };
            children.push(block_end_node);

            for bl in blank_lines {
                children.push(NodeOrToken::Token(bl));
            }

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::SrcBlock.into(), children))
        })
}

#[derive(Clone, Debug)]
enum CommentOrBlockType {
    Comment,
    Example,
}

pub(crate) fn comment_or_example_block_parser<'a>(
    block_type: CommentOrBlockType,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let block_type_str = match block_type {
        CommentOrBlockType::Comment => "comment",
        CommentOrBlockType::Example => "example",
    };

    let data = none_of("\n").repeated().at_least(1).collect::<String>();
    let begin_row = object::whitespaces()
        .then(object::just_case_insensitive("#+BEGIN_"))
        .then(object::just_case_insensitive(block_type_str)) // name
        .then(object::whitespaces_g1().then(data).or_not())
        .then(object::whitespaces())
        .then(just("\n"))
        .map(
            |(((((ws1, begin), block_type), maybe_ws2_data), ws3), nl)| {
                (ws1, begin, block_type, maybe_ws2_data, ws3, nl)
            },
        );

    let end_row = object::whitespaces()
        .then(object::just_case_insensitive("#+END_"))
        .then(object::just_case_insensitive(block_type_str))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .map(|((((ws1, end), block_type), ws2), nl)| (ws1, end, block_type, ws2, nl));

    let content = object::line_parser()
        .and_is(end_row.clone().not()) // No line may start with #+end_NAME.
        .and_is(just("*").not())
        .repeated()
        .collect::<Vec<String>>()
        .map(|s| s.join(""));

    begin_row
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(move |(((begin_row, content), end_row), blank_lines), _e| {
            let mut children = vec![];

            let block_begin_node = {
                let (ws1, begin, block_type, maybe_ws2_data, ws3, nl) = begin_row;
                let mut children = vec![];
                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws1,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &begin,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &block_type.to_uppercase(),
                )));

                if let Some((ws2, data)) = maybe_ws2_data {
                    if ws2.len() > 0 {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Whitespace.into(),
                            &ws2,
                        )));
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &data,
                    )));
                }

                if ws3.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws3,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    &nl,
                )));
                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockBegin.into(), children))
            };
            children.push(block_begin_node);

            if content.len() > 0 {
                let mut c_children = vec![];
                c_children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &content,
                )));
                let node = NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::BlockContent.into(),
                    c_children,
                ));
                children.push(node);
            }

            let block_end_node = {
                let (ws1, end, block_type, ws2, nl) = end_row;

                let mut children = vec![];
                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws1,
                    )));
                }
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &end,
                )));
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &block_type.to_uppercase(),
                )));

                if ws2.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws2,
                    )));
                }
                match nl {
                    Some(_nl) => {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Newline.into(),
                            &_nl,
                        )));
                    }
                    None => {}
                }
                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockEnd.into(), children))
            };
            children.push(block_end_node);

            for bl in blank_lines {
                children.push(NodeOrToken::Token(bl));
            }

            match block_type {
                CommentOrBlockType::Comment => {
                    NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::CommentBlock.into(), children))
                }
                CommentOrBlockType::Example => {
                    NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::ExampleBlock.into(), children))
                }
            }
        })
}

/// comment block
pub(crate) fn comment_block_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    comment_or_example_block_parser(CommentOrBlockType::Comment)
}

/// example block
pub(crate) fn example_block_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    comment_or_example_block_parser(CommentOrBlockType::Example)
}

/// verse block
pub(crate) fn verse_block_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let data = none_of("\n").repeated().at_least(1).collect::<String>();
    let begin_row = object::whitespaces()
        .then(object::just_case_insensitive("#+BEGIN_"))
        .then(object::just_case_insensitive("verse")) // name
        .then(object::whitespaces_g1().then(data).or_not())
        .then(object::whitespaces())
        .then(just("\n"))
        .map(
            |(((((ws1, begin), block_type), maybe_ws2_data), ws3), nl)| {
                (ws1, begin, block_type, maybe_ws2_data, ws3, nl)
            },
        );

    let end_row = object::whitespaces()
        .then(object::just_case_insensitive("#+END_"))
        .then(object::just_case_insensitive("verse"))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .map(|((((ws1, end), block_type), ws2), nl)| (ws1, end, block_type, ws2, nl));

    let fullset_objects_parser = object::object_parser()
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<NodeOrToken<GreenNode, GreenToken>>>();
    let content_inner = object::line_parser_allow_blank()
        .and_is(end_row.clone().not()) // No line may start with #+end_NAME.
        .and_is(just("*").not())
        .repeated()
        .collect::<Vec<String>>()
        .map(|s| s.join(""));
    let content = fullset_objects_parser.nested_in(content_inner.to_slice());

    begin_row
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(move |(((begin_row, content), end_row), blank_lines), _e| {
            let mut children = vec![];

            let block_begin_node = {
                let (ws1, begin, block_type, maybe_ws2_data, ws3, nl) = begin_row;
                let mut children = vec![];
                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws1,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &begin,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &block_type.to_uppercase(),
                )));

                if let Some((ws2, data)) = maybe_ws2_data {
                    if ws2.len() > 0 {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Whitespace.into(),
                            &ws2,
                        )));
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Text.into(),
                        &data,
                    )));
                }

                if ws3.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws3,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    &nl,
                )));
                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockBegin.into(), children))
            };
            children.push(block_begin_node);

            let mut content_children = vec![];
            for node in content {
                content_children.push(node);
            }
            children.push(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::BlockContent.into(),
                content_children,
            )));

            let block_end_node = {
                let (ws1, end, block_type, ws2, nl) = end_row;

                let mut children = vec![];
                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws1,
                    )));
                }
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &end,
                )));
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &block_type.to_uppercase(),
                )));

                if ws2.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Whitespace.into(),
                        &ws2,
                    )));
                }
                match nl {
                    Some(_nl) => {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OrgSyntaxKind::Newline.into(),
                            &_nl,
                        )));
                    }
                    None => {}
                }
                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockEnd.into(), children))
            };
            children.push(block_end_node);

            for bl in blank_lines {
                children.push(NodeOrToken::Token(bl));
            }

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::VerseBlock.into(), children))
        })
}

pub(crate) fn block_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    block_begin_row_parser()
        .then(
            any()
                .and_is(block_end_row_parser().not())
                .repeated()
                .collect::<String>(),
        )
        .then(block_end_row_parser())
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(|(((begin_row, content), end_row), blank_lines), e| {
            // println!("content={:?}", content);
            let mut children = vec![];
            children.push(begin_row);

            if content.len() > 0 {
                let mut c_children = vec![];

                let lesser_block_type: HashSet<String> =
                    ["EXAMPLE", "VERSE", "SRC", "COMMENT", "EXPORT"]
                        .iter()
                        .map(|s| s.to_string())
                        .collect();

                if lesser_block_type.contains(&e.state().block_type) {
                    let text =
                        NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Text.into(), &content));
                    c_children.push(text);
                } else {
                    let text =
                        NodeOrToken::Token(GreenToken::new(OrgSyntaxKind::Text.into(), &content));
                    let paragraph = NodeOrToken::Node(GreenNode::new(
                        OrgSyntaxKind::Paragraph.into(),
                        vec![text],
                    ));
                    c_children.push(paragraph);
                }

                let node = NodeOrToken::Node(GreenNode::new(
                    OrgSyntaxKind::BlockContent.into(),
                    c_children,
                ));
                children.push(node);
            }

            children.push(end_row);

            for bl in blank_lines {
                children.push(NodeOrToken::Token(bl));
            }

            let block_type = e.state().block_type.clone();
            let kind = match block_type.as_str() {
                // TODO: greater block vs lesser block?
                "CENTER" => OrgSyntaxKind::CenterBlock,
                "QUOTE" => OrgSyntaxKind::QuoteBlock,

                "COMMENT" => OrgSyntaxKind::CommentBlock,
                "EXAMPLE" => OrgSyntaxKind::ExampleBlock,
                "VERSE" => OrgSyntaxKind::VerseBlock,
                "SRC" => OrgSyntaxKind::SrcBlock,
                "EXPORT" => OrgSyntaxKind::ExportBlock,

                _ => OrgSyntaxKind::SpecialBlock,
            };

            e.state().block_type = String::new(); // reset state
            NodeOrToken::Node(GreenNode::new(kind.into(), children))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use crate::parser::object;
    use crate::parser::{ParserState, SyntaxNode};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_export_block_01() {
        assert_eq!(
            get_parser_output(
                export_block_parser(),
                r##"#+BEGIN_export html 
#+END_export
"##
            ),
            r###"ExportBlock@0..34
  BlockBegin@0..21
    Text@0..8 "#+BEGIN_"
    Text@8..14 "EXPORT"
    Whitespace@14..15 " "
    Text@15..19 "html"
    Whitespace@19..20 " "
    Newline@20..21 "\n"
  BlockEnd@21..34
    Text@21..27 "#+END_"
    Text@27..33 "EXPORT"
    Newline@33..34 "\n"
"###
        );
    }

    #[test]
    #[should_panic]
    fn test_export_block_02() {
        get_parser_output(
            export_block_parser(),
            r##"#+BEGIN_export 
#+END_export
"##,
        );
    }

    #[test]
    #[should_panic]
    fn test_export_block_03() {
        get_parser_output(
            export_block_parser(),
            r##"#+BEGIN_export html latex
#+END_export
"##,
        );
    }

    #[test]
    #[should_panic]
    fn test_export_block_04() {
        get_parser_output(
            export_block_parser(),
            r##"#+BEGIN_export html
* head
#+END_export
"##,
        );
    }

    #[test]
    fn test_verse_block_01() {
        assert_eq!(
            get_parser_output(
                verse_block_parser(),
                r##"#+BEGIN_verse

example
#+END_verse
"##
            ),
            r##"VerseBlock@0..35
  BlockBegin@0..14
    Text@0..8 "#+BEGIN_"
    Text@8..13 "VERSE"
    Newline@13..14 "\n"
  BlockContent@14..23
    Text@14..23 "\nexample\n"
  BlockEnd@23..35
    Text@23..29 "#+END_"
    Text@29..34 "VERSE"
    Newline@34..35 "\n"
"##
        );
    }

    #[test]
    fn test_verse_block_02() {
        assert_eq!(
            get_parser_output(
                verse_block_parser(),
                r##"  #+BEGIN_VERSE
     Great clouds   overhead
     Tiny black birds rise and fall
     Snow covers Emacs

       ---AlexSchroeder, =hello=
    #+END_VERSE
"##
            ),
            r##"VerseBlock@0..154
  BlockBegin@0..16
    Whitespace@0..2 "  "
    Text@2..10 "#+BEGIN_"
    Text@10..15 "VERSE"
    Newline@15..16 "\n"
  BlockContent@16..138
    Text@16..130 "     Great clouds   o ..."
    Verbatim@130..137
      Equals@130..131 "="
      Text@131..136 "hello"
      Equals@136..137 "="
    Text@137..138 "\n"
  BlockEnd@138..154
    Whitespace@138..142 "    "
    Text@142..148 "#+END_"
    Text@148..153 "VERSE"
    Newline@153..154 "\n"
"##
        );
    }

    #[test]
    fn test_src_block_01() {
        assert_eq!(
            get_parser_output(
                src_block_parser(),
                r##"#+BEGIN_src rust -l -n :var foo=bar  
fn main() {
}
#+END_src
"##
            ),
            r##"SrcBlock@0..62
  BlockBegin@0..38
    Text@0..8 "#+BEGIN_"
    Text@8..11 "SRC"
    Whitespace@11..12 " "
    SrcBlockLanguage@12..16 "rust"
    Whitespace@16..17 " "
    SrcBlockSwitches@17..22 "-l -n"
    Whitespace@22..23 " "
    SrcBlockHeaderArguments@23..37 ":var foo=bar  "
    Newline@37..38 "\n"
  BlockContent@38..52
    Text@38..52 "fn main() {\n}\n"
  BlockEnd@52..62
    Text@52..58 "#+END_"
    Text@58..61 "SRC"
    Newline@61..62 "\n"
"##
        );
    }

    #[test]
    fn test_block_bad() {
        let input = "#+BEGIN_SRC python
#+END_DRC";
        let mut state = RollbackState(ParserState::default());
        let r = block_parser().parse_with_state(input, &mut state);
        assert_eq!(r.has_errors(), true);

        for e in r.errors() {
            eprintln!("error: {:?}", e);
        }
    }

    #[test]
    fn test_block_src() {
        let input = "#+BEGIN_sRC python
#+END_SrC";
        let mut state = RollbackState(ParserState::default());
        let r = block_parser().parse_with_state(input, &mut state);
        assert_eq!(r.has_output(), true);
        let syntax_tree = SyntaxNode::new_root(r.into_result().unwrap().into_node().expect("xxx"));
        println!("{:#?}", syntax_tree);
        assert_eq!(
            format!("{:#?}", syntax_tree),
            r##"SrcBlock@0..28
  BlockBegin@0..19
    Text@0..8 "#+BEGIN_"
    Text@8..11 "SRC"
    Whitespace@11..12 " "
    Text@12..18 "python"
    Newline@18..19 "\n"
  BlockEnd@19..28
    Text@19..25 "#+END_"
    Text@25..28 "SRC"
"##
        );
    }

    #[test]
    fn test_block_src_full() {
        let mut state = RollbackState(ParserState::default());

        let input = r###"#+BEGIN_sRC python
print("hi");
print("py");
#+END_SrC"###;
        let r = block_parser().parse_with_state(input, &mut state);
        assert_eq!(r.has_output(), true);
        let syntax_tree = SyntaxNode::new_root(r.into_result().unwrap().into_node().expect("xxx"));

        println!("{:#?}", syntax_tree);
        assert_eq!(
            format!("{:#?}", syntax_tree),
            r##"SrcBlock@0..54
  BlockBegin@0..19
    Text@0..8 "#+BEGIN_"
    Text@8..11 "SRC"
    Whitespace@11..12 " "
    Text@12..18 "python"
    Newline@18..19 "\n"
  BlockContent@19..45
    Text@19..45 "print(\"hi\");\nprint(\"p ..."
  BlockEnd@45..54
    Text@45..51 "#+END_"
    Text@51..54 "SRC"
"##
        );
    }

    #[test]
    fn test_block_example() {
        let mut state = RollbackState(ParserState::default());

        let input = "#+BEGIN_example
#+END_examplE";
        let r = block_parser().parse_with_state(input, &mut state);
        assert_eq!(r.has_output(), true);

        let syntax_tree = SyntaxNode::new_root(r.into_result().unwrap().into_node().expect("xxx"));
        println!("{:#?}", syntax_tree);

        assert_eq!(
            format!("{:#?}", syntax_tree),
            r##"ExampleBlock@0..29
  BlockBegin@0..16
    Text@0..8 "#+BEGIN_"
    Text@8..15 "EXAMPLE"
    Newline@15..16 "\n"
  BlockEnd@16..29
    Text@16..22 "#+END_"
    Text@22..29 "EXAMPLE"
"##
        );
    }
}
