//! Parser of org-mode
mod common;
mod element;
pub(crate) mod object;
mod org_file;
pub(crate) mod syntax;

use chrono::prelude::*;
use chumsky::prelude::*;
use rowan::{GreenNode, GreenNodeBuilder, GreenToken, NodeOrToken, WalkEvent};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use syntax::OrgSyntaxKind;
use syntax::SyntaxNode;
use tracing;

pub mod config;

type NT = NodeOrToken<GreenNode, GreenToken>;
type OSK = OrgSyntaxKind;
type MyState = extra::SimpleState<ParserState>;
type MyExtra<'a, C> = extra::Full<Rich<'a, char>, MyState, C>;

#[derive(Clone, Debug)]
pub struct ParserState {
    radio_targets: HashSet<String>,
}

impl Default for ParserState {
    fn default() -> Self {
        Self {
            radio_targets: HashSet::new(),
        }
    }
}

impl ParserState {
    fn new(radio_targets: HashSet<String>) -> Self {
        Self { radio_targets }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ParserResult {
    green: NodeOrToken<GreenNode, GreenToken>,
}

// get text from syntax node `e`
pub fn get_text(e: &SyntaxNode) -> String {
    let mut text = String::new();
    let mut preorder = e.preorder_with_tokens();
    while let Some(event) = preorder.next() {
        match event {
            WalkEvent::Enter(element) => {
                if let Some(token) = element.as_token() {
                    text.push_str(token.text());
                }
            }
            _ => {}
        }
    }
    text
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

    // get keyword and macro_tempalte(name -> template)
    fn get_keyword_and_macro_template(
        &self,
        syntax_tree: &SyntaxNode,
    ) -> (HashMap<String, String>, HashMap<String, String>) {
        let mut keyword = std::collections::HashMap::<String, String>::new();
        let mut macro_template = std::collections::HashMap::<String, String>::new();

        let mut preorder = syntax_tree.preorder();
        while let Some(event) = preorder.next() {
            match event {
                WalkEvent::Enter(element) => {
                    if element.kind() == OSK::Keyword {
                        let key = element
                            .first_child_by_kind(&|e| e == OSK::KeywordKey)
                            .expect("must have KeywordKey")
                            .children_with_tokens()
                            .map(|e| e.as_token().expect("todo").text().to_string())
                            .collect::<String>()
                            .to_ascii_uppercase();

                        let value = element
                            .first_child_by_kind(&|e| e == OSK::KeywordValue)
                            .expect("must have KeywordValue")
                            .children_with_tokens()
                            .map(|e| {
                                if let Some(node) = e.as_node() {
                                    get_text(&node)
                                } else {
                                    e.as_token().expect("todo").text().to_string()
                                }
                            })
                            .collect::<String>()
                            .trim()
                            .to_string();

                        if key == "MACRO" {
                            if let Some((name, template)) =
                                value.split_once(|c: char| c.is_whitespace())
                            {
                                macro_template.insert(
                                    name.to_ascii_uppercase().to_string(),
                                    template.trim().to_string(),
                                ); // overwrite here
                            }
                        } else {
                            if keyword.contains_key(&key) {
                                keyword
                                    .get_mut(&key)
                                    .expect("has value")
                                    .push_str(&format!(" {value}"))
                            } else {
                                keyword.insert(key, value);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        tracing::debug!("keyword collected: {:#?}", keyword);
        tracing::debug!("macro collected: {:#?}", macro_template);
        (keyword, macro_template)
    }

    fn expand_macro_template(template: &String, args: Vec<String>) -> String {
        match args.len() {
            0 => template.clone(),
            1 => template.replace("$1", &args[0]),
            2 => template.replace("$1", &args[0]).replace("$2", &args[1]),
            3 => template
                .replace("$1", &args[0])
                .replace("$2", &args[1])
                .replace("$3", &args[2]),
            4 => template
                .replace("$1", &args[0])
                .replace("$2", &args[1])
                .replace("$3", &args[2])
                .replace("$4", &args[3]),
            5 => template
                .replace("$1", &args[0])
                .replace("$2", &args[1])
                .replace("$3", &args[2])
                .replace("$4", &args[3])
                .replace("$5", &args[4]),
            6 => template
                .replace("$1", &args[0])
                .replace("$2", &args[1])
                .replace("$3", &args[2])
                .replace("$4", &args[3])
                .replace("$5", &args[4])
                .replace("$6", &args[5]),
            7 => template
                .replace("$1", &args[0])
                .replace("$2", &args[1])
                .replace("$3", &args[2])
                .replace("$4", &args[3])
                .replace("$5", &args[4])
                .replace("$6", &args[5])
                .replace("$7", &args[6]),

            8 => template
                .replace("$1", &args[0])
                .replace("$2", &args[1])
                .replace("$3", &args[2])
                .replace("$4", &args[3])
                .replace("$5", &args[4])
                .replace("$6", &args[5])
                .replace("$7", &args[6])
                .replace("$8", &args[7]),

            9 => template
                .replace("$1", &args[0])
                .replace("$2", &args[1])
                .replace("$3", &args[2])
                .replace("$4", &args[3])
                .replace("$5", &args[4])
                .replace("$6", &args[5])
                .replace("$7", &args[6])
                .replace("$8", &args[7])
                .replace("$9", &args[8]),
            _ => {
                panic!("only <= 9 arguments are supported in #+macro definition")
            }
        }
    }

    // Rebuild syntax(red) tree using macro template
    // - builder:
    // - node: input node
    // - k2v: used in some macros such as title,author,email or keyword
    // - marcro_template:
    // - input_file: to get meda data such as modified_time
    fn rebuild_with_macro_updates<P: AsRef<Path>>(
        builder: &mut GreenNodeBuilder,
        node: &SyntaxNode,
        k2v: &HashMap<String, String>,
        macro_template: &HashMap<String, String>,
        input_file: P,
    ) {
        let input_file_path = input_file.as_ref();
        let metadata = fs::metadata(input_file_path).expect("todo");
        let modified_time = metadata.modified().expect("todo");
        let modified_time: DateTime<Local> = modified_time.clone().into();
        let input_file_name = input_file_path.file_name().expect("file name");

        builder.start_node(node.kind().into());

        if node.kind() == OSK::Macro {
            let name = node
                .first_child_or_token_by_kind(&|s| s == OSK::MacroName)
                .expect("must have one MacroName")
                .as_token()
                .unwrap()
                .text()
                .to_ascii_uppercase()
                .to_string();
            let args = match node.first_child_or_token_by_kind(&|s| s == OSK::MacroArgs) {
                Some(e) => e
                    .as_token()
                    .expect("todo")
                    .text()
                    .split(",")
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>(),
                None => vec![],
            };
            match name.as_str() {
                "TITLE" | "AUTHOR" | "EMAIL" => {
                    if let Some(keyword_value_expanded) = k2v.get(&name) {
                        builder.start_node(OSK::Macro.into());
                        builder.token(OSK::Text.into(), keyword_value_expanded);
                        builder.finish_node();
                    }
                }
                "DATE" => {
                    if let Some(keyword_value_expanded) = k2v.get(&name) {
                        builder.start_node(OSK::Macro.into());

                        if args.len() > 0 {
                            let args = args.join("");
                            let ts_parser = object::timestamp::FlexibleDateTimeParser::new();
                            let ts = ts_parser.parse(
                                &keyword_value_expanded
                                    [1..keyword_value_expanded.chars().count() - 1],
                            );
                            if ts.is_ok() {
                                let z = ts.unwrap();
                                let z = z.format(&args).to_string();
                                builder.token(OSK::Text.into(), &z);
                            }
                        } else {
                            builder.token(OSK::Text.into(), keyword_value_expanded);
                        }
                        builder.finish_node();
                    }
                }
                "KEYWORD" => {
                    let args = args.join("").to_ascii_uppercase();
                    if let Some(args_keyword_value_expanded) = k2v.get(&args) {
                        builder.start_node(OSK::Macro.into());
                        builder.token(OSK::Text.into(), args_keyword_value_expanded);
                        builder.finish_node();
                    }
                }

                "MODIFICATION-TIME" => {
                    let args = args.join("");
                    builder.start_node(OSK::Macro.into());
                    let modified_ts = modified_time.format(&args).to_string();
                    builder.token(OSK::Text.into(), &modified_ts);
                    builder.finish_node();
                }

                "INPUT-FILE" => {
                    builder.start_node(OSK::Macro.into());
                    builder.token(
                        OSK::Text.into(),
                        input_file_name.to_str().expect("filename"),
                    );
                    builder.finish_node();
                }

                "RESULTS" => {
                    let args = args.join("");
                    builder.start_node(OSK::Macro.into());
                    builder.token(OSK::Text.into(), &args);
                    builder.finish_node();
                }

                "TIME" => {
                    let args = args.join("");
                    let now: DateTime<Local> = Local::now();
                    let export_ts = now.format(&args).to_string();
                    builder.start_node(OSK::Macro.into());
                    builder.token(OSK::Text.into(), &export_ts);
                    builder.finish_node();
                }

                macro_name if macro_template.contains_key(macro_name) => {
                    let template = macro_template.get(macro_name).expect("get template");
                    builder.start_node(OSK::Macro.into());
                    builder.token(
                        OSK::Text.into(),
                        &Self::expand_macro_template(&template, args),
                    );
                    builder.finish_node();
                }

                _ => {}
            }
        } else {
            for child in node.children_with_tokens() {
                match child {
                    NodeOrToken::Node(child_node) => {
                        Self::rebuild_with_macro_updates(
                            builder,
                            &child_node,
                            k2v,
                            macro_template,
                            input_file_path,
                        );
                    }
                    NodeOrToken::Token(token) => {
                        builder.token(token.kind().into(), token.text());
                    }
                }
            }
        }
        builder.finish_node();
    }

    fn expand_macro<P: AsRef<Path>>(
        &self,
        syntax_tree: &SyntaxNode,
        k2v: &HashMap<String, String>,
        macro_template: &HashMap<String, String>,
        input_file: P,
    ) -> String {
        let mut builder = GreenNodeBuilder::new();
        Self::rebuild_with_macro_updates(
            &mut builder,
            syntax_tree,
            k2v,
            macro_template,
            input_file,
        );
        let syntax_tree_with_macro_expanded = SyntaxNode::new_root(builder.finish());
        let preprocessed_text = get_text(&syntax_tree_with_macro_expanded);
        preprocessed_text
    }

    // Get radio target from RadioTarget defintion in pattern of <<<CONTENTS>>>
    // Note: use SimpleState, NOT RolbackState
    fn get_radio_targets(&self, input: &str) -> HashSet<String> {
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
        tracing::debug!("radio tagets collected: '{}'", radio_targets_text.join("|"));

        let mut targets = HashSet::new();
        for e in radio_targets_text {
            targets.insert(e);
        }
        targets
    }

    pub fn parse<P: AsRef<Path>>(&self, input_file: P) -> SyntaxNode {
        let path = input_file.as_ref();

        let input = &fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("Failed to read {:?}: {}", path, e));

        // radio_target <- "<<<" CONTENTS  ">>>", CONTENTS doest't contain \n, thus we can filter line by line
        // only use radio target related lines to speed up get the radio targets
        let radio_target_lines = input
            .lines()
            .filter(|s| s.contains("<<<") && s.contains(">>>"))
            .collect::<String>();
        let radio_targets = if radio_target_lines.len() > 0 {
            self.get_radio_targets(radio_target_lines.as_str())
        } else {
            HashSet::new()
        };

        let n_macro_reference = input
            .lines()
            .filter(|s| s.contains("{{{") && s.contains("}}}"))
            .count();
        let input_preprocessed = if n_macro_reference > 0 {
            // get the preprocessed input(macros are expanded) from raw input

            tracing::debug!(n_macro_reference, "preprocess needed");
            // parse raw input to get syntax(red) tree
            let parse_result_first_round = org_file::org_file_parser(self.config.clone())
                .parse_with_state(
                    input,
                    &mut extra::SimpleState(ParserState::new(radio_targets.clone())),
                );
            if parse_result_first_round.has_errors() {
                for e in parse_result_first_round.errors() {
                    tracing::error!("{:?}", e);
                }
            }
            let parse_result_first_round = ParserResult {
                green: parse_result_first_round
                    .into_output()
                    .expect("Parse failed in first round"),
            };
            let mut syntax_tree_first_round = parse_result_first_round.syntax();

            // get keyword and macro_template from sytnax tree
            let (k2v, macro_template) =
                self.get_keyword_and_macro_template(&syntax_tree_first_round);

            // get preprocessed_text
            &self.expand_macro(
                &mut syntax_tree_first_round,
                &k2v,
                &macro_template,
                input_file,
            )
        } else {
            tracing::trace!("preprocess ignored");
            input
        };
        tracing::trace!("preprocess done");

        let parse_result = org_file::org_file_parser(self.config.clone()).parse_with_state(
            input_preprocessed,
            &mut extra::SimpleState(ParserState::new(radio_targets)),
        );
        tracing::trace!("parse done");

        if parse_result.has_errors() {
            for e in parse_result.errors() {
                tracing::error!("{:?}", e);
            }
        }
        // tracing::trace!("{:#?}", parse_result);

        let green_tree = parse_result
            .into_output()
            .expect("Parse failed in into_output()")
            .into_node()
            .unwrap();

        let syntax_tree = SyntaxNode::new_root(green_tree); // i.e, red tree

        syntax_tree
    }
}

#[macro_export]
macro_rules! token {
    ($kind:expr , $value:expr) => {
        $crate::compiler::parser::NodeOrToken::<
            $crate::compiler::parser::GreenNode,
            $crate::compiler::parser::GreenToken,
        >::Token($crate::compiler::parser::GreenToken::new(
            $kind.into(),
            $value,
        ))
    };
}

#[macro_export]
macro_rules! node {
    ($kind:expr , $value:expr) => {
        $crate::compiler::parser::NodeOrToken::<
            $crate::compiler::parser::GreenNode,
            $crate::compiler::parser::GreenToken,
        >::Node($crate::compiler::parser::GreenNode::new(
            $kind.into(),
            $value,
        ))
    };
}
