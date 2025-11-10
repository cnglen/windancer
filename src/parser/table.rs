//! Table parser
use crate::parser::keyword::affiliated_keyword_parser;
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

// Any line with ‘|’ as the first non-whitespace character, then any number of table cells
fn table_standard_row<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    object::whitespaces()
        .then(just("|"))
        .then(
            object::table_cell::table_cell_parser(object::object_in_table_cell_parser())
                .repeated()
                .collect::<Vec<_>>(),
        )
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

// Any line with ‘|’ as the first non-whitespace character, then a line starting with ‘|-’ is a horizontal rule.  It separates rows explicitly.
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

pub(crate) fn table_formula_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    just("#+")
        .then(object::just_case_insensitive("TBLFM"))
        .then(just(":"))
        .then(object::whitespaces())
        .then(none_of("\r\n").repeated().collect::<String>())
        .then(object::newline_or_ending())
        .map_with(|(((((hash_plus, tblfm), colon), ws), formula), nl), e| {
            let mut children = vec![];

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::HashPlus.into(),
                hash_plus,
            )));

            children.push(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::KeywordKey.into(),
                vec![NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &tblfm,
                ))],
            )));

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                colon,
            )));

            if ws.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )));
            }

            children.push(NodeOrToken::Node(GreenNode::new(
                OrgSyntaxKind::TableFormulaValue.into(),
                vec![NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Text.into(),
                    &formula,
                ))],
            )));

            match nl {
                Some(newline) => {
                    children.push(NodeOrToken::Token(GreenToken::new(
                        OrgSyntaxKind::Newline.into(),
                        &newline,
                    )));
                    e.state().prev_char = newline.chars().last();
                }
                None => {}
            }

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::TableFormula.into(), children))
        })
}

pub(crate) fn table_parser<'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
> + Clone {
    // fixme: standard objects without footnote reference
    let affiliated_keywords = affiliated_keyword_parser(object::standard_set_object_parser())
        .repeated()
        .collect::<Vec<_>>();
    let rows = table_rule_row()
        .or(table_standard_row())
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>();
    let formulas = table_formula_parser().repeated().collect::<Vec<_>>();

    affiliated_keywords
        .then(rows)
        .then(formulas)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(|(((affiliated_keywords, rows), formulas), blanklines)| {
            let mut children = vec![];

            for e in affiliated_keywords {
                children.push(e);
            }

            for e in rows {
                children.push(e);
            }

            for e in formulas {
                children.push(e);
            }

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
