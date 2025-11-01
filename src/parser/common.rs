use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::{OrgLanguage, OrgSyntaxKind};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken};
use rowan::{SyntaxNode, SyntaxToken};

#[allow(dead_code)]
pub(crate) fn get_parser_output<'a>(
    parser: impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>
    + Clone,
    input: &'a str,
) -> String {
    match parser.parse(input).unwrap() {
        S2::Single(node) => {
            let syntax_tree: SyntaxNode<OrgLanguage> =
                SyntaxNode::new_root(node.into_node().expect("syntax node"));
            format!("{syntax_tree:#?}")
        }
        _ => String::from(""),
    }
}

#[allow(dead_code)]
pub(crate) fn get_parsers_output<'a>(
    parser: impl Parser<'a, &'a str, Vec<S2>, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>
    + Clone,
    input: &'a str,
) -> String {
    let ans = parser.parse(input).unwrap();
    let mut children: Vec<NodeOrToken<GreenNode, GreenToken>> = vec![];
    ans.iter().for_each(|e| match e {
        S2::Single(nt) => {
            children.push(nt.clone());
        }
        S2::Double(nt1, nt2) => {
            children.push(nt1.clone());
            children.push(nt2.clone());
        }
        _ => {}
    });
    let root = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
        OrgSyntaxKind::Root.into(),
        children.clone(),
    ));
    let syntax_tree: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(root.into_node().expect("xx"));

    format!("{syntax_tree:#?}")
}
