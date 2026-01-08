//! Parser of org-mode
use crate::parser::syntax::OrgSyntaxKind;
use crate::parser::syntax::SyntaxNode;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken, WalkEvent};
use std::collections::HashSet;
use std::sync::OnceLock;
mod common;
mod document;
mod element;
pub(crate) mod object;
pub(crate) mod syntax;
static RADIO_TARGETS: OnceLock<HashSet<String>> = OnceLock::new();
pub mod config;

type NT = NodeOrToken<GreenNode, GreenToken>;
type OSK = OrgSyntaxKind;
type MyState = extra::SimpleState<ParserState>;
type MyExtra<'a, C> = extra::Full<Rich<'a, char>, MyState, C>;

#[derive(Clone, Debug)]
pub struct ParserState {}

impl Default for ParserState {
    fn default() -> Self {
        Self {}
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ParserResult {
    green: NodeOrToken<GreenNode, GreenToken>,
}

impl ParserResult {
    // green tree
    pub fn green(&self) -> &GreenNode {
        match &self.green {
            NodeOrToken::Node(e) => e,
            NodeOrToken::Token(_) => todo!(),
        }
    }

    // red tree
    pub fn syntax(&self) -> SyntaxNode {
        match &self.green {
            NodeOrToken::Node(e) => SyntaxNode::new_root(e.clone()),
            NodeOrToken::Token(_) => todo!(),
        }
    }
}

#[allow(dead_code)]
pub struct OrgParser {
    pub config: config::OrgParserConfig,
}

impl OrgParser {
    pub fn new(config: config::OrgParserConfig) -> Self {
        OrgParser { config }
    }

    // update RADIO_TARGETS, maybe use SimpleState a better method. Note: performance.
    pub fn get_radio_targets(&self, input: &str) -> &'static HashSet<String> {
        let radio_targets: Vec<NodeOrToken<GreenNode, GreenToken>> =
            object::objects_parser::<()>(self.config.clone())
                .parse(input)
                .unwrap()
                .into_iter()
                .filter(|s| match s {
                    NodeOrToken::<GreenNode, GreenToken>::Node(n)
                        if n.kind() == OrgSyntaxKind::RadioTarget.into() =>
                    {
                        true
                    }
                    _ => false,
                })
                .collect();

        let root = NodeOrToken::<GreenNode, GreenToken>::Node(GreenNode::new(
            OrgSyntaxKind::Root.into(),
            radio_targets,
        ));

        let mut radio_targets_text = vec![];
        let syntax_tree = SyntaxNode::new_root(root.into_node().expect("xx"));
        for e in syntax_tree.children() {
            let mut text = String::new();
            let mut preorder = e.preorder_with_tokens();
            while let Some(event) = preorder.next() {
                match event {
                    WalkEvent::Enter(element) => {
                        if let Some(token) = element.as_token() {
                            if token.kind() != OrgSyntaxKind::LeftAngleBracket3
                                && token.kind() != OrgSyntaxKind::RightAngleBracket3
                            {
                                text.push_str(token.text());
                            }
                        }
                    }
                    _ => {}
                }
            }
            radio_targets_text.push(text);
        }
        RADIO_TARGETS.get_or_init(|| {
            let mut targets = HashSet::new();
            for e in radio_targets_text {
                targets.insert(e);
            }
            targets
        })
    }

    pub fn parse(&mut self, input: &str) -> ParserResult {
        let radio_target_lines = input
            .lines()
            .filter(|s| s.contains("<<<") && s.contains(">>>"))
            .collect::<String>();
        if radio_target_lines.len() > 0 {
            self.get_radio_targets(radio_target_lines.as_str()); // only use radio target related lines to speed up get the radio targets
        }

        let parse_result = document::document_parser(self.config.clone())
            .parse_with_state(input, &mut extra::SimpleState(ParserState::default()));

        if parse_result.has_errors() {
            for e in parse_result.errors() {
                eprintln!("error: {:?}", e);
            }
        }

        ParserResult {
            green: parse_result.into_output().expect("Parse failed"),
        }
    }
}

#[macro_export]
macro_rules! token {
    ($kind:expr , $value:expr) => {
        $crate::parser::NodeOrToken::<$crate::parser::GreenNode, $crate::parser::GreenToken>::Token(
            $crate::parser::GreenToken::new($kind.into(), $value),
        )
    };
}

#[macro_export]
macro_rules! node {
    ($kind:expr , $value:expr) => {
        $crate::parser::NodeOrToken::<$crate::parser::GreenNode, $crate::parser::GreenToken>::Node(
            $crate::parser::GreenNode::new($kind.into(), $value),
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_doc_01() {
        let input = "* 标题1\n 测试\n** 标题1.1\n测试\n测试\ntest\n*** 1.1.1 title\nContent\n* Title\nI have a dream\n"; // (signal: 11, SIGSEGV: invalid memory reference)
        // let input = "* 标题1\n 测试\n* 标";
        // let input = "* 标题1\n 测试\n* ba\n"; // (signal: 6, SIGABRT: process abort signal)
        let mut parser = OrgParser::new(config::OrgParserConfig::default());
        let syntax_node = parser.parse(input).syntax();
        let answer = r###"Document@0..97
  HeadingSubtree@0..74
    HeadingRow@0..10
      HeadingRowStars@0..1 "*"
      Whitespace@1..2 " "
      HeadingRowTitle@2..9
        Text@2..9 "标题1"
      Newline@9..10 "\n"
    Section@10..18
      Paragraph@10..18
        Text@10..18 " 测试\n"
    HeadingSubtree@18..74
      HeadingRow@18..31
        HeadingRowStars@18..20 "**"
        Whitespace@20..21 " "
        HeadingRowTitle@21..30
          Text@21..30 "标题1.1"
        Newline@30..31 "\n"
      Section@31..50
        Paragraph@31..50
          Text@31..50 "测试\n测试\ntest\n"
      HeadingSubtree@50..74
        HeadingRow@50..66
          HeadingRowStars@50..53 "***"
          Whitespace@53..54 " "
          HeadingRowTitle@54..65
            Text@54..65 "1.1.1 title"
          Newline@65..66 "\n"
        Section@66..74
          Paragraph@66..74
            Text@66..74 "Content\n"
  HeadingSubtree@74..97
    HeadingRow@74..82
      HeadingRowStars@74..75 "*"
      Whitespace@75..76 " "
      HeadingRowTitle@76..81
        Text@76..81 "Title"
      Newline@81..82 "\n"
    Section@82..97
      Paragraph@82..97
        Text@82..97 "I have a dream\n"
"###;
        assert_eq!(format!("{syntax_node:#?}"), answer);
    }

    #[test]
    fn test_doc_02() {
        let input = "* 标题1\na";
        let mut parser = OrgParser::new(config::OrgParserConfig::default());
        let syntax_node = parser.parse(input).syntax();
        assert_eq!(
            format!("{syntax_node:#?}"),
            r##"Document@0..11
  HeadingSubtree@0..11
    HeadingRow@0..10
      HeadingRowStars@0..1 "*"
      Whitespace@1..2 " "
      HeadingRowTitle@2..9
        Text@2..9 "标题1"
      Newline@9..10 "\n"
    Section@10..11
      Paragraph@10..11
        Text@10..11 "a"
"##
        );
    }
}

// todo: test of radio link
