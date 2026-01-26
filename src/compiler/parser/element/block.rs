//! Block parser
// - center/quote/special
// - export
// - src
// - verse
// - example
// - comment
// todo:
// - lots of redundancy code, combine to one block_parser(), which include bold/verse/example
// - #+BEGIN the same prefix to reduce rewind number
// begin_row: #+begin_name[ data][ ]\n
// for quote/center/comment/example/verse
// not for src/export/special
// begin_row <- #+BEGIN_NAME DATA?
// DATA <- !newline any*
// special: dynamic name
// export: data is mandatory
// src: data langugae
// data_parser
// name_parser
// fn block_parser_todo<'a, C: 'a, E, F>(
//     name_parser: E,
//     data_parser: F
// ) -> impl Parser<
//     'a,
//     &'a str,
//     NT,
//     MyExtra<'a, C>,
//     > + Clone
// where E: Parser<
//     'a,
//     &'a str,
//     &'a str,
//     MyExtra<'a, C>,
//     > + Clone,
// F:  Parser<
//     'a,
//     &'a str,
//     Vec<NT>,
//     MyExtra<'a, C>,
//     > + Clone,
// {
//     object::whitespaces()
//         .then(object::just_case_insensitive("#+begin_"))
//         .then(
//             // example
//             // src
//         )
//         .then()
//         .then(name_parser)
//         .then(data_parser)
//         .then(object::newline())
//         .then(content_parser)
//         .then()
//         .map(((((whitespaces, hash_plus_begin_underscore), name), data), newline)) {
//             let mut children = Vec::with_capacity(7);
//             if !whitespaces.is_empty() {
//                 children.push(NT::Token(GreenToken::new(
//                     OSK::Whitespace,
//                     whitespaces,
//                 )));
//             }

//             children.push(NT::Token(GreenToken::new(
//                 OSK::Text,
//                 begin,
//             )));
//             children.push(NT::Token(GreenToken::new(
//                 OSK::Text,
//                 name
//             )));
//             children.extend(data);
//             children.push(NT::Token(GreenToken::new(
//                 OSK::Newline,
//                 &nl,
//             )));
//         }
// }

use crate::compiler::parser::config::OrgParserConfig;
use crate::compiler::parser::{MyExtra, NT, OSK};
use crate::compiler::parser::{element, object};
use chumsky::prelude::*;
use phf::phf_set;

#[allow(unused)]
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

pub(crate) static ORG_BLOCK_NON_SPECIAL_NAMES: phf::Set<&'static str> = phf_set! {
    "SRC", "EXPORT", "EXAMPLE", "COMMENT", "VERSE", "CENTER", "QUOTE"
};

fn special_name_parser<'a, C: 'a>() -> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone {
    custom(|inp| {
        let before = inp.cursor();
        loop {
            match inp.peek() {
                Some(c) if !(c as char).is_whitespace() => {
                    inp.next();
                }
                _ => {
                    break;
                }
            }
        }
        let name: &str = inp.slice_since(&before..);
        if name.is_empty() || ORG_BLOCK_NON_SPECIAL_NAMES.contains(&name.to_uppercase()) {
            return Err(Rich::custom(
                inp.span_since(&before),
                format!("invalid special block name: '{}'", name),
            ));
        }

        Ok(name)
    })
}

/// export block
pub(crate) fn export_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let data = none_of("\n \t").repeated().at_least(1).to_slice();
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
    let affiliated_keywords = element::keyword::affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    // let affiliated_keywords = element::keyword::affiliated_keyword_parser()
    //     .repeated()
    //     .collect::<Vec<_>>();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(
            |((((keywords, begin_row), content), end_row), blank_lines)| {
                let mut children = Vec::with_capacity(3 + keywords.len() + blank_lines.len());

                children.extend(keywords);

                let block_begin_node = {
                    let (ws1, begin, block_type, ws2, data, ws3, nl) = begin_row;

                    let mut children = Vec::with_capacity(7);
                    if !ws1.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, ws1));
                    }
                    children.push(crate::token!(OSK::Text, begin));
                    children.push(crate::token!(OSK::Text, block_type));
                    if !ws2.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, ws2));
                    }
                    children.push(crate::token!(OSK::Text, data));
                    if !ws3.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, &ws3));
                    }
                    children.push(crate::token!(OSK::Newline, &nl));

                    crate::node!(OSK::BlockBegin, children)
                };
                children.push(block_begin_node);

                if !content.is_empty() {
                    let node =
                        crate::node!(OSK::BlockContent, vec![crate::token!(OSK::Text, content)]);
                    children.push(node);
                }

                children.push(end_row);
                children.extend(blank_lines);
                crate::node!(OSK::ExportBlock, children)
            },
        )
        .boxed()
}

/// simple export block
pub(crate) fn simple_export_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    let data = none_of("\n \t").repeated().at_least(1).to_slice();
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
    let affiliated_keywords = element::keyword::simple_affiliated_keyword_parser(config).repeated();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .ignored()
        .boxed()
}

pub(crate) fn simple_src_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    let language = none_of(" \t\n").repeated().at_least(1).to_slice();
    let switch_p1 = just("-l")
        .then(object::whitespaces_g1())
        .then(none_of("\n\" \t").repeated())
        .to_slice();
    let switch_p2 = one_of("+-")
        .then(any().filter(|c: &char| c.is_alphanumeric()))
        .to_slice();
    let switches = switch_p1.or(switch_p2).repeated().to_slice();
    let arguments = none_of("\n").repeated().to_slice();
    let begin_row = object::whitespaces()
        .then(object::just_case_insensitive("#+BEGIN_"))
        .then(object::just_case_insensitive("SRC")) // name
        .then(object::whitespaces_g1())
        .then(language)
        .then(object::whitespaces_g1().then(switches).or_not())
        .then(object::whitespaces_g1().then(arguments).or_not())
        .then(just("\n"));

    let end_row = end_row_parser("src");

    // No line may start with #+end_NAME.
    // Lines beginning with an asterisk must be quoted by a comma (,*) and lines beginning with #+ may be quoted by a comma when necessary (#+).
    let content = content_inner_parser(end_row.clone());

    let affiliated_keywords = element::keyword::simple_affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .ignored()
        .boxed()
}

/// src block
pub(crate) fn src_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let language = none_of(" \t\n").repeated().at_least(1).to_slice();
    let switch_p1 = just("-l")
        .then(object::whitespaces_g1())
        .then(none_of("\n\" \t").repeated())
        .to_slice();
    let switch_p2 = one_of("+-")
        .then(any().filter(|c: &char| c.is_alphanumeric()))
        .to_slice();
    let switches = switch_p1.or(switch_p2).repeated().at_least(1).to_slice();
    let arguments = none_of("\n").repeated().at_least(1).to_slice();
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
    let affiliated_keywords = element::keyword::affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(
            |((((keywords, begin_row), content), end_row), blank_lines)| {
                let mut children = Vec::with_capacity(keywords.len() + blank_lines.len() + 3);
                children.extend(keywords);
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
                    let mut children = Vec::with_capacity(10);
                    if !ws1.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, &ws1));
                    }

                    children.push(crate::token!(OSK::Text, &begin));

                    children.push(crate::token!(OSK::Text, block_type));

                    if !ws2.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, &ws2));
                    }

                    children.push(crate::token!(OSK::SrcBlockLanguage, language));

                    if let Some((ws3, switches)) = maybe_ws3_switches {
                        children.push(crate::token!(OSK::Whitespace, &ws3));

                        children.push(crate::token!(OSK::SrcBlockSwitches, switches));
                    }

                    if let Some((ws4, arguments)) = maybe_ws4_arguments {
                        children.push(crate::token!(OSK::Whitespace, &ws4));

                        children.push(crate::token!(OSK::SrcBlockHeaderArguments, arguments));
                    }

                    children.push(crate::token!(OSK::Newline, &nl));
                    crate::node!(OSK::BlockBegin, children)
                };
                children.push(block_begin_node);
                if !content.is_empty() {
                    let node =
                        crate::node!(OSK::BlockContent, vec![crate::token!(OSK::Text, &content)]);
                    children.push(node);
                }
                children.push(end_row);
                children.extend(blank_lines);

                crate::node!(OSK::SrcBlock, children)
            },
        )
        .boxed()
}

fn comment_or_example_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
    block_type: BlockType,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
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
    // let affiliated_keywords = element::keyword::affiliated_keyword_parser()
    //     .repeated()
    //     .collect::<Vec<_>>();
    let affiliated_keywords = element::keyword::affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(
            move |((((keywords, begin_row), content), end_row), blank_lines)| {
                let mut children = Vec::with_capacity(4 + keywords.len() + blank_lines.len());

                children.extend(keywords);

                children.push(begin_row);

                if !content.is_empty() {
                    let mut c_children = Vec::with_capacity(1);
                    c_children.push(crate::token!(OSK::Text, &content));
                    let node = crate::node!(OSK::BlockContent, c_children);
                    children.push(node);
                }

                children.push(end_row);

                children.extend(blank_lines);

                match block_type {
                    BlockType::Comment => {
                        crate::node!(OSK::CommentBlock, children)
                    }
                    BlockType::Example => {
                        crate::node!(OSK::ExampleBlock, children)
                    }
                    _ => {
                        panic!("not supported type")
                    }
                }
            },
        )
        .boxed()
}

/// comment block
pub(crate) fn comment_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    comment_or_example_block_parser(config, BlockType::Comment)
}

/// example block
pub(crate) fn example_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    comment_or_example_block_parser(config, BlockType::Example)
}

fn simple_comment_or_example_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
    block_type: BlockType,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
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
    let affiliated_keywords = element::keyword::simple_affiliated_keyword_parser(config).repeated();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .ignored()
        .boxed()
}

/// simple comment block
pub(crate) fn simple_comment_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    simple_comment_or_example_block_parser(config, BlockType::Comment)
}

/// simple example block
pub(crate) fn simple_example_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    simple_comment_or_example_block_parser(config, BlockType::Example)
}

/// verse block
pub(crate) fn verse_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let begin_row = begin_row_parser("verse");
    let end_row = end_row_parser("verse");

    let fullset_objects_parser = object::object_parser(config.clone())
        .clone()
        .repeated()
        .at_least(1)
        .collect::<Vec<NT>>();
    let content_inner = object::line_parser_allow_blank()
        .and_is(end_row.clone().ignored().not()) // No line may start with #+end_NAME.
        .and_is(element::heading::simple_heading_row_parser().not())
        .repeated();
    let content = fullset_objects_parser.nested_in(content_inner.to_slice());

    // let affiliated_keywords = element::keyword::affiliated_keyword_parser()
    //     .repeated()
    //     .collect::<Vec<_>>();
    let affiliated_keywords = element::keyword::affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(
            move |((((keywords, begin_row), content), end_row), blank_lines)| {
                let mut children = Vec::with_capacity(4 + keywords.len() + blank_lines.len());

                children.extend(keywords);

                children.push(begin_row);

                children.push(crate::node!(OSK::BlockContent, content));

                children.push(end_row);

                children.extend(blank_lines);

                crate::node!(OSK::VerseBlock, children)
            },
        )
        .boxed()
}

pub(crate) fn simple_verse_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    let begin_row = begin_row_parser("verse");
    let end_row = end_row_parser("verse");
    let content_inner = object::line_parser_allow_blank()
        .and_is(end_row.clone().ignored().not()) // No line may start with #+end_NAME.
        .and_is(
            element::heading::simple_heading_row_parser()
                .ignored()
                .not(),
        )
        .repeated();
    // let affiliated_keywords = element::keyword::simple_affiliated_keyword_parser().repeated();
    let affiliated_keywords = element::keyword::affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .ignore_then(begin_row)
        .ignore_then(content_inner)
        .ignore_then(end_row)
        .ignore_then(object::blank_line_parser().repeated())
        .ignored()
        .boxed()
}

fn begin_row_parser<'a, C: 'a>(
    name: &'a str,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let data = none_of("\n").repeated().to_slice();

    object::whitespaces()
        .then(object::just_case_insensitive("#+begin_"))
        .then(object::just_case_insensitive(name))
        .then(object::whitespaces_g1().then(data).or_not())
        .then(object::newline())
        .map(|((((ws1, begin), block_type), maybe_ws2_data), nl)| {
            let mut children = Vec::with_capacity(7);
            if !ws1.is_empty() {
                children.push(crate::token!(OSK::Whitespace, &ws1));
            }

            children.push(crate::token!(OSK::Text, &begin));

            children.push(crate::token!(OSK::Text, block_type));

            if let Some((ws2, data)) = maybe_ws2_data {
                if !ws2.is_empty() {
                    children.push(crate::token!(OSK::Whitespace, &ws2));
                }

                children.push(crate::token!(OSK::Text, data));
            }

            children.push(crate::token!(OSK::Newline, &nl));

            crate::node!(OSK::BlockBegin, children)
        })
}

// for non-special block
fn end_row_parser<'a, C: 'a>(
    name: &'a str,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    object::whitespaces()
        .then(object::just_case_insensitive("#+end_"))
        .then(object::just_case_insensitive(name))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .map(|((((ws1, end), name), ws2), nl)| {
            let mut children = Vec::with_capacity(5);
            if !ws1.is_empty() {
                children.push(crate::token!(OSK::Whitespace, &ws1));
            }
            children.push(crate::token!(OSK::Text, &end));
            children.push(crate::token!(OSK::Text, name));

            if !ws2.is_empty() {
                children.push(crate::token!(OSK::Whitespace, &ws2));
            }

            if let Some(e) = nl {
                children.push(crate::token!(OSK::Newline, &e));
            }
            crate::node!(OSK::BlockEnd, children)
        })
}

fn content_inner_parser<'a, C: 'a>(
    end_row: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
) -> impl Parser<'a, &'a str, &'a str, MyExtra<'a, C>> + Clone {
    object::line_parser()
        .or(object::blank_line_str_parser())
        .and_is(end_row.ignored().not()) // No line may start with #+end_NAME.
        .and_is(
            element::heading::simple_heading_row_parser()
                .ignored()
                .not(),
        )
        .repeated()
        .to_slice()
}

fn center_or_quote_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
    element_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
    block_type: BlockType,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
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
        .and_is(
            element::heading::simple_heading_row_parser()
                .ignored()
                .not(),
        )
        .repeated()
        .to_slice();
    let content = element_parser
        .repeated()
        .collect::<Vec<_>>()
        .nested_in(content_inner)
        .map(|s| crate::node!(OSK::BlockContent, s));

    let affiliated_keywords = element::keyword::affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    affiliated_keywords
        .then(begin_row)
        .then(content)
        .then(end_row)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(
            move |((((keywords, begin_row), content), end_row), blank_lines)| {
                // reset state
                let mut children = Vec::with_capacity(keywords.len() + 3 + blank_lines.len());
                children.extend(keywords);
                children.push(begin_row);
                children.push(content);
                children.push(end_row);
                children.extend(blank_lines);

                match block_type {
                    BlockType::Center => {
                        crate::node!(OSK::CenterBlock, children)
                    }
                    BlockType::Quote => crate::node!(OSK::QuoteBlock, children),
                    _ => {
                        panic!("xxx")
                    }
                }
            },
        )
        .boxed()
}

pub(crate) fn center_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
    element_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    center_or_quote_block_parser(config, element_parser, BlockType::Center)
}

pub(crate) fn quote_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
    element_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    center_or_quote_block_parser(config, element_parser, BlockType::Quote)
}

fn simple_center_or_quote_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
    block_type: BlockType,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
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
        .and_is(end_row.clone().ignored().not())
        .and_is(
            element::heading::simple_heading_row_parser()
                .ignored()
                .not(),
        )
        .repeated();
    let affiliated_keywords = element::keyword::simple_affiliated_keyword_parser(config).repeated();

    affiliated_keywords
        .ignore_then(begin_row)
        .ignore_then(content_inner)
        .ignore_then(end_row)
        .ignore_then(object::blank_line_parser().repeated())
        .ignored()
        .boxed()
}

pub(crate) fn simple_center_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    simple_center_or_quote_block_parser(config, BlockType::Center)
}

pub(crate) fn simple_quote_block_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    simple_center_or_quote_block_parser(config, BlockType::Quote)
}

pub(crate) fn special_block_parser<'a, C: 'a + std::default::Default>(
    config: OrgParserConfig,
    element_parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    // let affiliated_keywords = element::keyword::affiliated_keyword_parser()
    //     .repeated()
    //     .collect::<Vec<_>>();
    let affiliated_keywords = element::keyword::affiliated_keyword_parser(config)
        .repeated()
        .collect::<Vec<_>>();

    let begin_row = object::whitespaces()
        .then(object::just_case_insensitive("#+begin_"))
        .then(special_name_parser())
        .then(
            object::whitespaces_g1()
                .then(none_of("\n").repeated().to_slice())
                .or_not(),
        )
        .then(object::newline())
        .map(
            |((((begin_whitespaces1, begin), begin_name), maybe_ws_parameters), begin_newline)| {
                (
                    begin_whitespaces1,
                    begin,
                    begin_name,
                    maybe_ws_parameters,
                    begin_newline,
                )
            },
        );

    let end_row = object::whitespaces()
        .then(object::just_case_insensitive("#+end_"))
        .then(just("").configure(
            |cfg, ctx: &(&str, &str, &str, Option<(&str, &str)>, &str)| cfg.seq((*ctx).2),
        ))
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .map(
            |((((end_whitespaces1, end_), end_name), end_whitespaces2), end_maybe_newline)| {
                (
                    end_whitespaces1,
                    end_,
                    end_name,
                    end_whitespaces2,
                    end_maybe_newline,
                )
            },
        );

    let content_inner = object::line_parser()
        .or(object::blank_line_str_parser())
        .and_is(end_row.clone().ignored().not())
        .and_is(
            element::heading::simple_heading_row_parser()
                .ignored()
                .not(),
        )
        .repeated()
        .to_slice();

    affiliated_keywords
        .then(
            begin_row // element_parser can't be used here since element_parser's context is ()!!! move to the final map()
                .then_with_ctx(content_inner.then(end_row)),
        )
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map_with(
            move |(
                (
                    keywords,
                    (
                        (begin_whitespaces1, begin, begin_name, maybe_ws_parameters, begin_newline),
                        (
                            contents,
                            (end_whitespaces1, end_, end_name, end_whitespaces2, end_maybe_newline),
                        ),
                    ),
                ),
                blanklines,
            ),
                  e| {
                let mut children = Vec::with_capacity(keywords.len() + 3 + blanklines.len());
                children.extend(keywords);

                let begin_node = {
                    let mut children = Vec::with_capacity(6);
                    if !begin_whitespaces1.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, begin_whitespaces1));
                    }
                    children.push(crate::token!(OSK::Text, begin));
                    children.push(crate::token!(OSK::Text, begin_name));

                    if let Some((ws, parameters)) = maybe_ws_parameters {
                        if !ws.is_empty() {
                            children.push(crate::token!(OSK::Whitespace, &ws));
                        }

                        children.push(crate::token!(OSK::Text, parameters));
                    }

                    children.push(crate::token!(OSK::Newline, begin_newline));

                    crate::node!(OSK::BlockBegin, children)
                };
                children.push(begin_node);

                // element_parser is here to avoid context error
                let mut state = e.state();
                let content_node = element_parser
                    .clone()
                    .repeated()
                    .collect::<Vec<_>>()
                    .map(|s| crate::node!(OSK::BlockContent, s))
                    .parse_with_state(contents, &mut state)
                    .into_output()
                    .unwrap();
                children.push(content_node);

                let end_node = {
                    let mut children = Vec::with_capacity(5);
                    if !end_whitespaces1.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, end_whitespaces1));
                    }

                    children.push(crate::token!(OSK::Text, &end_));

                    children.push(crate::token!(OSK::Text, end_name));

                    if !end_whitespaces2.is_empty() {
                        children.push(crate::token!(OSK::Whitespace, end_whitespaces2));
                    }

                    if let Some(newline) = end_maybe_newline {
                        children.push(crate::token!(OSK::Newline, &newline));
                    }
                    crate::node!(OSK::BlockEnd, children)
                };
                children.push(end_node);
                children.extend(blanklines);

                crate::node!(OSK::SpecialBlock, children)
            },
        )
        .boxed()
}

pub(crate) fn simple_special_block_parser<'a, C: 'a + std::default::Default>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    let affiliated_keywords = element::keyword::simple_affiliated_keyword_parser(config).repeated();

    let begin_row = object::whitespaces()
        .then(object::just_case_insensitive("#+begin_"))
        .then(special_name_parser())
        .then(
            object::whitespaces_g1()
                .then(none_of("\n").repeated().to_slice())
                .or_not(),
        )
        .then(object::newline())
        .map(
            |((((begin_whitespaces1, begin), begin_name), maybe_ws_parameters), begin_newline)| {
                (
                    begin_whitespaces1,
                    begin,
                    begin_name,
                    maybe_ws_parameters,
                    begin_newline,
                )
            },
        );

    let end_row = object::whitespaces()
        .ignore_then(object::just_case_insensitive("#+end_"))
        .ignore_then(just("").configure(
            |cfg, ctx: &(&str, &str, &str, Option<(&str, &str)>, &str)| cfg.seq((*ctx).2),
        ))
        .ignore_then(object::whitespaces())
        .ignore_then(object::newline_or_ending())
        .ignored();

    let content_inner = object::line_parser()
        .or(object::blank_line_str_parser())
        .and_is(end_row.ignored().not())
        .repeated()
        .to_slice();

    affiliated_keywords
        .ignore_then(
            begin_row // element_parser can't be used here since element_parser's context is ()!!! move to the final map()
                .then_with_ctx(content_inner.ignore_then(end_row)),
        )
        .ignore_then(object::blank_line_parser().repeated())
        .boxed()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::parser::common::{get_parser_output, get_parsers_output};
    use crate::compiler::parser::config::OrgParserConfig;
    use crate::compiler::parser::element::element_parser;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_export_block_01() {
        assert_eq!(
            get_parser_output(
                export_block_parser::<()>(OrgParserConfig::default()),
                r##"#+BEGIN_export html 
#+END_export
"##
            ),
            r###"ExportBlock@0..34
  BlockBegin@0..21
    Text@0..8 "#+BEGIN_"
    Text@8..14 "export"
    Whitespace@14..15 " "
    Text@15..19 "html"
    Whitespace@19..20 " "
    Newline@20..21 "\n"
  BlockEnd@21..34
    Text@21..27 "#+END_"
    Text@27..33 "export"
    Newline@33..34 "\n"
"###
        );
    }

    #[test]
    #[should_panic]
    fn test_export_block_02() {
        get_parser_output(
            export_block_parser::<()>(OrgParserConfig::default()),
            r##"#+BEGIN_export 
#+END_export
"##,
        );
    }

    #[test]
    #[should_panic]
    fn test_export_block_03() {
        get_parser_output(
            export_block_parser::<()>(OrgParserConfig::default()),
            r##"#+BEGIN_export html latex
#+END_export
"##,
        );
    }

    #[test]
    #[should_panic]
    fn test_export_block_04() {
        get_parser_output(
            export_block_parser::<()>(OrgParserConfig::default()),
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
                verse_block_parser::<()>(OrgParserConfig::default()),
                r##"#+BEGIN_verse

example
#+END_verse
"##
            ),
            r##"VerseBlock@0..35
  BlockBegin@0..14
    Text@0..8 "#+BEGIN_"
    Text@8..13 "verse"
    Newline@13..14 "\n"
  BlockContent@14..23
    Text@14..23 "\nexample\n"
  BlockEnd@23..35
    Text@23..29 "#+END_"
    Text@29..34 "verse"
    Newline@34..35 "\n"
"##
        );
    }

    #[test]
    fn test_verse_block_02() {
        assert_eq!(
            get_parser_output(
                verse_block_parser::<()>(OrgParserConfig::default()),
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
                src_block_parser::<()>(OrgParserConfig::default()),
                r##"#+BEGIN_src rust -l -n :var foo=bar  
fn main() {
}
#+END_src
"##
            ),
            r##"SrcBlock@0..62
  BlockBegin@0..38
    Text@0..8 "#+BEGIN_"
    Text@8..11 "src"
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
    Text@58..61 "src"
    Newline@61..62 "\n"
"##
        );
    }

    #[test]
    #[should_panic]
    fn test_src_block_02() {
        let input = "#+BEGIN_SRC python
#+END_DRC";
        get_parser_output(src_block_parser::<()>(OrgParserConfig::default()), input);
    }

    #[test]
    fn test_src_block_03() {
        let input = "#+BEGIN_sRC python
#+END_SrC";
        assert_eq!(
            get_parser_output(src_block_parser::<()>(OrgParserConfig::default()), input),
            r##"SrcBlock@0..28
  BlockBegin@0..19
    Text@0..8 "#+BEGIN_"
    Text@8..11 "sRC"
    Whitespace@11..12 " "
    SrcBlockLanguage@12..18 "python"
    Newline@18..19 "\n"
  BlockEnd@19..28
    Text@19..25 "#+END_"
    Text@25..28 "SrC"
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
            get_parser_output(src_block_parser::<()>(OrgParserConfig::default()), input),
            r##"SrcBlock@0..54
  BlockBegin@0..19
    Text@0..8 "#+BEGIN_"
    Text@8..11 "sRC"
    Whitespace@11..12 " "
    SrcBlockLanguage@12..18 "python"
    Newline@18..19 "\n"
  BlockContent@19..45
    Text@19..45 "print(\"hi\");\nprint(\"p ..."
  BlockEnd@45..54
    Text@45..51 "#+END_"
    Text@51..54 "SrC"
"##
        );
    }

    #[test]
    fn test_example_block_01() {
        let input = "#+BEGIN_example
#+END_examplE";
        assert_eq!(
            get_parser_output(
                example_block_parser::<()>(OrgParserConfig::default()),
                input
            ),
            r##"ExampleBlock@0..29
  BlockBegin@0..16
    Text@0..8 "#+BEGIN_"
    Text@8..15 "example"
    Newline@15..16 "\n"
  BlockEnd@16..29
    Text@16..22 "#+END_"
    Text@22..29 "examplE"
"##
        );
    }

    #[test]
    fn test_center_block_01() {
        assert_eq!(
            get_parser_output(
                center_block_parser(
                    OrgParserConfig::default(),
                    element_parser::<()>(OrgParserConfig::default())
                ),
                r##"#+BEGIN_center
a *bold* test
#+END_center
"##
            ),
            r##"CenterBlock@0..42
  BlockBegin@0..15
    Text@0..8 "#+BEGIN_"
    Text@8..14 "center"
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
    Text@35..41 "center"
    Newline@41..42 "\n"
"##
        );
    }

    #[test]
    fn test_center_block_02() {
        assert_eq!(
            get_parser_output(
                center_block_parser(
                    OrgParserConfig::default(),
                    element_parser::<()>(OrgParserConfig::default())
                ),
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
                special_block_parser(
                    OrgParserConfig::default(),
                    element_parser::<()>(OrgParserConfig::default())
                ),
                r##"#+BEGIN_xx
special block
#+END_xx
"##
            ),
            r##"SpecialBlock@0..34
  BlockBegin@0..11
    Text@0..8 "#+BEGIN_"
    Text@8..10 "xx"
    Newline@10..11 "\n"
  BlockContent@11..25
    Paragraph@11..25
      Text@11..25 "special block\n"
  BlockEnd@25..34
    Text@25..31 "#+END_"
    Text@31..33 "xx"
    Newline@33..34 "\n"
"##
        );
    }

    #[test]
    fn test_special_block_04() {
        assert_eq!(
            get_parser_output(
                special_block_parser(
                    OrgParserConfig::default(),
                    element_parser::<()>(OrgParserConfig::default())
                ),
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
    Text@8..10 "xx"
    Newline@10..11 "\n"
  BlockContent@11..81
    Paragraph@11..14
      Text@11..14 "xx\n"
    CenterBlock@14..81
      BlockBegin@14..29
        Text@14..22 "#+begin_"
        Text@22..28 "center"
        Newline@28..29 "\n"
      BlockContent@29..68
        Paragraph@29..36
          Text@29..36 "center\n"
        QuoteBlock@36..68
          BlockBegin@36..50
            Text@36..44 "#+begin_"
            Text@44..49 "quote"
            Newline@49..50 "\n"
          BlockContent@50..56
            Paragraph@50..56
              Text@50..56 "quote\n"
          BlockEnd@56..68
            Text@56..62 "#+end_"
            Text@62..67 "quote"
            Newline@67..68 "\n"
      BlockEnd@68..81
        Text@68..74 "#+end_"
        Text@74..80 "center"
        Newline@80..81 "\n"
  BlockEnd@81..90
    Text@81..87 "#+END_"
    Text@87..89 "xx"
    Newline@89..90 "\n"
"##
        );
    }

    #[test]
    fn test_special_block_05() {
        assert_eq!(
            get_parser_output(
                center_block_parser(
                    OrgParserConfig::default(),
                    element_parser::<()>(OrgParserConfig::default())
                ),
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
    Text@8..14 "center"
    Newline@14..15 "\n"
  BlockContent@15..64
    QuoteBlock@15..64
      BlockBegin@15..29
        Text@15..23 "#+begin_"
        Text@23..28 "quote"
        Newline@28..29 "\n"
      BlockContent@29..52
        SpecialBlock@29..52
          BlockBegin@29..40
            Text@29..37 "#+begin_"
            Text@37..39 "xx"
            Newline@39..40 "\n"
          BlockContent@40..43
            Paragraph@40..43
              Text@40..43 "qq\n"
          BlockEnd@43..52
            Text@43..49 "#+end_"
            Text@49..51 "xx"
            Newline@51..52 "\n"
      BlockEnd@52..64
        Text@52..58 "#+end_"
        Text@58..63 "quote"
        Newline@63..64 "\n"
  BlockEnd@64..77
    Text@64..70 "#+end_"
    Text@70..76 "center"
    Newline@76..77 "\n"
"##
        );
    }

    #[test]
    fn test_center_block_06() {
        // cant nested the same block
        assert_eq!(
            get_parsers_output(
                element_parser::<()>(OrgParserConfig::default())
                    .repeated()
                    .collect::<Vec<_>>(),
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
      Text@8..14 "center"
      Newline@14..15 "\n"
    BlockContent@15..33
      Paragraph@15..33
        Text@15..22 "#+begin"
        Subscript@22..29
          Underscore@22..23 "_"
          Text@23..29 "center"
        Text@29..33 "\ncc\n"
    BlockEnd@33..46
      Text@33..39 "#+end_"
      Text@39..45 "center"
      Newline@45..46 "\n"
  Paragraph@46..59
    Text@46..51 "#+end"
    Subscript@51..58
      Underscore@51..52 "_"
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
                element_parser::<()>(OrgParserConfig::default())
                    .repeated()
                    .collect::<Vec<_>>(),
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
      Text@8..10 "xx"
      Newline@10..11 "\n"
    BlockContent@11..53
      SpecialBlock@11..53
        BlockBegin@11..22
          Text@11..19 "#+begin_"
          Text@19..21 "yy"
          Newline@21..22 "\n"
        BlockContent@22..44
          SpecialBlock@22..44
            BlockBegin@22..32
              Text@22..30 "#+begin_"
              Text@30..31 "z"
              Newline@31..32 "\n"
            BlockContent@32..36
              Paragraph@32..36
                Text@32..36 "xyz\n"
            BlockEnd@36..44
              Text@36..42 "#+end_"
              Text@42..43 "z"
              Newline@43..44 "\n"
        BlockEnd@44..53
          Text@44..50 "#+end_"
          Text@50..52 "yy"
          Newline@52..53 "\n"
    BlockEnd@53..62
      Text@53..59 "#+end_"
      Text@59..61 "xx"
      Newline@61..62 "\n"
"##
        );
    }

    #[test]
    fn test_src_block_99() {
        let input = "#+NAME: foo
#+begin_src mermaid :file ./assets/test_demo_git.png :cache yes :exports results
  flowchart RL
    WD[WorkingDirectory] -- git add --> SA[StageArea] -- git commit --> Repo[.git]
    Repo --> |git checkout| WD
#+end_src
";

        assert_eq!(
            get_parser_output(src_block_parser::<()>(OrgParserConfig::default()), input),
            r##"SrcBlock@0..232
  AffiliatedKeyword@0..12
    HashPlus@0..2 "#+"
    KeywordKey@2..6
      Text@2..6 "NAME"
    Colon@6..7 ":"
    Whitespace@7..8 " "
    KeywordValue@8..11
      Text@8..11 "foo"
    Newline@11..12 "\n"
  BlockBegin@12..93
    Text@12..20 "#+begin_"
    Text@20..23 "src"
    Whitespace@23..24 " "
    SrcBlockLanguage@24..31 "mermaid"
    Whitespace@31..32 " "
    SrcBlockHeaderArguments@32..92 ":file ./assets/test_d ..."
    Newline@92..93 "\n"
  BlockContent@93..222
    Text@93..222 "  flowchart RL\n    WD ..."
  BlockEnd@222..232
    Text@222..228 "#+end_"
    Text@228..231 "src"
    Newline@231..232 "\n"
"##
        );
    }
}
