//! Parser of org-mode

pub(crate) mod syntax;

mod common;
mod document;
mod element;
mod footnote_definition;
mod heading;
mod latex_environment;
mod list;
pub(crate) mod object;
mod paragraph;
mod section;

use crate::parser::syntax::OrgSyntaxKind;

use chumsky::inspector::RollbackState;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenToken, NodeOrToken, WalkEvent};
use std::ops::Range;

use crate::parser::syntax::SyntaxNode;

/// S2: Return nodes whose number if Smaller than Two
/// - 0: None
/// - 1: Single(Node)
/// - 2: Double(Node, Node)
#[derive(Debug)]
pub enum S2 {
    None,                                       // zero
    Single(NodeOrToken<GreenNode, GreenToken>), // one
    Double(
        NodeOrToken<GreenNode, GreenToken>,
        NodeOrToken<GreenNode, GreenToken>,
    ), // two
}

// 上下文状态：当前解析的标题级别
#[derive(Clone, Debug)]
pub(crate) struct ParserState {
    level_stack: Vec<usize>,
    block_type: String,     // begin_type, end_type: 两个解析器需要相同的type数据
    latex_env_name: String, // latex \begin{}
    item_indent: Vec<usize>,
    prev_char: Option<char>,             // previous char
    prev_char_backup: Vec<Option<char>>, // previous char backup manually set for later resume back
    radio_targets: Vec<String>,
}

impl Default for ParserState {
    fn default() -> Self {
        Self {
            level_stack: vec![0],
            block_type: String::new(),
            latex_env_name: String::new(),
            item_indent: vec![],
            prev_char: None,
            prev_char_backup: vec![None],
            radio_targets: vec![],
        }
    }
}

impl ParserState {
    fn default_with_radio_targets(radio_targets: Vec<String>) -> Self {
        Self {
            level_stack: vec![0],
            block_type: String::new(),
            latex_env_name: String::new(),
            item_indent: vec![],
            prev_char: None,
            radio_targets: radio_targets,
            prev_char_backup: vec![None],
        }
    }
}

// 表示解析结果的类型，直接包含 GreenNode 和文本信息
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ParserResult {
    // FIXME: use Arc<GreenNode>?
    green: NodeOrToken<GreenNode, GreenToken>,
    text: String, // 作用?
    span: Range<usize>,
}

impl ParserResult {
    pub fn green(&self) -> &GreenNode {
        match &self.green {
            NodeOrToken::Node(e) => e,
            NodeOrToken::Token(_) => todo!(),
        }
    }
    pub fn syntax(&self) -> SyntaxNode {
        match &self.green {
            NodeOrToken::Node(e) => SyntaxNode::new_root(e.clone()),
            NodeOrToken::Token(_) => todo!(),
        }
    }
}

#[allow(dead_code)]
pub struct OrgParser {
    pub config: OrgConfig,
}

impl OrgParser {
    pub fn new(config: OrgConfig) -> Self {
        OrgParser { config }
    }

    pub fn get_radio_targets(&self, input: &str) -> Vec<String> {
        let radio_targets: Vec<NodeOrToken<GreenNode, GreenToken>> = object::objects_parser()
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
        radio_targets_text
    }

    pub fn parse(&mut self, input: &str) -> ParserResult {
        let radio_targets = self.get_radio_targets(input);
        println!("radio_targets={:?}", radio_targets);

        let parse_result = document::document_parser().parse_with_state(
            input,
            &mut RollbackState(ParserState::default_with_radio_targets(radio_targets)),
        );

        // let parse_result = document::document_parser()
        //     .parse_with_state(input, &mut RollbackState(ParserState::default()));

        if parse_result.has_errors() {
            for e in parse_result.errors() {
                println!("{:?}", e);
            }
        }

        parse_result.into_output().expect("Parse failed")
    }
}

// FIXME: config for parser and render?
#[allow(dead_code)]
pub struct OrgConfig {
    pub todo_keywords: Vec<String>,
    // pub dual_keywords: Vec<String>,
    // // org-element-parsed-keywords
    // pub parsed_keywords: Vec<String>,

    // // see org-element-affiliated-keywords
    // pub affiliated_keywords: Vec<String>,
}

impl Default for OrgConfig {
    fn default() -> Self {
        OrgConfig {
            // parsed_keywords: vec!["CAPTION".to_string()],
            // dual_keywords: vec!["CAPTION".to_string(), "RESULTS".to_string()],
            todo_keywords: vec!["TODO".to_string(), "DONE".to_string()],
            // affiliated_keywords: vec![
            //     "CAPTION".to_string(),
            //     "DATA".to_string(),
            //     "HEADER".to_string(),
            //     "HEADERS".to_string(),
            //     "LABEL".to_string(),
            //     "NAME".to_string(),
            //     "PLOT".to_string(),
            //     "RESNAME".to_string(),
            //     "RESULT".to_string(),
            //     "RESULTS".to_string(),
            //     "SOURCE".to_string(),
            //     "SRCNAME".to_string(),
            //     "TBLNAME".to_string(),
            // ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_headlines() {
        let input = "* 标题1\n 测试\n** 标题1.1\n测试\n测试\ntest\n*** 1.1.1 title\nContent\n* Title\nI have a dream\n";
        let mut parser = OrgParser::new(OrgConfig::default());
        let syntax_node = parser.parse(input).syntax();
        let answer = r###"Document@0..97
  HeadingSubtree@0..74
    HeadingRow@0..10
      HeadingRowStars@0..1 "*"
      Whitespace@1..2 " "
      HeadingRowTitle@2..9 "标题1"
      Newline@9..10 "\n"
    Section@10..18
      Paragraph@10..18
        Text@10..18 " 测试\n"
    HeadingSubtree@18..74
      HeadingRow@18..31
        HeadingRowStars@18..20 "**"
        Whitespace@20..21 " "
        HeadingRowTitle@21..30 "标题1.1"
        Newline@30..31 "\n"
      Section@31..50
        Paragraph@31..50
          Text@31..50 "测试\n测试\ntest\n"
      HeadingSubtree@50..74
        HeadingRow@50..66
          HeadingRowStars@50..53 "***"
          Whitespace@53..54 " "
          HeadingRowTitle@54..65 "1.1.1 title"
          Newline@65..66 "\n"
        Section@66..74
          Paragraph@66..74
            Text@66..74 "Content\n"
  HeadingSubtree@74..97
    HeadingRow@74..82
      HeadingRowStars@74..75 "*"
      Whitespace@75..76 " "
      HeadingRowTitle@76..81 "Title"
      Newline@81..82 "\n"
    Section@82..97
      Paragraph@82..97
        Text@82..97 "I have a dream\n"
"###;
        println!("{}", format!("{syntax_node:#?}"));
        println!("{}", answer);

        assert_eq!(format!("{syntax_node:#?}"), answer);
    }

    #[test]
    fn test_basic_headlines_v2() {
        let input = "* 标题1\na";
        let mut parser = OrgParser::new(OrgConfig::default());
        let syntax_node = parser.parse(input).syntax();
        println!("{}", format!("{syntax_node:#?}"));
    }
}

// todo: test of radio link
