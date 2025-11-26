//! Block parser
// todo: reduce code; just::configure from another parser?, don't use state
// can nested self:? center center
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, element, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

type NT = NodeOrToken<GreenNode, GreenToken>;
type OSK = OrgSyntaxKind;

#[derive(Clone, Debug)]
enum BlockType {
    Verse,
    Src,
    Export,
    Example,
    Comment,
    Center,
    Quote,
}

fn special_name_parser<'a>()
-> impl Parser<'a, &'a str, String, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    any()
        .filter(|c: &char| !c.is_whitespace())
        .repeated()
        .at_least(1)
        .collect::<String>()
        .try_map_with(|s, e| match s.to_uppercase().as_str() {
            "SRC" | "EXPORT" | "EXAMPLE" | "COMMENT" | "VERSE" | "CENTER" | "QUOTE" => {
                let error =
                    Rich::custom::<&str>(e.span(), &format!("{s} is not special block name"));
                Err(error)
            }
            _ => Ok(s),
        })
}

fn special_block_begin_row_parser<'a>()
-> impl Parser<'a, &'a str, NT, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    object::whitespaces()
        .then(object::just_case_insensitive("#+begin_"))
        .then(special_name_parser())
        .then(
            object::whitespaces_g1()
                .then(none_of("\n").repeated().collect::<String>())
                .or_not(),
        )
        .then(just("\n"))
        .map_with(|((((ws, begin), name), maybe_ws_parameters), nl), e| {
            let mut children = vec![];

            if ws.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OSK::Whitespace.into(),
                    &ws,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OSK::Text.into(),
                &begin,
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OSK::Text.into(),
                &name.to_uppercase(),
            )));

            e.state().block_type.push(name.clone().to_uppercase()); // update state

            match maybe_ws_parameters {
                None => {}
                Some((ws, p)) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Whitespace.into(),
                        &ws,
                    )));
                    children.push(NodeOrToken::Token(GreenToken::new(OSK::Text.into(), &p)));
                }
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OSK::Newline.into(),
                &nl,
            )));

            NodeOrToken::Node(GreenNode::new(OSK::BlockBegin.into(), children))
        })
}

fn special_block_end_row_parser<'a>()
-> impl Parser<'a, &'a str, NT, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    object::whitespaces()
        .then(object::just_case_insensitive("#+END_"))
        .then(special_name_parser())
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .try_map_with(|((((ws1, end), block_type), ws2), nl), e| {
            // Not using validate, use try_map_with to halt when an error is generated instead of continuing
            // Not using map_with, which is not executed in and_is(block_rend_row_parser().not())

            let block_type_match = e
                .state()
                .block_type
                .last()
                .map_or(false, |c| *c == block_type.to_uppercase());
            // println!("special_block_end_row_parser@try_map_with@end: type@state={:?}, type@current={}, block_type_match={}", e.state().block_type, block_type.to_uppercase(), block_type_match);
            match block_type_match {
                false => Err(Rich::custom(e.span(), &format!("block type mismatched",))),
                true => Ok((ws1, end, block_type, ws2, nl)),
            }
        })
        .map(|(ws1, end, btype, ws2, nl)| {
            let mut children = vec![];
            if ws1.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OSK::Whitespace.into(),
                    &ws1,
                )));
            }
            children.push(NodeOrToken::Token(GreenToken::new(OSK::Text.into(), &end)));
            children.push(NodeOrToken::Token(GreenToken::new(
                OSK::Text.into(),
                &btype.to_uppercase(),
            )));
            if ws2.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OSK::Whitespace.into(),
                    &ws2,
                )));
            }
            match nl {
                Some(_nl) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Newline.into(),
                        &_nl,
                    )));
                }
                None => {}
            }
            NodeOrToken::Node(GreenNode::new(OSK::BlockEnd.into(), children))
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
    let end_row = end_row_parser("export");

    // No line may start with #+end_NAME.
    // Lines beginning with an asterisk must be quoted by a comma (,*) and lines beginning with #+ may be quoted by a comma when necessary (#+).
    let content = content_inner_parser(end_row.clone());
    let affiliated_keywords = element::keyword::affiliated_keyword_parser()
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(
            |((((keywords, begin_row), content), end_row), blank_lines), e| {
                let mut children = vec![];

                for keyword in keywords {
                    children.push(keyword);
                }

                let block_begin_node = {
                    let (ws1, begin, block_type, ws2, data, ws3, nl) = begin_row;
                    let mut children = vec![];
                    if ws1.len() > 0 {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OSK::Whitespace.into(),
                            &ws1,
                        )));
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Text.into(),
                        &begin,
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Text.into(),
                        &block_type.to_uppercase(),
                    )));

                    if ws2.len() > 0 {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OSK::Whitespace.into(),
                            &ws2,
                        )));
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(OSK::Text.into(), &data)));

                    if ws3.len() > 0 {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OSK::Whitespace.into(),
                            &ws3,
                        )));
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Newline.into(),
                        &nl,
                    )));
                    NodeOrToken::Node(GreenNode::new(OSK::BlockBegin.into(), children))
                };
                children.push(block_begin_node);

                if content.len() > 0 {
                    let mut c_children = vec![];
                    c_children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Text.into(),
                        &content,
                    )));
                    let node =
                        NodeOrToken::Node(GreenNode::new(OSK::BlockContent.into(), c_children));
                    children.push(node);
                }

                children.push(end_row);

                for bl in blank_lines {
                    children.push(NodeOrToken::Token(bl));
                }

                NodeOrToken::Node(GreenNode::new(OSK::ExportBlock.into(), children))
            },
        )
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

    let end_row = end_row_parser("src");

    // No line may start with #+end_NAME.
    // Lines beginning with an asterisk must be quoted by a comma (,*) and lines beginning with #+ may be quoted by a comma when necessary (#+).
    let content = content_inner_parser(end_row.clone());

    let affiliated_keywords = element::keyword::affiliated_keyword_parser()
        .repeated()
        .collect::<Vec<_>>();
    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(
            |((((keywords, begin_row), content), end_row), blank_lines), e| {
                let mut children = vec![];

                for keyword in keywords {
                    children.push(keyword);
                }

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
                            OSK::Whitespace.into(),
                            &ws1,
                        )));
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Text.into(),
                        &begin,
                    )));

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Text.into(),
                        &block_type.to_uppercase(),
                    )));

                    if ws2.len() > 0 {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OSK::Whitespace.into(),
                            &ws2,
                        )));
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::SrcBlockLanguage.into(),
                        &language,
                    )));

                    if let Some((ws3, switches)) = maybe_ws3_switches {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OSK::Whitespace.into(),
                            &ws3,
                        )));

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OSK::SrcBlockSwitches.into(),
                            switches,
                        )));
                    }

                    if let Some((ws4, arguments)) = maybe_ws4_arguments {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OSK::Whitespace.into(),
                            &ws4,
                        )));

                        children.push(NodeOrToken::Token(GreenToken::new(
                            OSK::SrcBlockHeaderArguments.into(),
                            &arguments,
                        )));
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Newline.into(),
                        &nl,
                    )));
                    NodeOrToken::Node(GreenNode::new(OSK::BlockBegin.into(), children))
                };
                children.push(block_begin_node);

                if content.len() > 0 {
                    let mut c_children = vec![];
                    c_children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Text.into(),
                        &content,
                    )));
                    let node =
                        NodeOrToken::Node(GreenNode::new(OSK::BlockContent.into(), c_children));
                    children.push(node);
                }

                children.push(end_row);

                for bl in blank_lines {
                    children.push(NodeOrToken::Token(bl));
                }

                NodeOrToken::Node(GreenNode::new(OSK::SrcBlock.into(), children))
            },
        )
}

fn comment_or_example_block_parser<'a>(
    block_type: BlockType,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let name = match block_type {
        BlockType::Comment => "comment",
        BlockType::Example => "example",
        _ => {
            panic!("not supported type")
        }
    };

    let begin_row = begin_row_parser(name);
    let end_row = end_row_parser(name);
    let content = content_inner_parser(end_row.clone());
    let affiliated_keywords = element::keyword::affiliated_keyword_parser()
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(
            move |((((keywords, begin_row), content), end_row), blank_lines), _e| {
                let mut children = vec![];

                for keyword in keywords {
                    children.push(keyword);
                }

                children.push(begin_row);

                if content.len() > 0 {
                    let mut c_children = vec![];
                    c_children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Text.into(),
                        &content,
                    )));
                    let node =
                        NodeOrToken::Node(GreenNode::new(OSK::BlockContent.into(), c_children));
                    children.push(node);
                }

                children.push(end_row);

                for bl in blank_lines {
                    children.push(NodeOrToken::Token(bl));
                }

                match block_type {
                    BlockType::Comment => {
                        NodeOrToken::Node(GreenNode::new(OSK::CommentBlock.into(), children))
                    }
                    BlockType::Example => {
                        NodeOrToken::Node(GreenNode::new(OSK::ExampleBlock.into(), children))
                    }
                    _ => {
                        panic!("not supported type")
                    }
                }
            },
        )
}

/// comment block
pub(crate) fn comment_block_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    comment_or_example_block_parser(BlockType::Comment)
}

/// example block
pub(crate) fn example_block_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    comment_or_example_block_parser(BlockType::Example)
}

/// verse block
pub(crate) fn verse_block_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let begin_row = begin_row_parser("verse");
    let end_row = end_row_parser("verse");

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

    let affiliated_keywords = element::keyword::affiliated_keyword_parser()
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(
            move |((((keywords, begin_row), content), end_row), blank_lines), _e| {
                let mut children = vec![];

                for keyword in keywords {
                    children.push(keyword);
                }

                children.push(begin_row);

                let mut content_children = vec![];
                for node in content {
                    content_children.push(node);
                }
                children.push(NodeOrToken::Node(GreenNode::new(
                    OSK::BlockContent.into(),
                    content_children,
                )));

                children.push(end_row);

                for bl in blank_lines {
                    children.push(NodeOrToken::Token(bl));
                }

                NodeOrToken::Node(GreenNode::new(OSK::VerseBlock.into(), children))
            },
        )
}

// begin_row: #+begin_name[ data][ ]\n
// for quote/center/comment/example
fn begin_row_parser<'a>(
    name: &'a str,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let data = none_of("\n").repeated().at_least(1).collect::<String>();
    object::whitespaces()
        .then(object::just_case_insensitive("#+begin_"))
        .then(object::just_case_insensitive(name))
        .then(object::whitespaces_g1().then(data).or_not())
        .then(object::whitespaces()) // fixme: ? delete
        .then(just("\n"))
        .map(
            |(((((ws1, begin), block_type), maybe_ws2_data), ws3), nl)| {
                let mut children = vec![];
                if ws1.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Whitespace.into(),
                        &ws1,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OSK::Text.into(),
                    &begin,
                )));

                children.push(NodeOrToken::Token(GreenToken::new(
                    OSK::Text.into(),
                    &block_type.to_uppercase(),
                )));

                if let Some((ws2, data)) = maybe_ws2_data {
                    if ws2.len() > 0 {
                        children.push(NodeOrToken::Token(GreenToken::new(
                            OSK::Whitespace.into(),
                            &ws2,
                        )));
                    }

                    children.push(NodeOrToken::Token(GreenToken::new(OSK::Text.into(), &data)));
                }

                if ws3.len() > 0 {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OSK::Whitespace.into(),
                        &ws3,
                    )));
                }

                children.push(NodeOrToken::Token(GreenToken::new(
                    OSK::Newline.into(),
                    &nl,
                )));

                NodeOrToken::Node(GreenNode::new(OSK::BlockBegin.into(), children))
            },
        )
}

// for non-special block
fn end_row_parser<'a>(
    name: &'a str,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    object::whitespaces()
        .then(object::just_case_insensitive("#+end_"))
        .then(object::just_case_insensitive(name))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .map(|((((ws1, end), name), ws2), nl)| {
            let mut children = vec![];
            if ws1.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OSK::Whitespace.into(),
                    &ws1,
                )));
            }
            children.push(NodeOrToken::Token(GreenToken::new(OSK::Text.into(), &end)));
            children.push(NodeOrToken::Token(GreenToken::new(
                OSK::Text.into(),
                &name.to_uppercase(),
            )));

            if ws2.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OSK::Whitespace.into(),
                    &ws2,
                )));
            }

            if let Some(e) = nl {
                children.push(NodeOrToken::Token(GreenToken::new(OSK::Newline.into(), &e)));
            }
            NodeOrToken::Node(GreenNode::new(OSK::BlockEnd.into(), children))
        })
}

fn content_inner_parser<'a>(
    end_row: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<'a, &'a str, &'a str, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    object::line_parser()
        .or(object::blank_line_str_parser())
        .and_is(end_row.not()) // No line may start with #+end_NAME.
        .and_is(just("*").not())
        .repeated()
        .to_slice()
}

fn center_or_quote_block_parser<'a>(
    element_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
    block_type: BlockType,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    let name = match block_type {
        BlockType::Center => "center",
        BlockType::Quote => "quote",
        _ => {
            panic!("not supported type")
        }
    };

    let begin_row = begin_row_parser(name);
    let end_row = end_row_parser(name);
    let content_inner = object::line_parser()
        .or(object::blank_line_str_parser())
        .and_is(end_row.clone().not())
        .and_is(just("*").not())
        .repeated()
        .to_slice();
    let content = element_parser
        .repeated()
        .collect::<Vec<_>>()
        .nested_in(content_inner)
        .map(|s| {
            let mut children = vec![];
            for e in s {
                children.push(e);
            }
            NodeOrToken::Node(GreenNode::new(OSK::BlockContent.into(), children))
        });

    let affiliated_keywords = element::keyword::affiliated_keyword_parser()
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(
            move |((((keywords, begin_row), content), end_row), blank_lines), e| {
                // reset state
                let mut children = vec![];

                for keyword in keywords {
                    children.push(keyword);
                }

                children.push(begin_row);
                children.push(content);

                // let mut content_children = vec![];
                // for node in content {
                //     content_children.push(node);
                // }
                // children.push(NodeOrToken::Node(GreenNode::new(
                //     OSK::BlockContent.into(),
                //     content_children,
                // )));
                children.push(end_row);

                for bl in blank_lines {
                    children.push(NodeOrToken::Token(bl));
                }

                match block_type {
                    BlockType::Center => {
                        NodeOrToken::Node(GreenNode::new(OSK::CenterBlock.into(), children))
                    }
                    BlockType::Quote => {
                        NodeOrToken::Node(GreenNode::new(OSK::QuoteBlock.into(), children))
                    }
                    _ => {
                        panic!("xxx")
                    }
                }
            },
        )
}

pub(crate) fn center_block_parser<'a>(
    element_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    center_or_quote_block_parser(element_parser, BlockType::Center)
}

pub(crate) fn quote_block_parser<'a>(
    element_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    center_or_quote_block_parser(element_parser, BlockType::Quote)
}

pub(crate) fn special_block_parser<'a>(
    element_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    // println!("greater_block parser ...");

    let content_inner = object::line_parser()
        .or(object::blank_line_str_parser())
        .and_is(special_block_end_row_parser().not())
        .repeated()
        .to_slice();

    let content = element_parser
        .repeated()
        .collect::<Vec<_>>()
        .nested_in(content_inner)
        .map(|s| {
            let mut children = vec![];
            for e in s {
                children.push(e);
            }
            NodeOrToken::Node(GreenNode::new(OSK::BlockContent.into(), children))
        });

    let affiliated_keywords = element::keyword::affiliated_keyword_parser()
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .then(special_block_begin_row_parser())
        // .map(|s|{println!("greater_block@s1={s:?}"); s})
        .then(content)
        // .map(|s|{println!("greater_block@s2={s:?}"); s})
        .then(special_block_end_row_parser())
        // .map(|s|{println!("greater_block@s3={s:?}"); s})
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(
            |((((keywords, begin_row), content), end_row), blank_lines), e| {
                // reset state
                // println!("greater_block@map_with: content={:?}", content);
                let mut children = vec![];
                for keyword in keywords {
                    children.push(keyword);
                }
                children.push(begin_row);
                children.push(content);
                children.push(end_row);
                for bl in blank_lines {
                    children.push(NodeOrToken::Token(bl));
                }

                let block_type = e.state().block_type.last().unwrap();
                // let kind = match block_type.as_str() {
                //     "CENTER" => OSK::CenterBlock,
                //     "QUOTE" => OSK::QuoteBlock,
                //     _ => OSK::SpecialBlock,
                // };

                e.state().block_type.pop(); // reset state
                NodeOrToken::Node(GreenNode::new(OSK::SpecialBlock.into(), children))
            },
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::{get_parser_output, get_parsers_output};
    use crate::parser::element::element_parser;
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
    #[should_panic]
    fn test_src_block_02() {
        let input = "#+BEGIN_SRC python
#+END_DRC";
        get_parser_output(src_block_parser(), input);
    }

    #[test]
    fn test_src_block_03() {
        let input = "#+BEGIN_sRC python
#+END_SrC";
        assert_eq!(
            get_parser_output(src_block_parser(), input),
            r##"SrcBlock@0..28
  BlockBegin@0..19
    Text@0..8 "#+BEGIN_"
    Text@8..11 "SRC"
    Whitespace@11..12 " "
    SrcBlockLanguage@12..18 "python"
    Newline@18..19 "\n"
  BlockEnd@19..28
    Text@19..25 "#+END_"
    Text@25..28 "SRC"
"##
        );
    }

    #[test]
    fn test_src_block_src_04() {
        let input = r###"#+BEGIN_sRC python
print("hi");
print("py");
#+END_SrC"###;
        assert_eq!(
            get_parser_output(src_block_parser(), input),
            r##"SrcBlock@0..54
  BlockBegin@0..19
    Text@0..8 "#+BEGIN_"
    Text@8..11 "SRC"
    Whitespace@11..12 " "
    SrcBlockLanguage@12..18 "python"
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
    fn test_example_block_01() {
        let input = "#+BEGIN_example
#+END_examplE";
        assert_eq!(
            get_parser_output(example_block_parser(), input),
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

    #[test]
    fn test_center_block_01() {
        assert_eq!(
            get_parser_output(
                center_block_parser(element_parser()),
                r##"#+BEGIN_center
a *bold* test
#+END_center
"##
            ),
            r##"CenterBlock@0..42
  BlockBegin@0..15
    Text@0..8 "#+BEGIN_"
    Text@8..14 "CENTER"
    Newline@14..15 "\n"
  BlockContent@15..29
    Paragraph@15..29
      Text@15..17 "a "
      Bold@17..23
        Asterisk@17..18 "*"
        Text@18..22 "bold"
        Asterisk@22..23 "*"
      Text@23..29 " test\n"
  BlockEnd@29..42
    Text@29..35 "#+END_"
    Text@35..41 "CENTER"
    Newline@41..42 "\n"
"##
        );
    }

    #[test]
    fn test_center_block_02() {
        assert_eq!(
            get_parser_output(
                center_block_parser(element_parser()),
                r##"#+BEGIN_CENTER
     Everything should be made as simple as possible, \\
     but not any simpler
     #+END_CENTER
"##
            ),
            r##"CenterBlock@0..115
  BlockBegin@0..15
    Text@0..8 "#+BEGIN_"
    Text@8..14 "CENTER"
    Newline@14..15 "\n"
  BlockContent@15..97
    Paragraph@15..97
      Text@15..69 "     Everything shoul ..."
      LineBreak@69..71
        BackSlash2@69..71 "\\\\"
      Text@71..97 "\n     but not any sim ..."
  BlockEnd@97..115
    Whitespace@97..102 "     "
    Text@102..108 "#+END_"
    Text@108..114 "CENTER"
    Newline@114..115 "\n"
"##
        );
    }

    #[test]
    fn test_special_block_03() {
        assert_eq!(
            get_parser_output(
                special_block_parser(element_parser()),
                r##"#+BEGIN_xx
special block
#+END_xx
"##
            ),
            r##"SpecialBlock@0..34
  BlockBegin@0..11
    Text@0..8 "#+BEGIN_"
    Text@8..10 "XX"
    Newline@10..11 "\n"
  BlockContent@11..25
    Paragraph@11..25
      Text@11..25 "special block\n"
  BlockEnd@25..34
    Text@25..31 "#+END_"
    Text@31..33 "XX"
    Newline@33..34 "\n"
"##
        );
    }

    #[test]
    fn test_special_block_04() {
        assert_eq!(
            get_parser_output(
                special_block_parser(element_parser()),
                r##"#+BEGIN_xx
xx
#+begin_center
center
#+begin_quote
quote
#+end_quote
#+end_center
#+END_xx
"##
            ),
            r##"SpecialBlock@0..90
  BlockBegin@0..11
    Text@0..8 "#+BEGIN_"
    Text@8..10 "XX"
    Newline@10..11 "\n"
  BlockContent@11..81
    Paragraph@11..14
      Text@11..14 "xx\n"
    CenterBlock@14..81
      BlockBegin@14..29
        Text@14..22 "#+begin_"
        Text@22..28 "CENTER"
        Newline@28..29 "\n"
      BlockContent@29..68
        Paragraph@29..36
          Text@29..36 "center\n"
        QuoteBlock@36..68
          BlockBegin@36..50
            Text@36..44 "#+begin_"
            Text@44..49 "QUOTE"
            Newline@49..50 "\n"
          BlockContent@50..56
            Paragraph@50..56
              Text@50..56 "quote\n"
          BlockEnd@56..68
            Text@56..62 "#+end_"
            Text@62..67 "QUOTE"
            Newline@67..68 "\n"
      BlockEnd@68..81
        Text@68..74 "#+end_"
        Text@74..80 "CENTER"
        Newline@80..81 "\n"
  BlockEnd@81..90
    Text@81..87 "#+END_"
    Text@87..89 "XX"
    Newline@89..90 "\n"
"##
        );
    }

    #[test]
    fn test_special_block_05() {
        assert_eq!(
            get_parser_output(
                center_block_parser(element_parser()),
                r##"#+BEGIN_center
#+begin_quote
#+begin_xx
qq
#+end_xx
#+end_quote
#+end_center
"##
            ),
            r##"CenterBlock@0..77
  BlockBegin@0..15
    Text@0..8 "#+BEGIN_"
    Text@8..14 "CENTER"
    Newline@14..15 "\n"
  BlockContent@15..64
    QuoteBlock@15..64
      BlockBegin@15..29
        Text@15..23 "#+begin_"
        Text@23..28 "QUOTE"
        Newline@28..29 "\n"
      BlockContent@29..52
        SpecialBlock@29..52
          BlockBegin@29..40
            Text@29..37 "#+begin_"
            Text@37..39 "XX"
            Newline@39..40 "\n"
          BlockContent@40..43
            Paragraph@40..43
              Text@40..43 "qq\n"
          BlockEnd@43..52
            Text@43..49 "#+end_"
            Text@49..51 "XX"
            Newline@51..52 "\n"
      BlockEnd@52..64
        Text@52..58 "#+end_"
        Text@58..63 "QUOTE"
        Newline@63..64 "\n"
  BlockEnd@64..77
    Text@64..70 "#+end_"
    Text@70..76 "CENTER"
    Newline@76..77 "\n"
"##
        );
    }

    #[test]
    fn test_center_block_06() {
        // cant nested the same block
        assert_eq!(
            get_parsers_output(
                element_parser().repeated().collect::<Vec<_>>(),
                r##"#+BEGIN_center
#+begin_center
cc
#+end_center
#+end_center
"##
            ),
            r##"Root@0..59
  CenterBlock@0..46
    BlockBegin@0..15
      Text@0..8 "#+BEGIN_"
      Text@8..14 "CENTER"
      Newline@14..15 "\n"
    BlockContent@15..33
      Paragraph@15..33
        Text@15..22 "#+begin"
        Subscript@22..29
          Caret@22..23 "_"
          Text@23..29 "center"
        Text@29..33 "\ncc\n"
    BlockEnd@33..46
      Text@33..39 "#+end_"
      Text@39..45 "CENTER"
      Newline@45..46 "\n"
  Paragraph@46..59
    Text@46..51 "#+end"
    Subscript@51..58
      Caret@51..52 "_"
      Text@52..58 "center"
    Text@58..59 "\n"
"##
        );
    }

    #[test]
    fn test_center_block_07() {
        // cant nested the same block
        assert_eq!(
            get_parsers_output(
                element_parser().repeated().collect::<Vec<_>>(),
                r##"#+BEGIN_xx
#+begin_yy
#+begin_z
xyz
#+end_z
#+end_yy
#+end_xx
"##
            ),
            r##"Root@0..62
  SpecialBlock@0..62
    BlockBegin@0..11
      Text@0..8 "#+BEGIN_"
      Text@8..10 "XX"
      Newline@10..11 "\n"
    BlockContent@11..53
      SpecialBlock@11..53
        BlockBegin@11..22
          Text@11..19 "#+begin_"
          Text@19..21 "YY"
          Newline@21..22 "\n"
        BlockContent@22..44
          SpecialBlock@22..44
            BlockBegin@22..32
              Text@22..30 "#+begin_"
              Text@30..31 "Z"
              Newline@31..32 "\n"
            BlockContent@32..36
              Paragraph@32..36
                Text@32..36 "xyz\n"
            BlockEnd@36..44
              Text@36..42 "#+end_"
              Text@42..43 "Z"
              Newline@43..44 "\n"
        BlockEnd@44..53
          Text@44..50 "#+end_"
          Text@50..52 "YY"
          Newline@52..53 "\n"
    BlockEnd@53..62
      Text@53..59 "#+end_"
      Text@59..61 "XX"
      Newline@61..62 "\n"
"##
        );
    }
}
