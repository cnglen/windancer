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

use std::collections::HashMap;
use std::env;

use crate::ast::element::{
    AffiliatedKeyword, CenterBlock, Comment, CommentBlock, Document, Drawer, Element, ExampleBlock,
    ExportBlock, FixedWidth, FootnoteDefinition, HeadingSubtree, HorizontalRule, Item, Keyword,
    LatexEnvironment, List, ListType, NodeProperty, Paragraph, Planning, PropertyDrawer,
    QuoteBlock, Section, SpecialBlock, SrcBlock, Table, TableFormula, TableRow, TableRowType,
    VerseBlock,
};
use crate::ast::error::AstError;
use crate::ast::object::{CitationReference, Object, TableCell, TableCellType};
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
    radio_targets: Vec<Object>,
    k2v: HashMap<String, Vec<Object>>,
}

impl Converter {
    fn new() -> Self {
        Self {
            footnote_label_to_rids: HashMap::new(),
            n_anonymous_label: 0,
            footnote_label_to_nid: HashMap::new(),
            footnote_definitions: vec![],
            radio_targets: vec![],
            k2v: HashMap::new(),
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
                    eprintln!("Only HeadingSubtree supported in Document's children!");
                    std::process::exit(1);
                }
            }
        }

        self.footnote_definitions.sort_by(|a, b| a.nid.cmp(&b.nid));

        Ok(Document {
            heading_subtrees: heading_subtrees,
            zeroth_section: zeroth_section,
            footnote_definitions: self.footnote_definitions.clone(),
            k2v: self.k2v.clone(),
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
        let mut title = vec![];
        let mut tags = vec![];
        let mut planning = None;
        let mut property_drawer = None;
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
                            OrgSyntaxKind::HeadingRowKeywordOther => {
                                keyword = Some(c.as_token().unwrap().text().to_string())
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
                                let ans = c
                                    .as_node()
                                    .unwrap()
                                    .children_with_tokens()
                                    .map(|e| self.convert_object(&e))
                                    .filter(|e| e.is_ok())
                                    .map(|e| e.unwrap())
                                    .filter(|e| e.is_some())
                                    .map(|e| e.unwrap())
                                    .collect();

                                title = ans;
                                // title = Some(c.as_token().unwrap().text().to_string())
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
                            OrgSyntaxKind::Planning => {
                                planning = Some(
                                    self.convert_planning(c.as_node().unwrap())
                                        .expect("planning"),
                                );
                            }

                            OrgSyntaxKind::PropertyDrawer => {
                                property_drawer = Some(
                                    self.convert_property_drawer(c.as_node().unwrap())
                                        .expect("property drawer"),
                                );
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
            section: section,
            level: level,
            is_commented: is_commented,
            keyword: keyword,
            priority: priority,
            tags: tags,
            title: title,
            planning,
            property_drawer,
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

        Ok(Section { elements: elements })
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

            OrgSyntaxKind::Comment => Ok(Element::Comment(self.convert_comment(&node)?)),

            OrgSyntaxKind::NodeProperty => {
                Ok(Element::NodeProperty(self.convert_node_property(&node)?))
            }

            OrgSyntaxKind::PropertyDrawer => Ok(Element::PropertyDrawer(
                self.convert_property_drawer(&node)?,
            )),

            OrgSyntaxKind::FixedWidth => Ok(Element::FixedWidth(self.convert_fixed_width(&node)?)),

            OrgSyntaxKind::HorizontalRule => {
                Ok(Element::HorizontalRule(self.convert_horizontal_rule()?))
            }

            OrgSyntaxKind::FootnoteDefinition => Ok(Element::FootnoteDefinition(
                self.convert_footnote_definition(&node)?,
            )),

            OrgSyntaxKind::LatexEnvironment => Ok(Element::LatexEnvironment(
                self.convert_latex_environment(&node)?,
            )),

            OrgSyntaxKind::Planning => Ok(Element::Planning(self.convert_planning(&node)?)),

            _ => {
                println!("node: {node:#?}");

                Err(AstError::UnknownNodeType {
                    kind: node.kind(),
                    position: None,
                })
            }
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

            OrgSyntaxKind::FootnoteReference => {
                Ok(self.convert_footnote_reference(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Entity => Ok(self.convert_entity(node_or_token.as_node().unwrap())?),

            OrgSyntaxKind::Timestamp => {
                Ok(self.convert_timestamp(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Macro => Ok(self.convert_macro(node_or_token.as_node().unwrap())?),

            OrgSyntaxKind::RadioTarget => {
                Ok(self.convert_radio_target(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::RadioLink => {
                Ok(self.convert_radio_link(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::LineBreak => Ok(Some(Object::LineBreak)),

            OrgSyntaxKind::Target => Ok(self.convert_target(node_or_token.as_node().unwrap())?),

            OrgSyntaxKind::LatexFragment => {
                Ok(self.convert_latex_fragment(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Subscript => {
                Ok(self.convert_subscript(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Superscript => {
                Ok(self.convert_superscript(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Whitespace => Ok(Some(Object::Whitespace(String::from(" ")))),

            // OrgSyntaxKind::Link => Ok(self.convert_link(node_or_token.as_node().unwrap())?),
            OrgSyntaxKind::PlainLink | OrgSyntaxKind::Link | OrgSyntaxKind::AngleLink => {
                Ok(self.convert_link(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::InlineSourceBlock => {
                Ok(self.convert_inline_source_block(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::InlineBabelCall => {
                Ok(self.convert_inline_babel_call(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::Citation => Ok(self.convert_citation(node_or_token.as_node().unwrap())?),

            OrgSyntaxKind::StatisticsCookie => {
                Ok(self.convert_statistics_cookie(node_or_token.as_node().unwrap())?)
            }

            OrgSyntaxKind::ExportSnippet => {
                Ok(self.convert_export_snippet(node_or_token.as_node().unwrap())?)
            }

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
        let mut affiliated_keywords: Vec<AffiliatedKeyword> = vec![];
        for child in node.children_with_tokens() {
            match child.kind() {
                OrgSyntaxKind::AffiliatedKeyword => {
                    if let Ok(affiliated_keyword) =
                        self.convert_affiliated_keyword(&child.as_node().unwrap())
                    {
                        affiliated_keywords.push(affiliated_keyword)
                    }
                }
                _ => {
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
            }
        }

        Ok(Paragraph {
            objects,
            affiliated_keywords,
        })
    }

    // element.drawrer
    fn convert_drawer(&mut self, node: &SyntaxNode) -> Result<Drawer, AstError> {
        let mut name = String::new();
        let mut contents: Vec<Element> = vec![];
        let mut affiliated_keywords = vec![];

        for child in node.children_with_tokens() {
            match child.kind() {
                OrgSyntaxKind::AffiliatedKeyword => {
                    if let Ok(affiliated_keyword) =
                        self.convert_affiliated_keyword(&child.as_node().unwrap())
                    {
                        affiliated_keywords.push(affiliated_keyword);
                    }
                }
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
            name,
            contents,
            affiliated_keywords,
        })
    }

    // element.property_drawrer
    fn convert_property_drawer(&mut self, node: &SyntaxNode) -> Result<PropertyDrawer, AstError> {
        let contents = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::NodeProperty)
            .map(|e| self.convert_node_property(e.as_node().unwrap()).unwrap())
            .collect::<Vec<_>>();

        Ok(PropertyDrawer { contents })
    }

    // element.planning
    fn convert_planning(&mut self, node: &SyntaxNode) -> Result<Planning, AstError> {
        let keyword = node
            .first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::PlanningKeyword)
            .expect("planning must has one text")
            .as_token()
            .unwrap()
            .text()
            .to_string();

        let timestamp = node
            .first_child_by_kind(&|c| c == OrgSyntaxKind::Timestamp)
            .expect("planning must has one timestamp");
        let timestamp = self
            .convert_timestamp(&timestamp)
            .expect("timestamp")
            .expect("timestamp");

        Ok(Planning { keyword, timestamp })
    }

    // element.table
    fn convert_table(&mut self, node: &SyntaxNode) -> Result<Table, AstError> {
        let mut name = None;
        let mut caption = vec![];
        let separator = None;
        let mut rows = vec![];
        let mut header = vec![];
        let mut formulas = vec![];

        let idx_rule_row = node
            .children()
            .enumerate()
            .find(|(_i, e)| e.kind() == OrgSyntaxKind::TableRuleRow)
            .map(|(idx, _)| idx)
            .unwrap_or(0);
        for (i, row) in node.children().enumerate() {
            match row.kind() {
                OrgSyntaxKind::TableStandardRow => {
                    if i < idx_rule_row {
                        header.push(self.convert_table_row(&row, TableRowType::Header)?);
                    } else {
                        rows.push(self.convert_table_row(&row, TableRowType::Data)?);
                    }
                }
                OrgSyntaxKind::TableRuleRow => {
                    // rows.push(self.convert_table_row(&row, TableRowType::Rule)?);
                }

                OrgSyntaxKind::TableFormula => {
                    formulas.push(self.convert_table_formula(&row)?);
                }

                OrgSyntaxKind::AffiliatedKeyword => {
                    let affliated_keyword = self.convert_affiliated_keyword(&row)?;

                    match affliated_keyword.key.to_uppercase().as_str() {
                        "CAPTION" => {
                            caption = affliated_keyword.value;
                        }
                        "NAME" => {
                            name = Some(
                                affliated_keyword
                                    .value
                                    .into_iter()
                                    .filter(|e| match e {
                                        Object::Text(_t) => true,
                                        _ => false,
                                    })
                                    .map(|e| match e {
                                        Object::Text(t) => t,
                                        _ => String::from(""),
                                    })
                                    .collect::<String>(),
                            );
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        Ok(Table {
            name,
            caption,
            header,
            separator,
            rows,
            formulas,
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
            .map(|e| self.convert_table_cell(&e, row_type.clone()))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect::<Vec<_>>();

        Ok(TableRow { cells, row_type })
    }

    // element: table_row??
    fn convert_table_formula(&mut self, node: &SyntaxNode) -> Result<TableFormula, AstError> {
        let data = node
            .first_child_by_kind(&|e| e == OrgSyntaxKind::TableFormulaValue)
            .expect("table formula value")
            .first_child_or_token_by_kind(&|e| e == OrgSyntaxKind::Text)
            .expect("text")
            .as_token()
            .unwrap()
            .text()
            .to_string();

        Ok(TableFormula { data })
    }

    // object.table_cell
    // fixme
    fn convert_table_cell(
        &mut self,
        node: &SyntaxNode,
        row_type: TableRowType,
    ) -> Result<Option<Object>, AstError> {
        let contents = node
            .children_with_tokens()
            .map(|e| self.convert_object(&e))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect();

        let cell_type = match row_type {
            TableRowType::Data => TableCellType::Data,
            TableRowType::Header => TableCellType::Header,
            _ => TableCellType::Data,
        };

        Ok(Some(Object::TableCell(TableCell {
            contents,
            cell_type,
        })))
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

    // object.radio_target
    fn convert_radio_target(&mut self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let objects = node
            .children_with_tokens()
            .map(|e| self.convert_object(&e))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect::<Vec<_>>();

        let radio_target = Object::RadioTarget(objects);
        self.radio_targets.push(radio_target.clone());
        Ok(Some(radio_target))
    }

    // object.radio_link
    fn convert_radio_link(&mut self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let objects = node
            .children_with_tokens()
            .map(|e| self.convert_object(&e))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect::<Vec<_>>();

        let radio_link = Object::RadioLink(objects);
        Ok(Some(radio_link))
    }

    // object.target
    fn convert_target(&mut self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let text = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Text)
            .map(|e| e.as_token().unwrap().text().to_string())
            .collect::<String>();

        Ok(Some(Object::Target(text)))
    }

    // object.timestamp
    fn convert_timestamp(&self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let text = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Text)
            .map(|e| e.as_token().unwrap().text().to_string())
            .collect::<String>();

        Ok(Some(Object::Timestamp(text)))
    }

    // object.text
    fn convert_text(&self, token: &SyntaxToken) -> Result<Option<Object>, AstError> {
        Ok(Some(Object::Text(token.text().to_string())))
    }

    // object.subscript
    fn convert_subscript(&mut self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let objects = node
            .children_with_tokens()
            .map(|e| self.convert_object(&e))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect();
        Ok(Some(Object::Subscript(objects)))
    }

    // object.subscript
    fn convert_superscript(&mut self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let objects = node
            .children_with_tokens()
            .map(|e| self.convert_object(&e))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect();
        Ok(Some(Object::Superscript(objects)))
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
    fn parse_pathreg(s: String) -> (String, String) {
        if s.starts_with("./") {
            ("file".to_string(), s.replacen("./", "file:./", 1))
        } else if s.starts_with("/") {
            ("file".to_string(), s.replacen("/", "file:/", 1))
        } else if s.starts_with("#") {
            ("custom_id".to_string(), s)
        } else if s.starts_with("(") && s.ends_with(")") {
            ("coderef".to_string(), s)
        } else if s.contains(":") {
            (s.split_once(":").unwrap().0.to_lowercase(), s)
        } else if s.starts_with("*") {
            ("internal_section".to_string(), s)
        } else {
            ("fuzzy".to_string(), s)
        }
    }

    fn convert_link(&mut self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let (pathreg, description) = match node.kind() {
            OrgSyntaxKind::Link => {
                let mut path = node
                    .first_child_by_kind(&|c| c == OrgSyntaxKind::LinkPath)
                    .expect("has link")
                    .first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::Text)
                    .expect("has text")
                    .as_token()
                    .unwrap()
                    .text()
                    .to_string();

                if path.starts_with("file:~") {
                    path = path.replace(
                        "file:~",
                        format!("file://{}", env::var("HOME").unwrap()).as_str(),
                    );
                }

                let desc_objects = if let Some(desc) =
                    node.first_child_by_kind(&|c| c == OrgSyntaxKind::LinkDescription)
                {
                    desc.children_with_tokens()
                        .filter(|e| {
                            e.kind() != OrgSyntaxKind::LeftSquareBracket
                                && e.kind() != OrgSyntaxKind::RightSquareBracket
                        })
                        .map(|e| self.convert_object(&e))
                        .filter(|e| e.is_ok())
                        .map(|e| e.unwrap())
                        .filter(|e| e.is_some())
                        .map(|e| e.unwrap())
                        .collect()
                } else {
                    vec![]
                };
                (path, desc_objects)
            }
            OrgSyntaxKind::AngleLink | OrgSyntaxKind::PlainLink => {
                let path = node
                    .children_with_tokens()
                    .filter(|e| e.kind() == OrgSyntaxKind::Text || e.kind() == OrgSyntaxKind::Colon)
                    .map(|e| e.as_token().unwrap().text().to_string())
                    .collect::<String>();
                (path, vec![])
            }
            _ => (String::from("unreachable"), vec![]),
        };

        let (protocol, path) = Self::parse_pathreg(pathreg);

        let is_image = (protocol == "file" || protocol == "https")
            && [".jpg", ".jpeg", ".png", ".gif", ".svg"]
                .iter()
                .any(|ext| path.to_lowercase().ends_with(ext));

        Ok(Some(Object::GeneralLink {
            protocol,
            path,
            description,
            is_image,
        }))
    }

    // object.footnote_reference
    // - generate footnote definition if possible
    fn convert_footnote_reference(
        &mut self,
        node: &SyntaxNode,
    ) -> Result<Option<Object>, AstError> {
        let is_anoymous = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::FootnoteReferenceLabel)
            .count()
            == 0;

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
            let objects = definition
                .as_node()
                .unwrap()
                .children_with_tokens()
                .filter(|e| e.kind() != OrgSyntaxKind::AffiliatedKeyword)
                .map(|e| self.convert_object(&e).unwrap().unwrap())
                .collect::<Vec<_>>();
            let affiliated_keywords = definition
                .as_node()
                .unwrap()
                .children_with_tokens()
                .filter(|e| e.kind() == OrgSyntaxKind::AffiliatedKeyword)
                .map(|e| {
                    self.convert_affiliated_keyword(&e.as_node().unwrap())
                        .unwrap()
                })
                .collect::<Vec<_>>();

            let element = Element::Paragraph(Paragraph {
                objects,
                affiliated_keywords,
            });

            // definition
            let rids = self.footnote_label_to_rids.get(&label).expect("todo");
            let nid = self.footnote_label_to_nid.get(&label).expect("todo");
            let footnote_definition = FootnoteDefinition {
                label: label.clone(),
                contents: vec![element],
                rids: rids.clone(),
                nid: *nid,
            };
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

    // object.inline_source_block
    fn convert_inline_source_block(&self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let lang = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::InlineSourceBlockLang)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        let body = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::InlineSourceBlockBody)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        let headers = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::InlineSourceBlockHeaders)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        let headers = if headers.is_empty() {
            None
        } else {
            Some(headers)
        };

        Ok(Some(Object::InlineSourceBlock {
            lang,
            headers,
            body,
        }))
    }

    // object.export_snippet
    fn convert_export_snippet(&self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let backend = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::ExportSnippetBackend)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        let value = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::ExportSnippetValue)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        Ok(Some(Object::ExportSnippet { backend, value }))
    }

    // object.inline_babel_call
    fn convert_inline_babel_call(&self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let name = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::InlineBabelCallName)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        let arguments = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::InlineBabelCallArguments)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        let header1 = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::InlineBabelCallHeader1)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        let header2 = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::InlineBabelCallHeader2)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        let header1 = if header1.is_empty() {
            None
        } else {
            Some(header1)
        };

        let header2 = if header2.is_empty() {
            None
        } else {
            Some(header2)
        };

        Ok(Some(Object::InlineBabelCall {
            name,
            header1,
            arguments,
            header2,
        }))
    }

    // object.citation_reference
    fn convert_citation_reference(
        &mut self,
        node: &SyntaxNode,
    ) -> Result<Option<CitationReference>, AstError> {
        let key = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::CitationReferenceKey)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        let key_prefix_node =
            node.first_child_by_kind(&|e| e == OrgSyntaxKind::CitationReferenceKeyPrefix);
        let key_prefix = match key_prefix_node {
            None => vec![],
            Some(e) => e
                .children_with_tokens()
                .map(|e| self.convert_object(&e))
                .filter(|e| e.is_ok())
                .map(|e| e.unwrap())
                .filter(|e| e.is_some())
                .map(|e| e.unwrap())
                .collect::<Vec<_>>(),
        };

        let key_suffix_node =
            node.first_child_by_kind(&|e| e == OrgSyntaxKind::CitationReferenceKeySuffix);
        let key_suffix = match key_suffix_node {
            None => vec![],
            Some(e) => e
                .children_with_tokens()
                .map(|e| self.convert_object(&e))
                .filter(|e| e.is_ok())
                .map(|e| e.unwrap())
                .filter(|e| e.is_some())
                .map(|e| e.unwrap())
                .collect::<Vec<_>>(),
        };

        Ok(Some(CitationReference {
            key_prefix,
            key,
            key_suffix,
        }))
    }

    // object.citation
    fn convert_citation(&mut self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let citestyle = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::CitationCitestyle)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();
        let citestyle = if citestyle.is_empty() {
            None
        } else {
            Some(citestyle)
        };

        let references = node
            .children()
            .filter(|e| e.kind() == OrgSyntaxKind::CitationReference)
            .map(|e| self.convert_citation_reference(&e))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect::<Vec<_>>();

        let global_prefix_node =
            node.first_child_by_kind(&|e| e == OrgSyntaxKind::CitationGlobalPrefix);
        let global_prefix = match global_prefix_node {
            None => vec![],
            Some(e) => e
                .children_with_tokens()
                .map(|e| self.convert_object(&e))
                .filter(|e| e.is_ok())
                .map(|e| e.unwrap())
                .filter(|e| e.is_some())
                .map(|e| e.unwrap())
                .collect::<Vec<_>>(),
        };

        let global_suffix_node =
            node.first_child_by_kind(&|e| e == OrgSyntaxKind::CitationGlobalSuffix);
        let global_suffix = match global_suffix_node {
            None => vec![],
            Some(e) => e
                .children_with_tokens()
                .map(|e| self.convert_object(&e))
                .filter(|e| e.is_ok())
                .map(|e| e.unwrap())
                .filter(|e| e.is_some())
                .map(|e| e.unwrap())
                .collect::<Vec<_>>(),
        };

        Ok(Some(Object::Citation {
            global_prefix,
            citestyle,
            references,
            global_suffix,
        }))
    }

    // object.statistics_cookie
    fn convert_statistics_cookie(&self, node: &SyntaxNode) -> Result<Option<Object>, AstError> {
        let value = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Text)
            .map(|e| e.as_token().expect("todo").text().to_string())
            .collect::<String>();

        Ok(Some(Object::StatisticsCookie(value)))
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
                let _q = node.first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::BlockBegin);
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
                let _q = node.first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::BlockBegin);
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
            parameters: parameters,
            contents: contents,
        })
    }

    // element.special_block
    fn convert_special_block(&mut self, node: &SyntaxNode) -> Result<SpecialBlock, AstError> {
        let parameters = None;
        let mut contents = vec![];
        let mut name = String::new();

        match node.kind() {
            OrgSyntaxKind::SpecialBlock => {
                name = node
                    .first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::BlockBegin)
                    .expect(format!("no block begin found: {:#?}", node).as_str())
                    .as_node()
                    .unwrap()
                    .children_with_tokens()
                    .filter(|e| e.kind() == OrgSyntaxKind::Text)
                    .nth(1)
                    .expect("special block begin row should has at least two text")
                    .as_token()
                    .expect("todo")
                    .text()
                    .to_string()
                    .to_lowercase();

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
            parameters: parameters,
            contents: contents,
            name: name,
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
            data: data,
            contents: contents,
        })
    }

    // element.src_block
    fn convert_src_block(&mut self, node: &SyntaxNode) -> Result<SrcBlock, AstError> {
        let data = None;
        let mut contents = vec![];
        let mut language = String::new();
        match node.kind() {
            OrgSyntaxKind::SrcBlock => {
                language = node
                    .first_child_by_kind(&|c| c == OrgSyntaxKind::BlockBegin)
                    .unwrap()
                    .first_child_or_token_by_kind(&|c| c == OrgSyntaxKind::SrcBlockLanguage)
                    .unwrap()
                    .as_token()
                    .unwrap()
                    .text()
                    .to_string()
                    .to_lowercase();

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
            language: language,
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
            data: data,
            contents: contents,
        })
    }

    //

    // element.item
    fn convert_item(&mut self, node: &SyntaxNode) -> Result<Item, AstError> {
        let mut bullet = String::new();
        let counter_set = None;
        let mut checkbox = None;
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

                OrgSyntaxKind::ListItemCheckbox => {
                    checkbox = Some(format!(
                        "[{}]",
                        child
                            .as_node()
                            .unwrap()
                            .first_child_or_token_by_kind(&|e| e == OrgSyntaxKind::Text)
                            .unwrap()
                            .as_token()
                            .unwrap()
                            .text()
                            .to_string(),
                    ));
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
            .first_child_by_kind(&|e| e == OrgSyntaxKind::ListItem)
            .expect("list must has at least one item")
            .children()
            // .filter(|e| e.kind() == OrgSyntaxKind::ListItem)
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
    fn convert_horizontal_rule(&self) -> Result<HorizontalRule, AstError> {
        Ok(HorizontalRule {})
    }

    // element.comment
    fn convert_comment(&self, node: &SyntaxNode) -> Result<Comment, AstError> {
        let text = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Text)
            .map(|e| e.as_token().unwrap().text().to_string())
            .collect::<String>();

        Ok(Comment { text: text })
    }

    // element.node_property
    fn convert_node_property(&self, node: &SyntaxNode) -> Result<NodeProperty, AstError> {
        let text = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Text)
            .map(|e| e.as_token().unwrap().text().to_string())
            .collect::<Vec<_>>();

        let name: String = text.first().expect("must have at least 1 text").to_string();
        let value: Option<String> = if text.len() == 2 {
            Some(text.last().expect("must have at a last text").to_string())
        } else {
            None
        };

        Ok(NodeProperty { name, value })
    }

    // element.fixed_width
    fn convert_fixed_width(&self, node: &SyntaxNode) -> Result<FixedWidth, AstError> {
        let text = node
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::FixedWidthLine)
            .flat_map(|e| {
                e.as_node()
                    .unwrap()
                    .children_with_tokens()
                    .filter(|e| e.kind() == OrgSyntaxKind::Text)
            })
            .map(|e| e.as_token().unwrap().text().trim_start().to_string())
            .collect::<Vec<String>>()
            .join("\n");

        Ok(FixedWidth { text: text })
    }

    // element.keyword
    fn convert_keyword(&mut self, node: &SyntaxNode) -> Result<Keyword, AstError> {
        let key = node
            .first_child_by_kind(&|e| e == OrgSyntaxKind::KeywordKey)
            .unwrap()
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Text)
            .map(|e| e.as_token().unwrap().text().to_string())
            .collect::<String>();

        let value = node
            .first_child_by_kind(&|e| e == OrgSyntaxKind::KeywordValue)
            .unwrap()
            .children_with_tokens()
            .map(|e| self.convert_object(&e))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect::<Vec<_>>();

        self.k2v.insert(key.clone(), value.clone());

        Ok(Keyword { key, value })
    }

    // element.affiliatedkeyword
    fn convert_affiliated_keyword(
        &mut self,
        node: &SyntaxNode,
    ) -> Result<AffiliatedKeyword, AstError> {
        let key = node
            .first_child_by_kind(&|e| e == OrgSyntaxKind::KeywordKey)
            .unwrap()
            .children_with_tokens()
            .filter(|e| e.kind() == OrgSyntaxKind::Text)
            .map(|e| e.as_token().unwrap().text().to_string())
            .collect::<String>();

        let optvalue = if let Some(node_optvalue) =
            node.first_child_by_kind(&|e| e == OrgSyntaxKind::KeywordOptvalue)
        {
            Some(
                node_optvalue
                    .children_with_tokens()
                    .filter(|e| e.kind() == OrgSyntaxKind::Text)
                    .map(|e| e.as_token().unwrap().text().to_string())
                    .collect::<String>(),
            )
        } else {
            None
        };

        let objects = node
            .first_child_by_kind(&|e| e == OrgSyntaxKind::KeywordValue)
            .unwrap()
            .children_with_tokens()
            .map(|e| self.convert_object(&e))
            .filter(|e| e.is_ok())
            .map(|e| e.unwrap())
            .filter(|e| e.is_some())
            .map(|e| e.unwrap())
            .collect::<Vec<_>>();

        Ok(AffiliatedKeyword {
            key,
            optvalue,
            value: objects,
        })
    }

    // element.latex_environment
    fn convert_latex_environment(&self, node: &SyntaxNode) -> Result<LatexEnvironment, AstError> {
        Ok(LatexEnvironment {
            text: format!("{}", node.clone()),
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
