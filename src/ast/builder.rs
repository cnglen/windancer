//! ASTBuilder builds the AST from SyntaxNode(RedTree/SyntaxTree)
//! - using the private inner `Converter` to keep the **public `AstBuilder` API simple**
//! - Converter::convert()
//!   - convert_document()
//!    - convert_section()
//!      - convert_element()
//!        - convert_paragraph()
//!        - convert_...
//!  - convert_heading_subtree()
//!      - convert_section()
//!      - convert_heading_subtree(), call self recursively
//!
//! Element: Each kind of ast::Element has a convert_xxx_element(), which convert SyntaxNode to ast::Element
//! Object: `convert_object()``, Each kind of ast::Object has a convert_xxx_object(), which convert SyntaxNode to ast::Object

use rowan::GreenNode;
use std::collections::HashMap;

use crate::ast::element::{
    CenterBlock, CommentBlock, Document, Drawer, Element, ExampleBlock, ExportBlock,
    FootnoteDefinition, HeadingSubtree, HorizontalRule, Item, Keyword, LatexEnvironment, List,
    ListType, Paragraph, QuoteBlock, Section, SpecialBlock, SrcBlock, Table, TableRow,
    TableRowType, VerseBlock,
};
use crate::ast::error::AstError;
use crate::ast::object::{Object, TableCell};
use crate::parser::syntax::{OrgSyntaxKind, SyntaxElement, SyntaxNode, SyntaxToken};

pub struct AstBuilder;

impl AstBuilder {
    pub fn new() -> Self {
        AstBuilder {}
    }

    pub fn build(&self, root: &SyntaxNode) -> Result<Document, AstError> {
        Converter::new().convert(root)
    }
}

// 内部转换器，不公开
struct Converter {
    // 必要的状态字段
    footnote_label_to_rids: HashMap<String, Vec<usize>>,
    n_anonymous_label: usize,
    footnote_label_to_nid: HashMap<String, usize>,
    footnote_definitions: Vec<FootnoteDefinition>,
}

impl Converter {
    fn new() -> Self {
        Self {
            footnote_label_to_rids: HashMap::new(),
            n_anonymous_label: 0,
            footnote_label_to_nid: HashMap::new(),
            footnote_definitions: vec![],
            /* 初始化状态 */
        }
    }

    fn convert(&mut self, root: &SyntaxNode) -> Result<Document, AstError> {
        self.convert_document(root)
    }

    /// 在第一个标题处分割节点列表
    fn split_at_first_heading(&self, nodes: Vec<SyntaxNode>) -> (Vec<SyntaxNode>, Vec<SyntaxNode>) {
        let mut zeroth_nodes = Vec::new();
        let mut remaining_nodes = Vec::new();

        if let Some(first_node) = nodes.get(0) {
            match first_node.kind() {
                OrgSyntaxKind::HeadingSubtree => {
                    remaining_nodes.push(first_node.clone());
                }
                OrgSyntaxKind::Section => {
                    zeroth_nodes.push(first_node.clone());
                }
                _ => {}
            }
        }

        for node in nodes.into_iter().skip(1) {
            remaining_nodes.push(node);
        }

        (zeroth_nodes, remaining_nodes)
    }

    // 内部转换方法可以访问状态
    fn convert_document(&mut self, node: &SyntaxNode) -> Result<Document, AstError> {
        // 使用内部状态进行转换
        let children = node.children().collect::<Vec<_>>();
        let (zeroth_nodes, remainig_nodes) = self.split_at_first_heading(children);

        let mut zeroth_section = None;
        if !zeroth_nodes.is_empty() {
            zeroth_section = Some(self.convert_section(&zeroth_nodes[0])?);
        }

        let mut heading_subtrees = Vec::new();
        for child in remainig_nodes {
            match child.kind() {
                OrgSyntaxKind::HeadingSubtree => {
                    heading_subtrees.push(self.convert_heading_subtree(&child)?);
                }
                _ => {
                    // 处理其他节点类型或忽略未知节点
                    eprintln!("Only HeadingSubtree suppoted in Document's children!");
                    std::process::exit(1);
                }
            }
        }

        self.footnote_definitions.sort_by(|a, b| a.nid.cmp(&b.nid));

        Ok(Document {
            syntax: node.clone(),
            heading_subtrees: heading_subtrees,
            zeroth_section: zeroth_section,
            footnote_definitions: self.footnote_definitions.clone(),
        })
    }

    fn convert_heading_subtree(&mut self, node: &SyntaxNode) -> Result<HeadingSubtree, AstError> {
        // let level = Self::extract_heading_level(node)?;
        // let title = Self::extract_text_content(node)?;
        // let children = Self::convert_children(node)?;

        let mut subtrees = vec![];
        let mut level: u8 = 0;
        let mut section = None;
        let mut is_commented = false;
        let mut priority = None;
        let mut keyword = None;
        let mut title = None;
        let mut tags = vec![];
        for child in node.children() {
            match child.kind() {
                OrgSyntaxKind::Section => match self.convert_section(&child) {
                    Ok(s) => section = Some(s),
                    Err(_) => {}
                },
                OrgSyntaxKind::HeadingRow => {
                    for c in child.children_with_tokens() {
                        match c.kind() {
                            OrgSyntaxKind::HeadingRowStars => {
                                level = c.as_token().unwrap().text().len() as u8;
                            }
                            OrgSyntaxKind::HeadingRowKeywordTodo => {
                                keyword = Some("TODO".to_string())
                            }
                            OrgSyntaxKind::HeadingRowKeywordDone => {
                                keyword = Some("DONE".to_string())
                            }
                            OrgSyntaxKind::HeadingRowPriority => {
                                match c
                                    .as_node()
                                    .unwrap()
                                    .first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::Text)
                                {
                                    Some(p) => {
                                        priority = Some(p.as_token().unwrap().text().to_string());
                                    }
                                    _ => {}
                                }
                            }
                            OrgSyntaxKind::HeadingRowComment => {
                                is_commented = true;
                            }
                            OrgSyntaxKind::HeadingRowTitle => {
                                title = Some(c.as_token().unwrap().text().to_string())
                            }
                            OrgSyntaxKind::HeadingRowTags => {
                                let tc = c.as_node().unwrap();
                                for child in tc.children_with_tokens() {
                                    match child.kind() {
                                        OrgSyntaxKind::HeadingRowTag => {
                                            tags.push(child.as_token().unwrap().text().to_string());
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                OrgSyntaxKind::HeadingSubtree => match self.convert_heading_subtree(&child) {
                    Ok(cc) => {
                        subtrees.push(cc);
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        Ok(HeadingSubtree {
            syntax: node.clone(),
            section: section,
            level: level,
            is_commented: is_commented,
            keyword: keyword,
            priority: priority,
            tags: tags,
            title: title,
            sub_heading_subtrees: subtrees,
        })
    }

    fn convert_section(&mut self, node: &SyntaxNode) -> Result<Section, AstError> {
        let mut elements = vec![];

        for child in node.children() {
            if let Ok(element) = self.convert_element(&child) {
                elements.push(element);
            }
        }

        Ok(Section {
            syntax: node.clone(),
            elements: elements,
        })
    }

    /// Conver SyntaxTree(RedTree) to ast::Element
    fn convert_element(&mut self, node: &SyntaxNode) -> Result<Element, AstError> {
        match node.kind() {
            OrgSyntaxKind::Paragraph => Ok(Element::Paragraph(self.convert_paragraph(&node)?)),

            OrgSyntaxKind::Drawer => Ok(Element::Drawer(self.convert_drawer(&node)?)),

            OrgSyntaxKind::Table => Ok(Element::Table(self.convert_table(&node)?)),

            OrgSyntaxKind::CenterBlock => {
                Ok(Element::CenterBlock(self.convert_center_block(&node)?))
            }

            OrgSyntaxKind::QuoteBlock => Ok(Element::QuoteBlock(self.convert_quote_block(&node)?)),

            OrgSyntaxKind::SpecialBlock => {
                Ok(Element::SpecialBlock(self.convert_special_block(&node)?))
            }

            OrgSyntaxKind::ExampleBlock => {
                Ok(Element::ExampleBlock(self.convert_example_block(&node)?))
            }
            OrgSyntaxKind::VerseBlock => Ok(Element::VerseBlock(self.convert_verse_block(&node)?)),
            OrgSyntaxKind::SrcBlock => Ok(Element::SrcBlock(self.convert_src_block(&node)?)),
            OrgSyntaxKind::CommentBlock => {
                Ok(Element::CommentBlock(self.convert_comment_block(&node)?))
            }
            OrgSyntaxKind::ExportBlock => {
                Ok(Element::ExportBlock(self.convert_export_block(&node)?))
            }

            OrgSyntaxKind::List => Ok(Element::List(self.convert_list(&node)?)),
            OrgSyntaxKind::Keyword => Ok(Element::Keyword(self.convert_keyword(&node)?)),

            OrgSyntaxKind::HorizontalRule => Ok(Element::HorizontalRule(
                self.convert_horizontal_rule(&node)?,
            )),

            OrgSyntaxKind::FootnoteDefinition => Ok(Element::FootnoteDefinition(
                self.convert_footnote_definition(&node)?,
            )),

            OrgSyntaxKind::LatexEnvironment => Ok(Element::LatexEnvironment(
                self.convert_latex_environment(&node)?,
            )),

            _ => Err(AstError::UnknownNodeType {
                kind: node.kind(),
                position: None,
            }),
        }
    }

    // conver to object
    fn convert_object(
        &mut self,
        node_or_token: &SyntaxElement,
    ) -> Result<Option<Object>, AstError> {
        match node_or_token.kind() {
            OrgSyntaxKind::Text => {
                // Ok(Some(Object::Text(node_or_token.as_token().unwrap().text().to_string())));
                Ok(self.convert_text(node_or_token.as_token().unwrap())?)
            }

            OrgSyntaxKind::Bold => {
                Ok(self.convert_markup("Bold", node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Italic => {
                Ok(self.convert_markup("Italic", node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Underline => {
                Ok(self.convert_markup("Underline", node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Strikethrough => {
                Ok(self.convert_markup("Strikethrough", node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Verbatim => {
                Ok(self.convert_markup("Verbatim", node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Code => {
                Ok(self.convert_markup("Code", node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Link => Ok(self.convert_link(node_or_token.as_node().unwrap())?),

            OrgSyntaxKind::FootnoteReference => {
                Ok(self.convert_footnote_reference(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Entity => Ok(self.convert_entity(node_or_token.as_node().unwrap())?),

            OrgSyntaxKind::Macro => Ok(self.convert_macro(node_or_token.as_node().unwrap())?),

            OrgSyntaxKind::LineBreak => Ok(Some(Object::LineBreak)),

            OrgSyntaxKind::LatexFragment => {
                Ok(self.convert_latex_fragment(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Whitespace => Ok(Some(Object::Whitespace(String::from(" ")))),

            OrgSyntaxKind::Asterisk => Ok(None),
            OrgSyntaxKind::BlankLine => Ok(None),

            _ => Err(AstError::UnknownNodeType {
                kind: node_or_token.kind(),
                position: None,
            }),
        }
    }

    // element.paragraph
    fn convert_paragraph(&mut self, node: &SyntaxNode) -> Result<Paragraph, AstError> {
        let mut objects = vec![];
        // println!("convert_paragraph: {:#?}", node.children());
        for child in node.children_with_tokens() {
            let t = self.convert_object(&child);
            match t {
                Ok(Some(e)) => {
                    objects.push(e);
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("error={:?}", e);
                }
            }
        }
        Ok(Paragraph {
            syntax: node.clone(),
            objects: objects,
        })
    }

    // element.drawrer
    fn convert_drawer(&mut self, node: &SyntaxNode) -> Result<Drawer, AstError> {
        let mut name = String::new();
        let mut contents: Vec<Element> = vec![];

        for child in node.children_with_tokens() {
            match child.kind() {
                OrgSyntaxKind::DrawerBegin => {
                    name = child
                        .as_node()
                        .unwrap()
                        .children_with_tokens()
                        .filter(|e| e.kind() == OrgSyntaxKind::Text)
                        .map(|e| e.as_token().unwrap().text().to_string())
                        .collect::<Vec<String>>()
                        .join("");
                }
                OrgSyntaxKind::DrawerContent => {
                    for grandson in child.as_node().unwrap().children() {
                        if let Ok(element) = self.convert_element(&grandson) {
                            contents.push(element);
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(Drawer {
            syntax: node.clone(),
            name,
            contents,
        })
    }

    // element.table
    fn convert_table(&mut self, node: &SyntaxNode) -> Result<Table, AstError> {
        let name = None;
        let caption = None;
        let header = None;
        let separator = None;
        let mut rows = vec![];

        for row in node.children() {
            match row.kind() {
                OrgSyntaxKind::TableStandardRow => {
                    rows.push(self.convert_table_row(&row, TableRowType::Data)?);
                }
                OrgSyntaxKind::TableRuleRow => {
                    rows.push(self.convert_table_row(&row, TableRowType::Rule)?);
                }
                _ => {}
            }
        }
        Ok(Table {
            syntax: node.clone(),
            name,
            caption,
            header,
            separator,
            rows,
        })
    }

    // element.table_row
    fn convert_table_row(
        &mut self,
        node: &SyntaxNode,
        row_type: TableRowType,
    ) -> Result<TableRow, AstError> {
        let cells = node
            .children()
            .filter(|e| e.kind() == OrgSyntaxKind::TableCell)
            .map(|e| self.convert_table_cell(&e))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect::<Vec<_>>();

        Ok(TableRow {
            syntax: node.clone(),
            cells,
            row_type,
        })
    }

    // object.table_cell
    fn convert_table_cell(&mut self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let contents = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Text)
            .map(|e| self.convert_object(&e))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect();

        Ok(Some(Object::TableCell(TableCell { contents })))
    }

    // convert markup object
    fn convert_markup(
        &mut self,
        markup_type: &str,
        node: &SyntaxNode,
    ) -> Result<Option<Object>, AstError> {
        let mut objects = vec![];
        for child in node.children_with_tokens() {
            match self.convert_object(&child) {
                Ok(Some(object)) => {
                    objects.push(object);
                }
                _ => {}
            }
        }
        match markup_type {
            "Bold" => Ok(Some(Object::Bold(objects))),
            "Italic" => Ok(Some(Object::Italic(objects))),
            "Underline" => Ok(Some(Object::Underline(objects))),
            "Strikethrough" => Ok(Some(Object::Strikethrough(objects))),
            "Code" => Ok(Some(Object::Code(objects))),
            "Verbatim" => Ok(Some(Object::Verbatim(objects))),
            _ => Ok(Some(Object::Verbatim(objects))), // fixme
        }
    }

    // object.text
    fn convert_text(&self, token: &SyntaxToken) -> Result<Option<Object>, AstError> {
        Ok(Some(Object::Text(token.text().to_string())))
    }

    // object.macro
    fn convert_macro(&self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let name = node
            .first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::MacroName)
            .expect("has name")
            .as_token()
            .unwrap()
            .text()
            .to_string();

        let args_token = node.first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::MacroArgs);

        let arguments = match args_token {
            None => {
                vec![]
            }
            Some(token) => {
                let raw = token.as_token().unwrap().text().to_string();
                let raw = raw.replace(r##"\,"##, "MAGIC_ESCAPED_COMMA");
                let a = raw
                    .split(",")
                    .map(|s| s.trim().replace("MAGIC_ESCAPED_COMMA", ",").to_string())
                    .collect::<Vec<String>>();
                a
            }
        };

        Ok(Some(Object::Macro { name, arguments }))
    }

    // object.link
    fn convert_link(&self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let url = node
            .first_child_by_kind(&|c| c == OrgSyntaxKind::LinkPath)
            .expect("has link")
            .first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::Text)
            .expect("has text")
            .as_token()
            .unwrap()
            .text()
            .to_string();

        let text = if let Some(desc) =
            node.first_child_by_kind(&|c| c == OrgSyntaxKind::LinkDescription)
        {
            Some(
                desc.first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::Text)
                    .expect("has text")
                    .as_token()
                    .unwrap()
                    .text()
                    .to_string(),
            )
        } else {
            None
        };

        Ok(Some(Object::Link { url, text }))
    }

    // object.footnote_reference
    // - generate footnote definition if possible
    fn convert_footnote_reference(
        &mut self,
        node: &SyntaxNode,
    ) -> Result<Option<Object>, AstError> {
        let is_anoymous = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Colon2)
            .count()
            > 0;

        // todo: bold  fixme
        let is_inline = node
            .children_with_tokens()
            .filter(|e| {
                e.kind() == OrgSyntaxKind::FootnoteReferenceLabel
                    || e.kind() == OrgSyntaxKind::FootnoteReferenceDefintion
            })
            .count()
            == 2;

        let (raw_label, raw_definition) = if is_inline {
            let label = node
                .children_with_tokens()
                .filter(|e| e.kind() == OrgSyntaxKind::FootnoteReferenceLabel)
                .nth(0)
                .expect("must have label");
            let definition = node
                .children_with_tokens()
                .filter(|e| e.kind() == OrgSyntaxKind::FootnoteReferenceDefintion)
                .nth(0)
                .expect("must have defintion");
            (Some(label), Some(definition))
        } else if is_anoymous {
            let definition = node
                .children_with_tokens()
                .filter(|e| e.kind() == OrgSyntaxKind::FootnoteReferenceDefintion)
                .nth(0)
                .expect("must have defintion");
            (None, Some(definition))
        } else {
            let label = node
                .children_with_tokens()
                .filter(|e| e.kind() == OrgSyntaxKind::FootnoteReferenceLabel)
                .nth(0)
                .expect("must have label");
            (Some(label), None)
        };

        // println!("{:?}:{:?}", raw_label, raw_definition);

        let label = match raw_label {
            Some(e) => {
                let _label = e.as_token().expect("todo").text().to_string();
                if self.footnote_label_to_rids.contains_key(&_label) {
                    let n_rid = self
                        .footnote_label_to_rids
                        .get(&_label)
                        .expect("todo")
                        .len();
                    self.footnote_label_to_rids
                        .get_mut(&_label)
                        .expect("todo")
                        .push(n_rid + 1);
                } else {
                    self.footnote_label_to_rids.insert(_label.clone(), vec![1]);
                    self.footnote_label_to_nid
                        .insert(_label.clone(), self.footnote_label_to_rids.len());
                }
                _label
            }
            None => {
                let label_generated = format!("anonymous_{}", self.n_anonymous_label);
                self.n_anonymous_label = self.n_anonymous_label + 1;
                self.footnote_label_to_rids
                    .insert(label_generated.clone(), vec![1]);
                self.footnote_label_to_nid
                    .insert(label_generated.clone(), self.footnote_label_to_rids.len());
                label_generated
            }
        };

        if let Some(definition) = raw_definition {
            let green_node = GreenNode::new(OrgSyntaxKind::Paragraph.into(), vec![]);
            let faked_syntax_node = SyntaxNode::new_root(green_node);

            let objects = definition
                .as_node()
                .unwrap()
                .children_with_tokens()
                .map(|e| self.convert_object(&e).unwrap().unwrap())
                .collect::<Vec<_>>();
            let element = Element::Paragraph(Paragraph {
                objects: objects,
                syntax: faked_syntax_node.clone(),
            });

            // definition
            let rids = self.footnote_label_to_rids.get(&label).expect("todo");
            let nid = self.footnote_label_to_nid.get(&label).expect("todo");
            let footnote_definition = FootnoteDefinition {
                label: label.clone(),
                contents: vec![element],
                syntax: faked_syntax_node,
                rids: rids.clone(),
                nid: *nid,
            };

            // println!("footnotedefinition={:?}", footnote_definition);
            self.footnote_definitions.push(footnote_definition);
        }

        Ok(Some(Object::FootnoteReference {
            label: label.clone(),
            nid: *self.footnote_label_to_nid.get(&label).expect("todo"),
            label_rid: self.footnote_label_to_rids.get(&label).expect("todo").len(),
        }))
    }

    // object.link
    fn convert_entity(&self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let n_name = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::EntityName)
            .count();
        let n_space = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Spaces)
            .count();

        let name = if n_name == 1 {
            node.children_with_tokens()
                .filter(|e| e.kind() == OrgSyntaxKind::EntityName)
                .map(|e| e.as_token().expect("todo").text().to_string())
                .collect::<String>()
        } else if n_space == 1 {
            node.children_with_tokens()
                .filter(|e| e.kind() == OrgSyntaxKind::Spaces)
                .map(|e| format!("_{}", e.as_token().expect("todo").text().to_string()))
                .collect::<String>()
        } else {
            String::from("error occured, please fixme")
        };

        Ok(Some(Object::Entity { name }))
    }

    // object.latex_fragment
    fn convert_latex_fragment(&self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let tokens = node.children_with_tokens();
        let display_mode = if tokens
            .clone()
            .filter(|e| e.kind() == OrgSyntaxKind::Dollar2)
            .count()
            == 2
        {
            Some(true)
        } else if tokens
            .clone()
            .filter(|e| {
                e.kind() == OrgSyntaxKind::LeftSquareBracket
                    || e.kind() == OrgSyntaxKind::RightSquareBracket
            })
            .count()
            == 2
        {
            Some(true)
        } else if tokens
            .clone()
            .filter(|e| {
                e.kind() == OrgSyntaxKind::LeftRoundBracket
                    || e.kind() == OrgSyntaxKind::RightRoundBracket
            })
            .count()
            == 2
        {
            Some(false)
        } else if tokens.filter(|e| e.kind() == OrgSyntaxKind::Dollar).count() == 2 {
            Some(false)
        } else {
            // e.g: \enlargethispage{2\baselineskip}
            None
        };

        let content = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Text)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        Ok(Some(Object::LatexFragment {
            content,
            display_mode,
        }))
    }

    // element.center_block
    fn convert_center_block(&mut self, node: &SyntaxNode) -> Result<CenterBlock, AstError> {
        let parameters = None;
        let mut contents = vec![];

        match node.kind() {
            OrgSyntaxKind::CenterBlock => {
                let q = node.first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::BlockBegin);
                for e in node
                    .first_child_by_kind(&|c| c == OrgSyntaxKind::BlockContent)
                    .unwrap()
                    .children()
                {
                    if let Ok(element) = self.convert_element(&e) {
                        contents.push(element);
                    }
                }
            }

            _ => {}
        }

        Ok(CenterBlock {
            syntax: node.clone(),
            parameters: parameters,
            contents: contents,
        })
    }

    // element.quote_block
    fn convert_quote_block(&mut self, node: &SyntaxNode) -> Result<QuoteBlock, AstError> {
        let parameters = None;
        let mut contents = vec![];

        match node.kind() {
            OrgSyntaxKind::QuoteBlock => {
                let q = node.first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::BlockBegin);
                for e in node
                    .first_child_by_kind(&|c| c == OrgSyntaxKind::BlockContent)
                    .unwrap()
                    .children()
                {
                    if let Ok(element) = self.convert_element(&e) {
                        contents.push(element);
                    }
                }
            }
            _ => {}
        }

        Ok(QuoteBlock {
            syntax: node.clone(),
            parameters: parameters,
            contents: contents,
        })
    }

    // element.special_block
    fn convert_special_block(&mut self, node: &SyntaxNode) -> Result<SpecialBlock, AstError> {
        let parameters = None;
        let mut contents = vec![];

        match node.kind() {
            OrgSyntaxKind::SpecialBlock => {
                let q = node.first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::BlockBegin);
                for e in node
                    .first_child_by_kind(&|c| c == OrgSyntaxKind::BlockContent)
                    .expect(format!("no block content found: {:#?}", node).as_str())
                    .children()
                {
                    if let Ok(element) = self.convert_element(&e) {
                        contents.push(element);
                    }
                }
            }
            _ => {}
        }

        Ok(SpecialBlock {
            syntax: node.clone(),
            parameters: parameters,
            contents: contents,
            name: String::from("todo"),
        })
    }

    // element.example_block
    fn convert_example_block(&mut self, node: &SyntaxNode) -> Result<ExampleBlock, AstError> {
        let data = None;
        let mut contents = vec![];

        match node.kind() {
            OrgSyntaxKind::ExampleBlock => {
                for e in node
                    .first_child_by_kind(&|c| c == OrgSyntaxKind::BlockContent)
                    .unwrap()
                    .children_with_tokens()
                {
                    if let Ok(Some(object)) = self.convert_object(&e) {
                        contents.push(object);
                    }
                }
            }

            _ => {}
        }

        Ok(ExampleBlock {
            syntax: node.clone(),
            data: data,
            contents: contents,
        })
    }

    // element.comment_block
    fn convert_comment_block(&mut self, node: &SyntaxNode) -> Result<CommentBlock, AstError> {
        let data = None;
        let mut contents = vec![];

        match node.kind() {
            OrgSyntaxKind::CommentBlock => {
                for e in node
                    .first_child_by_kind(&|c| c == OrgSyntaxKind::BlockContent)
                    .unwrap()
                    .children_with_tokens()
                {
                    if let Ok(Some(object)) = self.convert_object(&e) {
                        contents.push(object);
                    }
                }
            }

            _ => {}
        }

        Ok(CommentBlock {
            syntax: node.clone(),
            data: data,
            contents: contents,
        })
    }

    // element.verse_block
    fn convert_verse_block(&mut self, node: &SyntaxNode) -> Result<VerseBlock, AstError> {
        let data = None;
        let mut contents = vec![];

        match node.kind() {
            OrgSyntaxKind::VerseBlock => {
                for e in node
                    .first_child_by_kind(&|c| c == OrgSyntaxKind::BlockContent)
                    .unwrap()
                    .children_with_tokens()
                {
                    if let Ok(Some(object)) = self.convert_object(&e) {
                        contents.push(object);
                    }
                }
            }

            _ => {}
        }

        Ok(VerseBlock {
            syntax: node.clone(),
            data: data,
            contents: contents,
        })
    }

    // element.src_block
    fn convert_src_block(&mut self, node: &SyntaxNode) -> Result<SrcBlock, AstError> {
        let data = None;
        let mut contents = vec![];

        match node.kind() {
            OrgSyntaxKind::SrcBlock => {
                for e in node
                    .first_child_by_kind(&|c| c == OrgSyntaxKind::BlockContent)
                    .unwrap()
                    .children_with_tokens()
                {
                    if let Ok(Some(object)) = self.convert_object(&e) {
                        contents.push(object);
                    }
                }
            }

            _ => {}
        }

        Ok(SrcBlock {
            syntax: node.clone(),
            data: data,
            contents: contents,
        })
    }

    // element.export_block
    fn convert_export_block(&mut self, node: &SyntaxNode) -> Result<ExportBlock, AstError> {
        let data = None;
        let mut contents = vec![];

        match node.kind() {
            OrgSyntaxKind::ExportBlock => {
                for e in node
                    .first_child_by_kind(&|c| c == OrgSyntaxKind::BlockContent)
                    .unwrap()
                    .children_with_tokens()
                {
                    if let Ok(Some(object)) = self.convert_object(&e) {
                        contents.push(object);
                    }
                }
            }

            _ => {}
        }

        Ok(ExportBlock {
            syntax: node.clone(),
            data: data,
            contents: contents,
        })
    }

    //

    // element.item
    fn convert_item(&mut self, node: &SyntaxNode) -> Result<Item, AstError> {
        let mut bullet = String::new();
        let counter_set = None;
        let checkbox = None;
        let mut tag = None;
        let mut contents = vec![];
        for child in node.children_with_tokens() {
            // println!("{:#?}", child);

            match child.kind() {
                OrgSyntaxKind::ListItemBullet => {
                    bullet = child
                        .as_node()
                        .unwrap()
                        .first_child_or_token_by_kind(&|e| e == OrgSyntaxKind::Text)
                        .unwrap()
                        .as_token()
                        .unwrap()
                        .text()
                        .to_string();
                }

                OrgSyntaxKind::ListItemTag => {
                    tag = Some(
                        child
                            .as_node()
                            .unwrap()
                            .first_child_or_token_by_kind(&|e| e == OrgSyntaxKind::Text)
                            .unwrap()
                            .as_token()
                            .unwrap()
                            .text()
                            .to_string(),
                    );
                }

                // FIXME: ListItemparser
                //
                OrgSyntaxKind::ListItemContent => {
                    for cc in child.as_node().unwrap().children() {
                        if let Ok(element) = self.convert_element(&cc) {
                            contents.push(element);
                        }
                    }
                }

                _ => {}
            }
        }
        // node
        //     .children_with_tokens()
        //     // .into_iter()
        //     .map(|e| {
        //         println!("....{:?}", e);

        //         match e.kind() {
        //             OrgSyntaxKind::ListItemBullet => {
        //                 bullet =
        //                          e.as_node().unwrap()
        //                          .first_child_by_kind(& |e| e==OrgSyntaxKind::Text)
        //                          .unwrap()
        //                          .text()
        //                          .to_string()
        //                  },

        //                  OrgSyntaxKind::ListItemContent => {
        //                      contents.push(
        //                          self.convert_element(e.as_node().unwrap()).unwrap()
        //                      );
        //                  },

        //                  _ => {}
        //              }
        //          }
        //     );

        // println!("bullet={:?}, contents={:?}", bullet, contents);
        Ok(Item {
            syntax: node.clone(),
            bullet,
            counter_set,
            checkbox,
            tag,
            contents,
        })
    }

    // element.footnote_definition
    fn convert_footnote_definition(
        &mut self,
        node: &SyntaxNode,
    ) -> Result<FootnoteDefinition, AstError> {
        let label = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Text)
            .nth(1)
            .expect("footnote definition should has at least two text")
            .as_token()
            .expect("todo")
            .text()
            .to_string();

        let contents = node
            .children()
            .map(|e| self.convert_element(&e))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .collect();

        let rids = self.footnote_label_to_rids.get(&label).expect(&format!(
            "convert_footnote_defintion(): Can't get {label} from {:?}, {node:?}",
            self.footnote_label_to_rids
        ));
        let nid = self.footnote_label_to_nid.get(&label).expect("todo");

        let footnote_definition = FootnoteDefinition {
            syntax: node.clone(),
            rids: rids.clone(),
            nid: *nid,
            label,
            contents,
        };

        self.footnote_definitions.push(footnote_definition.clone());

        Ok(footnote_definition)
    }

    fn get_list_type(&self, node: &SyntaxNode) -> ListType {
        // println!("node={:#?}", node);
        let is_ordered = node
            .first_child_by_kind(&|e| e == OrgSyntaxKind::ListItem)
            .expect("list must has at least one item")
            .first_child_by_kind(&|e| e == OrgSyntaxKind::ListItemBullet)
            .expect("item must has one bullet");
        // println!("is_ordered = {:#?}", is_ordered);

        let is_ordered = is_ordered
            .first_child_or_token_by_kind(&|e| e == OrgSyntaxKind::Text)
            .expect("bullet must has one text")
            .as_token()
            .unwrap()
            .text()
            .to_string()
            .as_str()
            .starts_with(|c: char| c.is_ascii_digit());

        let is_descriptive = node
            .children()
            .filter(|e| e.kind() == OrgSyntaxKind::ListItem)
            .any(|item| {
                item.children()
                    .any(|e| e.kind() == OrgSyntaxKind::ListItemTag)
            });

        if is_ordered {
            ListType::Ordered
        } else if is_descriptive {
            ListType::Descriptive
        } else {
            ListType::Unordered
        }
    }
    // element.list
    fn convert_list(&mut self, node: &SyntaxNode) -> Result<List, AstError> {
        Ok(List {
            syntax: node.clone(),
            list_type: self.get_list_type(node),
            items: node
                .children()
                .filter(|e| e.kind() == OrgSyntaxKind::ListItem)
                .map(|e| self.convert_item(&e))
                .filter(|e| e.is_ok())
                .map(|e| e.unwrap())
                .collect(),
        })
    }

    // element.horizontal_rule
    fn convert_horizontal_rule(&self, node: &SyntaxNode) -> Result<HorizontalRule, AstError> {
        Ok(HorizontalRule {
            syntax: node.clone(),
        })
    }

    // element.keyword
    fn convert_keyword(&self, node: &SyntaxNode) -> Result<Keyword, AstError> {
        let mut iter = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Text)
            .map(|e| e.into_token())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap());

        let key = iter.next().expect("first text").text().to_string();
        let value = iter.next().expect("second text").text().to_string();

        Ok(Keyword {
            syntax: node.clone(),
            key,
            value,
        })
    }

    // element.latex_environment
    fn convert_latex_environment(&self, node: &SyntaxNode) -> Result<LatexEnvironment, AstError> {
        Ok(LatexEnvironment {
            syntax: node.clone(),
        })
    }

    // fn extract_text_content(node: &SyntaxNode) -> Result<String, AstError> {
    //     // 提取纯文本内容，去除标记符号
    //     let text = node.text().to_string();
    //     // 移除前面的星号等标记
    //     let content = text.trim_start_matches('*').trim();
    //     Ok(content.to_string())
    // }

    // fn convert_children(node: &SyntaxNode) -> Result<Vec<AstNode>, AstError> {
    //     node.children()
    //         .map(|node| {Self::convert_node(&node)})
    //         .collect()
    // }
}
