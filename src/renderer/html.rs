//! HtmlRenderer renders AST to HTML string，including three levels:
//! - Document: `render_document()` renders the `Document` node of AST into html, which calls
//!   - `render_section()`
//!     - `render_element()`
//!   - `render_heading_subtree()`
//!     - `render_section()`
//!     - `render_heading_subtree()` call self **recursively**
//! - Element Render: `render_element()` renders `ast::element::Element`(Node) into html, which calls
//!   - `render_paragraph()`
//!   - `render_table()` -> `render_table_row()`
//!   - `render_drawer()`
//!   - `render_center_block()`, `render_quote_block()`, `render_special_block()`
//!   - `render_example_block()`, `render_explort_block()`, `render_verse_block()`, `render_comment_block()`, `render_export_block()`
//!   - `render_list()`, `render_item()`
//!   - `render_footnote_definition()`
//!   - `render_horizontal_rule()`
//!   - `render_latex_environment()`
//! - Object Render: `render_object()` renders `ast:object::Object`(Node Or Token) into html using match, including
//!   - Text
//!   - Bold, Italic, Underline, Strikethrough, Code, Verbatim
//!   - TableCell
//!   - Link
//!   - Whitespace
//!
//! Todo
//! - css: better apperance
//! - source code highlight
//! - title: property
//! - footnote

use crate::ast::element::{
    CenterBlock, CommentBlock, Document, Drawer, Element, ExampleBlock, ExportBlock,
    FootnoteDefinition, HeadingSubtree, HorizontalRule, Item, Keyword, LatexEnvironment, List,
    ListType, Paragraph, QuoteBlock, Section, SpecialBlock, SrcBlock, Table, TableRow, VerseBlock,
};

use crate::ast::object::Object;
use crate::parser::object::entity::ENTITYNAME_TO_HTML;

use std::fs;

pub struct HtmlRenderer {
    config: RenderConfig,
}

#[derive(Clone)]
pub struct RenderConfig {
    pub include_css: bool,
    pub class_prefix: String,
    pub highlight_code_blocks: bool,
}

impl HtmlRenderer {
    pub fn new(config: RenderConfig) -> Self {
        Self { config }
    }

    pub fn render_document(&self, document: &Document) -> String {
        let css = &fs::read_to_string("src/renderer/default.css").unwrap_or(String::new());

        let mut output = String::new();

        // // 文档开始
        // if self.config.include_css {
        //     output.push_str(&self.default_css());
        // }

        // 渲染所有元素
        if let Some(section) = &document.zeroth_section {
            output.push_str(&self.render_section(section));
        }

        for subtree in &document.heading_subtrees {
            output.push_str(&self.render_heading_subtree(subtree));
        }

        for footnote_definition in &document.footnote_definitions {
            output.push_str(&self.render_footnote_definition(footnote_definition));
        }

        let automatic_equation_numbering = true;
        let aen = if automatic_equation_numbering {
            r##"<script>
    window.MathJax = {
      tex: {
       tags: 'ams'
      }
    };
    </script>"##
        } else {
            ""
        };

        format!(
            r##"<!DOCTYPE html>
<html>
  <head>
    <meta http-equiv="Content-Type" content="text/html;charset=utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="generator" content="Org Mode">
    <title>Todo</title>
    <script defer src="https://cdn.jsdelivr.net/npm/mathjax@4/tex-mml-chtml.js"></script>
    <style type="text/css">
    {}
    </style>
    {}
  </head>
  <body>
  <div id="content" class="content">
  {}
  </div>
  <div id="postamble" class="status">
    <p class="date">Date: 20250802[Sat] 18:43:14</p>
    <p class="date">Created: 2025-10-19 Sun 16:01</p>
    <p class="validation"><a href="https://validator.w3.org/check?uri=referer">Validate</a></p>
  </div>
  </body>
</html>"##,
            css, aen, output
        )
    }

    fn render_section(&self, section: &Section) -> String {
        section
            .elements
            .iter()
            .map(|c| self.render_element(c))
            .collect::<String>()
    }

    fn render_heading_subtree(&self, heading: &HeadingSubtree) -> String {
        if heading.is_commented {
            return String::from("");
        }

        let todo_html = if let Some(todo) = &heading.keyword {
            format!(r#"<span class="todo">{}</span> "#, escape_html(todo))
        } else {
            String::new()
        };

        let tags_html = if !heading.tags.is_empty() {
            let tags: Vec<String> = heading
                .tags
                .iter()
                .map(|tag| format!(r#"<span class="tag">{}</span>"#, escape_html(tag)))
                .collect();
            format!(r#"<span class="tags">{}</span>"#, tags.join(" "))
        } else {
            String::new()
        };

        let section_html = if let Some(section) = &heading.section {
            self.render_section(section)
        } else {
            String::new()
        };

        let content = if !heading.sub_heading_subtrees.is_empty() {
            let children_html: Vec<String> = heading
                .sub_heading_subtrees
                .iter()
                .map(|child| self.render_heading_subtree(child))
                .collect();
            format!("\n{}", children_html.join(""))
        } else {
            String::new()
        };

        format!(
            r##"<div class="outline-{level}">
  <h{level}>
  {todo}
  {title}
  {tags}
  </h{level}>
</div>
 {section}
 {content}
 "##,
            level = heading.level,
            title = escape_html(&heading.title.clone().unwrap()),
            todo = todo_html,
            tags = tags_html,
            section = section_html,
            content = content
        )
    }

    fn render_element(&self, element: &Element) -> String {
        match element {
            Element::Paragraph(paragraph) => self.render_paragraph(paragraph),
            Element::Table(table) => self.render_table(table),
            Element::Drawer(drawer) => self.render_drawer(drawer),
            Element::CenterBlock(center_block) => self.render_center_block(center_block),
            Element::QuoteBlock(quote_block) => self.render_quote_block(quote_block),
            Element::SpecialBlock(special_block) => self.render_special_block(special_block),
            Element::ExampleBlock(example_block) => self.render_example_block(example_block),
            Element::ExportBlock(export_block) => self.render_export_block(export_block),
            Element::CommentBlock(comment_block) => self.render_comment_block(comment_block),
            Element::SrcBlock(src_block) => self.render_src_block(src_block),
            Element::VerseBlock(verse_block) => self.render_verse_block(verse_block),

            Element::List(list) => self.render_list(list),
            Element::Item(item) => self.render_item(item),
            Element::FootnoteDefinition(footnote_definition) => {
                String::from("")
                // self.render_footnote_definition(footnote_definition)
            }
            Element::HorizontalRule(rule) => self.render_horizontal_rule(rule),
            Element::Keyword(keyword) => self.render_keyword(keyword),
            Element::LatexEnvironment(env) => self.render_latex_environment(env),

            _ => String::from(""),
            // AstElement::List(list) => self.render_list(list),

            // AstElement::HorizontalRule => "<hr/>\n".to_string(),
            // ... 其他元素渲染
        }
    }

    fn render_table(&self, table: &Table) -> String {
        format!(
            r##"
 <table>
 {}</table>
 "##,
            table
                .rows
                .iter()
                .map(|r| self.render_table_row(r))
                .collect::<String>()
        )
    }

    fn render_table_row(&self, table_row: &TableRow) -> String {
        format!(
            "  <tr>{}</tr>\n",
            table_row
                .cells
                .iter()
                .map(|e| self.render_object(&e))
                .collect::<String>()
        )
    }

    fn render_drawer(&self, drawer: &Drawer) -> String {
        drawer
            .contents
            .iter()
            .map(|c| self.render_element(c))
            .collect()
    }

    fn render_object(&self, object: &Object) -> String {
        // println!("object={:?}", object);
        match object {
            Object::Text(text) => escape_html(text),

            Object::Bold(objects) => {
                let inner: String = objects.iter().map(|o| self.render_object(o)).collect();
                format!("<b>{}</b>", inner)
            }
            Object::Italic(objects) => {
                let inner: String = objects.iter().map(|o| self.render_object(o)).collect();
                format!("<i>{}</i>", inner)
            }
            Object::Underline(objects) => {
                let inner: String = objects.iter().map(|o| self.render_object(o)).collect();
                format!(r##"<span class="underline">{}</span>"##, inner)
            }

            Object::Strikethrough(objects) => {
                let inner: String = objects.iter().map(|o| self.render_object(o)).collect();
                format!(r##"<del>{}</del>"##, inner)
            }

            Object::Code(objects) => {
                let inner: String = objects.iter().map(|o| self.render_object(o)).collect();
                format!(r##"<code>{}</code>"##, inner)
            }

            Object::Verbatim(objects) => {
                let inner: String = objects.iter().map(|o| self.render_object(o)).collect();
                format!(r##"<code>{}</code>"##, inner)
            }

            Object::Whitespace(content) => {
                format!(r##"{}"##, content)
            }

            Object::TableCell(table_cell) => {
                format!(
                    " <td>{}</td> ",
                    table_cell
                        .contents
                        .iter()
                        .map(|e| self.render_object(e))
                        .collect::<String>()
                )
            }

            Object::Link { url, text } => {
                format!(
                    r##"<a href="{}">{}</a>"##,
                    url,
                    match text {
                        Some(v) => v,
                        None => url,
                    }
                )
            }

            Object::FootnoteReference {
                label,
                nid,
                label_rid,
            } => {
                // superscript:label
                format!(
                    r##"<sup>
  <a id="fnr.{label}.{label_rid}" class="footref" href="#fn.{label}" role="doc-backlink">{label}</a>
</sup>
"##,
                    label_rid = label_rid,
                    label = label,
                )

                // //  superscript: nid
                //                 format!(
                //                     r##"<sup>
                //   <a id="fnr.{label}" class="footref" href="#fn.{}" role="doc-backlink">{label}</a>
                // </sup>
                // "##,
                //                     label = nid
                //                 )
            }

            Object::Entity { name } => {
                let v = match ENTITYNAME_TO_HTML.get(name) {
                    Some(v) => v,
                    None => "fixme!! error occured",
                };

                format!("{v}")
            }

            Object::LineBreak => {
                format!("<br>\n")
            }

            Object::LatexFragment {
                content,
                display_mode,
            } => match display_mode {
                Some(true) => {
                    format!(
                        r##"\[
{}\]
"##,
                        content
                    )
                }
                Some(false) => {
                    format!(r"\({}\)", content)
                }

                None => String::from(""),
            },

            _ => String::from(""), // AstInline::Link { url, text } => {
                                   //     format!(r#"<a href="{}">{}</a>"#, escape_html(url), escape_html(text))
                                   // }
                                   // AstInline::Code(code) => {
                                   //     format!("<code>{}</code>", escape_html(code))
                                   // }
                                   // ... 其他内联元素渲染
        }
    }

    // <p class=?>?
    fn render_paragraph(&self, paragraph: &Paragraph) -> String {
        let content: String = paragraph
            .objects
            .iter()
            .map(|object| self.render_object(object))
            .collect();
        format!(
            r##"<p>{}
</p>
"##,
            content
        )
    }

    // fixme: link: collect all footnotes into a div
    fn render_footnote_definition(&self, footnote_definition: &FootnoteDefinition) -> String {
        let c = if footnote_definition.rids.len() == 1 {
            format!(
                r##"  <sup>
    <a class="footnum" href="#fnr.{label}.{rid}" role="doc-backlink">^</a>
  </sup>
"##,
                label = footnote_definition.label,
                rid = 1
            )
        } else {
            footnote_definition
                .rids
                .iter()
                .map(|rid| {
                    format!(
                        r##"  <sup>
    <a class="footnum" href="#fnr.{label}.{rid}" role="doc-backlink">{rid}</a>
  </sup>
"##,
                        label = footnote_definition.label,
                        rid = rid
                    )
                })
                .collect::<String>()
        };

        format!(
            r##"<div class="footdef">
  <a id="fn.{label}">{label}</a>: {c}
  <div class="footpara" role="doc-footnote">
   {def}
  </div>
</div>
"##,
            label = footnote_definition.label,
            c = c,
            def = footnote_definition
                .contents
                .iter()
                .map(|e| self.render_element(e))
                .collect::<String>()
                .replace("<p>", r##"<p class="footpara">"##)
        )
    }

    fn render_center_block(&self, block: &CenterBlock) -> String {
        format!(
            r##"<div class="center">
{}</div>
"##,
            block
                .contents
                .iter()
                .map(|e| self.render_element(e))
                .collect::<String>()
        )
    }

    fn render_quote_block(&self, block: &QuoteBlock) -> String {
        format!(
            r##"<blockquote>
{}</blockquote>
"##,
            block
                .contents
                .iter()
                .map(|e| self.render_element(e))
                .collect::<String>()
        )
    }

    fn render_special_block(&self, block: &SpecialBlock) -> String {
        format!(
            r##"<div class="special">
{}</div>
"##,
            block
                .contents
                .iter()
                .map(|e| self.render_element(e))
                .collect::<String>()
        )
    }

    fn render_example_block(&self, block: &ExampleBlock) -> String {
        format!(
            r##"<pre class="example">
{}</pre>
"##,
            block
                .contents
                .iter()
                .map(|e| self.render_object(e))
                .collect::<String>()
        )
    }

    fn render_verse_block(&self, block: &VerseBlock) -> String {
        format!(
            r##"<p class="verse">
{}</p>
"##,
            block
                .contents
                .iter()
                .map(|e| self.render_object(e))
                .collect::<String>()
        )
    }

    // FIXME: language
    fn render_src_block(&self, block: &SrcBlock) -> String {
        format!(
            r##"<div class="org-src-container">
  <pre class="src src-language"> {}</pre>
</div>
"##,
            block
                .contents
                .iter()
                .map(|e| self.render_object(e))
                .collect::<String>()
        )
    }

    // FIXME: only supoort html now
    fn render_export_block(&self, block: &ExportBlock) -> String {
        format!(
            r##"{}"##,
            block
                .contents
                .iter()
                .map(|e| self.render_object(e))
                .collect::<String>()
        )
    }

    fn render_comment_block(&self, block: &CommentBlock) -> String {
        format!(r##""##)
    }

    fn render_keyword(&self, keyword: &Keyword) -> String {
        format!(r##""##)
    }

    fn render_list(&self, list: &List) -> String {
        match list.list_type {
            ListType::Unordered => {
                format!(
                    r##"<ul>
{}</ul>
"##,
                    list.items
                        .iter()
                        .map(|i| self.render_item(&i))
                        .collect::<String>()
                )
            }

            ListType::Ordered => {
                format!(
                    r##"<ol>
{}</ol>
"##,
                    list.items
                        .iter()
                        .map(|i| self.render_item(&i))
                        .collect::<String>()
                )
            }

            ListType::Descriptive => {
                format!(
                    r##"<ul>
{}</ul>
"##,
                    list.items
                        .iter()
                        .map(|i| self.render_item(&i))
                        .collect::<String>()
                )
            }
        }
    }

    fn render_item(&self, item: &Item) -> String {
        match &item.tag {
            None => format!(
                r##"  <li>
{}  </li>
"##,
                item.contents
                    .iter()
                    .map(|i| self.render_element(&i))
                    .collect::<String>()
            ),

            Some(tag) => {
                format!(
                    r##"  <dt>{}</dt> <dd>{}</dd>
"##,
                    tag,
                    item.contents
                        .iter()
                        .map(|i| self.render_element(&i))
                        .collect::<String>()
                )
            }
        }
    }

    fn render_horizontal_rule(&self, horizontal_rule: &HorizontalRule) -> String {
        format!(
            r##"<hr>
"##
        )
    }

    fn render_latex_environment(&self, latex_environment: &LatexEnvironment) -> String {
        format!(
            r##"{}
"##,
            latex_environment.syntax()
        )
    }
}

// HTML转义工具函数
fn escape_html(text: &str) -> String {
    // html_escape::encode_text(text).to_string()
    text.to_string()
}
