//! Planning parser
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::{ParserState, object};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};

pub(crate) fn planning_parser<'a, C: 'a>() -> impl Parser<
    'a,
    &'a str,
    NodeOrToken<GreenNode, GreenToken>,
    extra::Full<Rich<'a, char>, RollbackState<ParserState>, C>,
> + Clone {
    object::whitespaces()
        .then(choice((
            just("DEADLINE"),
            just("SCHEDULED"),
            just("CLOSED"),
        )))
        .then(just(":"))
        .then(object::whitespaces())
        .then(object::timestamp::timestamp_parser())
        // .map(|s|{println!("planning: s={s:#?}"); s})
        .map(|((((ws1, keyword), colon), ws2), ts)| {
            let mut children = Vec::with_capacity(5);
            if !ws1.is_empty() {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    ws1,
                )));
            }
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::PlanningKeyword.into(),
                keyword,
            )));
            children.push(NodeOrToken::Token(GreenToken::new(
                OrgSyntaxKind::Colon.into(),
                colon,
            )));
            if !ws2.is_empty() {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    ws2,
                )));
            }
            children.push(ts);

            children
        })
        .repeated()
        .at_least(1)
        .collect::<Vec<_>>()
        .then(object::whitespaces())
        .then(object::newline_or_ending())
        .map(|((s, ws), maybe_nl)| {
            let mut children = vec![];

            for e in s {
                for ee in e {
                    children.push(ee);
                }
            }

            if ws.len() > 0 {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Whitespace.into(),
                    ws,
                )));
            }

            if let Some(nl) = maybe_nl {
                children.push(NodeOrToken::Token(GreenToken::new(
                    OrgSyntaxKind::Newline.into(),
                    nl,
                )));
            }

            NodeOrToken::Node(GreenNode::new(OrgSyntaxKind::Planning.into(), children))
        })
        .boxed()
}

#[cfg(test)]
mod tests {
    use crate::parser::common::get_parser_output;
    use crate::parser::element::planning::planning_parser;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_planning_01() {
        assert_eq!(
            get_parser_output(planning_parser::<()>(), r"   SCHEDULED: <1999-03-31 Wed>"),
            r##"Planning@0..30
  Whitespace@0..3 "   "
  PlanningKeyword@3..12 "SCHEDULED"
  Colon@12..13 ":"
  Whitespace@13..14 " "
  Timestamp@14..30
    Text@14..30 "<1999-03-31 Wed>"
"##
        );
    }

    #[test]
    fn test_planning_02() {
        assert_eq!(
            get_parser_output(
                planning_parser::<()>(),
                r"     SCHEDULED: <2006-03-12 Sun> DEADLINE: <2034-03-22 Wed>  "
            ),
            r##"Planning@0..61
  Whitespace@0..5 "     "
  PlanningKeyword@5..14 "SCHEDULED"
  Colon@14..15 ":"
  Whitespace@15..16 " "
  Timestamp@16..32
    Text@16..32 "<2006-03-12 Sun>"
  Whitespace@32..33 " "
  PlanningKeyword@33..41 "DEADLINE"
  Colon@41..42 ":"
  Whitespace@42..43 " "
  Timestamp@43..59
    Text@43..59 "<2034-03-22 Wed>"
  Whitespace@59..61 "  "
"##
        );
    }
}
