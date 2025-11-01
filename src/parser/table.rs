//! Table parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

fn table_cell<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    object::whitespaces()
        .then(
            none_of("\n|")
                .and_is(object::whitespaces().then(just("|")).not())
                .repeated()
                .collect::<String>(),
        )
        .then(object::whitespaces())
        .then(just("|").to(Some(String::from("|"))).or(end().to(None)))
        .map(|(((ws1, content), ws2), maybe_pipe)| {
            let mut children = vec![];

            if ws1.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws1,
                )));
            }

            if content.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &content,
                )));
            }

            if ws2.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws2,
                )));
            }

            match maybe_pipe {
                Some(pipe) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Pipe.into(),
                        &pipe,
                    )));
                }
                None => {}
            }

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::TableCell.into(),
                children,
            ))
        })
}

fn table_standard_row<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    object::whitespaces()
        .then(just("|"))
        .then(table_cell().repeated().collect::<Vec<_>>())
        .then(object::newline_or_ending())
        .map(|(((ws, pipe), cells), maybe_newline)| {
            let mut children = vec![];
            if ws.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )));
            }
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Pipe.into(),
                &pipe,
            )));
            for cell in cells {
                children.push(cell);
            }
            match maybe_newline {
                Some(newline) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Newline.into(),
                        &newline,
                    )));
                }
                None => {}
            }

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::TableStandardRow.into(),
                children,
            ))
        })
}

fn table_rule_row<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    object::whitespaces()
        .then(just("|"))
        .then(just("-"))
        .then(none_of("\n").repeated().collect::<String>())
        .then(object::newline_or_ending())
        .map(|((((ws, pipe), dash), content), maybe_newline)| {
            let mut children = vec![];
            if ws.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )));
            }
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Pipe.into(),
                &pipe,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Dash.into(),
                &dash,
            )));
            if content.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &content,
                )));
            }

            match maybe_newline {
                Some(newline) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Newline.into(),
                        &newline,
                    )));
                }
                None => {}
            }

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::TableRuleRow.into(),
                children,
            ))
        })
}

pub(crate) fn table_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    table_rule_row()
        .or(table_standard_row())
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(|(rows, blanklines)| {
            let mut children = vec![];

            for row in rows {
                children.push(row);
                // match row {
                //     NodeOrToken::Node(n) => {
                //         match n.kind() {
                //             val if val== OrgSyntaxKind::TableStandardRow.into() => {},
                //             val if val== OrgSyntaxKind::TableRuleRow.into() => {},
                //             _ => {}
                //         }
                //     }
                //     _ => {},
            }

            // println!("row={:#?}", row);
            // match row {

            //     children.push(row);
            // }
            // }

            for blankline in blanklines {
                children.push(NodeOrToken::Token(blankline));
            }

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Table.into(), children))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::SyntaxNode;

    #[test]
    fn test_table_cell() {
        let inputs = vec![" foo  |", "foo  |", "  foo|", "foo|", "foo"];
        let mut state = RollbackState(ParserState::default());
        for input in inputs {
            let ans = table_cell().parse_with_state(input, &mut state);
            match ans.clone().unwrap() {
                NodeOrToken::Node(e) => {
                    let syntax_node = SyntaxNode::new_root(e.clone());
                    println!("{:#?}", syntax_node);
                }
                _ => {}
            }

            assert!(ans.has_output());
        }
    }

    #[test]
    fn test_table() {
        let input = r##"  | Name  | Phone | Age |
  |-------+-------+-----|
  | Peter |  1234 |  24 |
  | Anna  |  4321 |  25 |
"##;
        let mut state = RollbackState(ParserState::default());
        let t = table_parser().parse_with_state(input, &mut state);
        let syntax_tree = SyntaxNode::new_root(t.into_result().unwrap().into_node().expect("xxx"));

        println!("{:#?}", syntax_tree);
        // assert_eq!(format!("{:#?}", syntax_tree), ans)
    }
}
