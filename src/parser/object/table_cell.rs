use crate::parser::ParserState;
use crate::parser::object;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// table cell parser
pub(crate) fn table_cell_parser<'a, C: 'a>(
    object_parser: impl Parser<
        'a,
        &'a str,
        NodeOrToken<GreenNode, GreenToken>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
    > + Clone
    + 'a,
) -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
    let minimal_and_other_objects_parser = object_parser
        .clone()
        .repeated()
        .collect::<Vec<NodeOrToken<GreenNode, GreenToken>>>();

    // CONTENTS SPACES|
    let contents_inner = none_of("|\n")
        .and_is(object::whitespaces().ignore_then(just("|").ignored()).not())
        .repeated()
        .to_slice();
    let contents = minimal_and_other_objects_parser.nested_in(contents_inner);
    // note: EOL not supported for simplicity
    let pipe = just("|");

    contents
        .then(object::whitespaces())
        .then(pipe)
        // .map(|s|{println!("table_cell_parser: s={s:?}"); s})
        .map(|((contents, ws), pipe)| {
            let mut children = Vec::with_capacity(contents.len() + 2);
            // println!("contents={:?}; ws={:?}; pipe={:?}", contents, ws, pipe);

            children.extend(contents);

            if !ws.is_empty() {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    &ws,
                )));
            }

            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Pipe.into(),
                &pipe,
            )));

            NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::TableCell.into(),
                children,
            ))
        })
        .boxed()
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::common::get_parser_output;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_table_cell_01() {
        assert_eq!(
            get_parser_output(
                table_cell_parser(object::object_in_table_cell_parser::<()>()),
                " foo |"
            ),
            r##"TableCell@0..6
  Text@0..4 " foo"
  Whitespace@4..5 " "
  Pipe@5..6 "|"
"##
        );
    }

    #[test]
    fn test_table_cell_02() {
        assert_eq!(
            get_parser_output(
                table_cell_parser(object::object_in_table_cell_parser::<()>()),
                " |"
            ),
            r##"TableCell@0..2
  Whitespace@0..1 " "
  Pipe@1..2 "|"
"##
        );
    }

    #[test]
    fn test_table_cell_03() {
        assert_eq!(
            get_parser_output(
                table_cell_parser(object::object_in_table_cell_parser::<()>()),
                "|"
            ),
            r##"TableCell@0..1
  Pipe@0..1 "|"
"##
        );
    }

    #[test]
    fn test_table_cell_04() {
        assert_eq!(
            get_parser_output(
                table_cell_parser(object::object_in_table_cell_parser::<()>()),
                "foo  |"
            ),
            r##"TableCell@0..6
  Text@0..3 "foo"
  Whitespace@3..5 "  "
  Pipe@5..6 "|"
"##
        );
    }

    #[test]
    fn test_table_cell_05() {
        assert_eq!(
            get_parser_output(
                table_cell_parser(object::object_in_table_cell_parser::<()>()),
                "  foo|"
            ),
            r##"TableCell@0..6
  Text@0..5 "  foo"
  Pipe@5..6 "|"
"##
        );
    }

    #[test]
    fn test_table_cell_06() {
        assert_eq!(
            get_parser_output(
                table_cell_parser(object::object_in_table_cell_parser::<()>()),
                "foo|"
            ),
            r##"TableCell@0..4
  Text@0..3 "foo"
  Pipe@3..4 "|"
"##
        );
    }

    #[test]
    #[should_panic]
    fn test_table_cell_07() {
        assert_eq!(
            get_parser_output(
                table_cell_parser(object::object_in_table_cell_parser::<()>()),
                "foo"
            ),
            r##"TableCell@0..6
  Text@0..4 " foo"
  Whitespace@4..5 " "
  Pipe@5..6 "|"
"##
        );
    }
}
