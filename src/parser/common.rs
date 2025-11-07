use crate::parser::ParserState;
use crate::parser::S2;
use crate::parser::syntax::{OrgLanguage, OrgSyntaxKind};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::SyntaxNode;
use rowan::{GreenNode, GreenToken, NodeOrToken};

#[allow(dead_code)]
pub(crate) fn get_parser_output<'a>(
    parser: impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>
    + Clone,
    input: &'a str,
) -> String {
    let a = parser.parse(input);

    let mut errors = vec![];
    errors.push(String::from("errors:"));
    if a.has_errors() {
        for error in a.errors() {
            // println!("{:?}", error);
            errors.push(format!("{:?}", error));
        }
    }

    if a.has_output() {
        match parser.parse(input).unwrap() {
            S2::Single(NodeOrToken::Node(node)) => {
                let syntax_tree: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(node);
                // println!("{syntax_tree:#?}");
                format!("{syntax_tree:#?}")
            }

            S2::Single(NodeOrToken::Token(token)) => {
                let root = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::Root.into(),
                    vec![NodeOrToken::Token(token)],
                ));
                let syntax_tree: SyntaxNode<OrgLanguage> =
                    SyntaxNode::new_root(root.into_node().expect("xx"));

                format!("{syntax_tree:#?}")
            }

            _ => String::from(""),
        }
    } else {
        errors.join("\n")
    }
}

#[allow(dead_code)]
pub(crate) fn get_parser_output_with_state<'a>(
    parser: impl Parser<'a, &'a str, S2, extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>>
    + Clone,
    input: &'a str,
    mut state: RollbackState<ParserState>,
) -> String {
    let a = parser.parse_with_state(input, &mut state);

    let mut errors = vec![];
    errors.push(String::from("errors:"));
    if a.has_errors() {
        for error in a.errors() {
            // println!("{:?}", error);
            errors.push(format!("{:?}", error));
        }
    }

    if a.has_output() {
        match parser.parse(input).unwrap() {
            S2::Single(NodeOrToken::Node(node)) => {
                let syntax_tree: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(node);
                // println!("{syntax_tree:#?}");
                format!("{syntax_tree:#?}")
            }

            S2::Single(NodeOrToken::Token(token)) => {
                let root = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
                    OrgSyntaxKind::Root.into(),
                    vec![NodeOrToken::Token(token)],
                ));
                let syntax_tree: SyntaxNode<OrgLanguage> =
                    SyntaxNode::new_root(root.into_node().expect("xx"));

                format!("{syntax_tree:#?}")
            }

            _ => String::from(""),
        }
    } else {
        errors.join("\n")
    }
}

#[allow(dead_code)]
pub(crate) fn get_parsers_output<'a>(
    parser: impl Parser<
        'a,
        &'a str,
        Vec<S2>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
    input: &'a str,
) -> String {
    let mut state = RollbackState(ParserState::default());
    get_parsers_output_with_state(parser, input, state)
}
pub(crate) fn get_parsers_output_with_state<'a>(
    parser: impl Parser<
        'a,
        &'a str,
        Vec<S2>,
        extra::Full<Rich<'a, char>, RollbackState<ParserState>, ()>,
    > + Clone,
    input: &'a str,
    mut state: RollbackState<ParserState>,
) -> String {
    let ans = parser.parse_with_state(input, &mut state).unwrap();
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
