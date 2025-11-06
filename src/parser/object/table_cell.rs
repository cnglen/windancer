use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::object;
use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

/// table cell parser
pub(crate) fn table_cell_parser<'a>(
    object_parser: impl Parser<
        'a,
        &'a str,
        S2,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
) -> impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>> + Clone
{
    let minimal_and_other_objects_parser = object_parser.clone().repeated().collect::<Vec<S2>>();

    // CONTENTS SPACES|
    let contents_inner = none_of("|\n")
        .and_is(object::whitespaces().then(just("|")).not())
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
            let mut children = vec![];
            // println!("contents={:?}; ws={:?}; pipe={:?}", contents, ws, pipe);

            for node in contents {
                match node {
                    S2::Single(e) => {
                        children.push(e);
                    }
                    S2::Double(e1, e2) => {
                        children.push(e1);
                        children.push(e2);
                    }
                    _ => {}
                }
            }

            // if contents.len()>0 {
            //     children.push(NodeOrToken::Token(GreenToken::new(
            //         OrgSyntaxKind::Text.into(),
            //         &contents,
            //     )));
            // }

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


            S2::Single(NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                OrgSyntaxKind::TableCell.into(),
                children,
            )))
        })

    // object::whitespaces()
    //     .then(
    //         none_of("\n|")
    //             .and_is(object::whitespaces().then(just("|")).not())
    //             .repeated()
    //             .collect::<String>(),
    //     )
    //     .then(object::whitespaces())
    //     .then(just("|").to(Some(String::from("|"))).or(end().to(None)))
    //     .map(|(((ws1, content), ws2), maybe_pipe)| {
    //         let mut children = vec![];

    //         if ws1.len() > 0 {
    //             children.push(NodeOrToken::Token(GreenToken::new(
    //                 OrgSyntaxKind::Whitespace.into(),
    //                 &ws1,
    //             )));
    //         }

    //         if content.len() > 0 {
    //             children.push(NodeOrToken::Token(GreenToken::new(
    //                 OrgSyntaxKind::Text.into(),
    //                 &content,
    //             )));
    //         }

    //         if ws2.len() > 0 {
    //             children.push(NodeOrToken::Token(GreenToken::new(
    //                 OrgSyntaxKind::Whitespace.into(),
    //                 &ws2,
    //             )));
    //         }

    //         match maybe_pipe {
    //             Some(pipe) => {
    //                 children.push(NodeOrToken::Token(GreenToken::new(
    //                     OrgSyntaxKind::Pipe.into(),
    //                     &pipe,
    //                 )));
    //             }
    //             None => {}
    //         }

    //         S2::Single(
    //             NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
    //                 OrgSyntaxKind::TableCell.into(),
    //                 children,
    //             ))
    //         )
    // })
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
                table_cell_parser(object::object_in_table_cell_parser()),
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
                table_cell_parser(object::object_in_table_cell_parser()),
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
                table_cell_parser(object::object_in_table_cell_parser()),
                "|"
            ),
            r##"TableCell@0..1
  Pipe@0..1 "|"
"##
        );
    }
}
