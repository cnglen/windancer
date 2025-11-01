//! Block parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, S2, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use std::collections::HashSet;

pub(crate) fn block_begin_row_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    object::whitespaces()
        .then(object::just_case_insensitive("#+BEGIN_"))
        .then(
            any()
                .filter(|c: &char| !c.is_whitespace())
                .repeated()
                .at_least(1)
                .collect::<String>(),
        )
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
            let node =
                NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockBegin.into(), children));

            S2::Single(node)
        })
}

pub(crate) fn block_end_row_parser<'a>()
-> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
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
            let node = NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::BlockEnd.into(), children));
            S2::Single(node)
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
            match begin_row {
                S2::Single(n) => {
                    children.push(n);
                }
                _ => {}
            }

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

            match end_row {
                S2::Single(n) => {
                    children.push(n);
                }
                _ => {}
            }

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
    use crate::parser::{ParserState, SyntaxNode};

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
