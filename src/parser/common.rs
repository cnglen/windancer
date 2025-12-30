use crate::parser::ParserState;
use crate::parser::syntax::OrgLanguage;
use crate::parser::{MyExtra, NT, OSK};
use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::NodeOrToken;
use rowan::SyntaxNode;

#[allow(dead_code)]
pub(crate) fn get_parser_output<'a, C: 'a + std::default::Default>(
    parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
    input: &'a str,
) -> String {
    let state = RollbackState(ParserState::default());
    get_parser_output_with_state(parser, input, state)
}

#[allow(dead_code)]
pub(crate) fn get_parser_output_with_state<'a, C: 'a + std::default::Default>(
    parser: impl Parser<'a, &'a str, NT, MyExtra<'a, C>> + Clone,
    input: &'a str,
    mut state: RollbackState<ParserState>,
) -> String {
    let (maybe_output, errors) = parser
        .parse_with_state(input, &mut state)
        .into_output_errors();

    if let Some(output) = maybe_output {
        match output {
            NodeOrToken::Node(node) => {
                let syntax_tree: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(node);
                format!("{syntax_tree:#?}")
            }

            NodeOrToken::Token(token) => {
                let root = crate::node!(OSK::Root, vec![NodeOrToken::Token(token)]);
                let syntax_tree: SyntaxNode<OrgLanguage> =
                    SyntaxNode::new_root(root.into_node().expect("xx"));

                format!("{syntax_tree:#?}")
            }
        }
    } else {
        panic!("{:?}", errors);
    }
}

#[allow(dead_code)]
pub(crate) fn get_parsers_output<'a, C: 'a + std::default::Default>(
    parser: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone,
    input: &'a str,
) -> String {
    let state = RollbackState(ParserState::default());
    get_parsers_output_with_state(parser, input, state)
}
pub(crate) fn get_parsers_output_with_state<'a, C: 'a + std::default::Default>(
    parser: impl Parser<'a, &'a str, Vec<NT>, MyExtra<'a, C>> + Clone,
    input: &'a str,
    mut state: RollbackState<ParserState>,
) -> String {
    let (ans, _errors) = parser
        .parse_with_state(input, &mut state)
        .into_output_errors();
    // let ans = parser.parse_with_state(input, &mut state).unwrap();
    let children = ans.expect("has_output");
    let root = crate::node!(OSK::Root, children);
    println!("c={root:?}");
    let syntax_tree: SyntaxNode<OrgLanguage> = SyntaxNode::new_root(root.into_node().expect("xx"));

    format!("{syntax_tree:#?}")
}
