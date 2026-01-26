//! Table parser
use crate::compiler::parser::config::OrgParserConfig;
use crate::compiler::parser::object;
use crate::compiler::parser::{MyExtra, NT, OSK};
use chumsky::prelude::*;

use crate::compiler::parser::element::keyword::{
    affiliated_keyword_parser, simple_affiliated_keyword_parser,
};

// Any line with ‘|’ as the first non-whitespace character, then any number of table cells
fn table_standard_row<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    object::whitespaces()
        .then(just("|"))
        .then(
            object::table_cell::table_cell_parser(object::object_in_table_cell_parser(config))
                .repeated()
                .collect::<Vec<_>>(),
        )
        .then(object::newline_or_ending())
        .map(|(((ws, pipe), cells), maybe_newline)| {
            let mut children = Vec::with_capacity(3 + cells.len());
            if !ws.is_empty() {
                children.push(crate::token!(OSK::Whitespace, ws));
            }
            children.push(crate::token!(OSK::Pipe, pipe));
            children.extend(cells);
            if let Some(newline) = maybe_newline {
                children.push(crate::token!(OSK::Newline, newline));
            }

            crate::node!(OSK::TableStandardRow, children)
        })
}

// Any line with ‘|’ as the first non-whitespace character, then a line starting with ‘|-’ is a horizontal rule.  It separates rows explicitly.
fn table_rule_row<'a, C: 'a>() -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    object::whitespaces()
        .then(just("|"))
        .then(just("-"))
        .then(none_of("\n").repeated().to_slice())
        .then(object::newline_or_ending())
        .map(|((((ws, pipe), dash), content), maybe_newline)| {
            let mut children = Vec::with_capacity(5);
            if !ws.is_empty() {
                children.push(crate::token!(OSK::Whitespace, ws));
            }
            children.push(crate::token!(OSK::Pipe, pipe));
            children.push(crate::token!(OSK::Dash, dash));
            if content.len() > 0 {
                children.push(crate::token!(OSK::Text, content));
            }

            match maybe_newline {
                Some(newline) => {
                    children.push(crate::token!(OSK::Newline, newline));
                }
                None => {}
            }

            crate::node!(OSK::TableRuleRow, children)
        })
}

pub(crate) fn table_formula_parser<'a, C: 'a>()
-> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    just("#+")
        .then(object::just_case_insensitive("TBLFM"))
        .then(just(":"))
        .then(object::whitespaces())
        .then(none_of(object::CRLF).repeated().to_slice())
        .then(object::newline_or_ending())
        .map(|(((((hash_plus, tblfm), colon), ws), formula), nl)| {
            let mut children = Vec::with_capacity(6);

            children.push(crate::token!(OSK::HashPlus, hash_plus));

            children.push(crate::node!(
                OSK::KeywordKey,
                vec![crate::token!(OSK::Text, tblfm)]
            ));

            children.push(crate::token!(OSK::Colon, colon));

            if !ws.is_empty() {
                children.push(crate::token!(OSK::Whitespace, ws));
            }

            children.push(crate::node!(
                OSK::TableFormulaValue,
                vec![crate::token!(OSK::Text, formula)]
            ));

            match nl {
                Some(newline) => {
                    children.push(crate::token!(OSK::Newline, newline));
                }
                None => {}
            }

            crate::node!(OSK::TableFormula, children)
        })
}

pub(crate) fn table_parser_inner<'a, C: 'a>(
    config: OrgParserConfig,
    affiliated_keywords_parser: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone + 'a,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    // fixme: standard objects without footnote reference
    let rows = table_rule_row()
        .or(table_standard_row(config))
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>();
    let formulas = table_formula_parser().repeated().collect::<Vec<_>>();

    affiliated_keywords_parser
        .then(rows)
        .then(formulas)
        .then(object::blank_line_parser().repeated().collect::<Vec<_>>())
        .map(|(((affiliated_keywords, rows), formulas), blanklines)| {
            let mut children = Vec::with_capacity(
                affiliated_keywords.len() + rows.len() + formulas.len() + blanklines.len(),
            );
            children.extend(affiliated_keywords);
            children.extend(rows);
            children.extend(formulas);
            children.extend(blanklines);

            crate::node!(OSK::Table, children)
        })
        .boxed()
}

pub(crate) fn table_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone {
    let affiliated_keywords_parser = affiliated_keyword_parser(config.clone())
        .repeated()
        .collect::<Vec<_>>();

    table_parser_inner(config, affiliated_keywords_parser)
}

// used for negative lookahead
pub(crate) fn simple_table_parser<'a, C: 'a>(
    config: OrgParserConfig,
) -> impl Parser<'a, &'a str, (), MyExtra<'a, C>> + Clone {
    let affiliated_keywords_parser = simple_affiliated_keyword_parser(config.clone())
        .repeated()
        .collect::<Vec<_>>();

    table_parser_inner(config, affiliated_keywords_parser).ignored()
}

#[cfg(test)]
mod tests {
    use super::*;
    // use crate::compiler::parser::ParserState;
    use crate::compiler::parser::SyntaxNode;
    use crate::compiler::parser::config::OrgParserConfig;

    // use chumsky::inspector::RollbackState;

    #[test]
    fn test_table() {
        let input = r##"  | Name  | Phone | Age |
  |-------+-------+-----|
  | Peter |  1234 |  24 |
  | Anna  |  4321 |  25 |
"##;
        // let mut state = RollbackState(ParserState::default());
        // let t = table_parser::<()>().parse_with_state(input, &mut state);
        // let t = table_parser::<()>().parse(input);
        let t = table_parser::<()>(OrgParserConfig::default()).parse(input);
        let syntax_tree = SyntaxNode::new_root(t.into_result().unwrap().into_node().expect("xxx"));

        println!("{:#?}", syntax_tree);
        // assert_eq!(format!("{:#?}", syntax_tree), ans)
    }
}
