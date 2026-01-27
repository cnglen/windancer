//! Render document for SSG

// input: Section(doc:=meta+org)
// output: HTML site

//! HtmlRenderer renders AST to HTML string，including three levels:
//! - OrgFile: `render_document()` renders the `OrgFile` node of AST into html, which calls
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
//! - title: property
//! - footnote

use std::ffi::OsStr;
use std::fs;
use std::path::Path;
use petgraph::dot::Dot;
use crate::compiler::Compiler;
use crate::compiler::ast_builder::element::{
    self, CenterBlock, CommentBlock, Drawer, Element, ExampleBlock, ExportBlock, FixedWidth,
    FootnoteDefinition, HeadingSubtree, Item, Keyword, LatexEnvironment, List, ListType, Paragraph,
    QuoteBlock, Section, SpecialBlock, SrcBlock, Table, TableRow, TableRowType, VerseBlock,
};
use crate::compiler::ast_builder::object::{GeneralLink, Object, TableCellType};
use crate::compiler::content::{Document, Section as ContentSection};
use crate::constants::entity::ENTITYNAME_TO_HTML;
use crate::export::ssg::toc::{TableOfContents, TocNode};
use std::collections::HashMap;
use std::ops::Not;

use fs_extra::dir::{CopyOptions, copy, create_all};

fn copy_directory(from: &Path, to: &Path) -> Result<String, fs_extra::error::Error> {
    let mut options = CopyOptions::new();
    options.overwrite = false; // Overwrite existing files
    options.copy_inside = false;
    options.content_only = true;

    copy(from, to, &options)?;

    Ok("todo".to_string())
}

// context: prev / next
pub struct Renderer {
    config: RendererConfig,
    table_counter: usize,
    figure_counter: usize,
    footnote_defintions: Vec<FootnoteDefinition>,
    compiler: Compiler,
}

#[derive(Debug, Clone)]
pub struct RendererConfig {
    pub output_directory: String,
    pub css: String, // path of css file
                     // pub class_prefix: String,
                     // pub highlight_code_blocks: bool,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            css: include_str!("default.css").to_string(),
            output_directory: "public".to_string(),
        }
    }
}

impl Renderer {
    pub fn slugify(s: String) -> String {
        s.to_ascii_lowercase()
            .split(&['-', '_', ' '])
            .collect::<Vec<_>>()
            .join("-")
    }

    pub fn new(config: RendererConfig) -> Self {
        Self {
            config: config,
            table_counter: 0,
            figure_counter: 0,
            footnote_defintions: vec![],
            compiler: Compiler::new(),
        }
    }

    pub fn build_file<P: AsRef<Path>>(&mut self, f_org: P) -> std::io::Result<String> {
        let doc = self
            .compiler
            .compile_file(f_org)
            .expect("no Document compiled");
        self.render_document(&doc, None)?;

        Ok("todo".to_string())
    }

    // Site?
    pub fn build<P: AsRef<Path>>(&mut self, d_org: P) -> std::io::Result<String> {
        tracing::info!("build ...");

        let output_directory = Path::new(&self.config.output_directory);
        if output_directory.exists() {
            let now_utc = chrono::Utc::now();
            let created_ts = now_utc.format("%Y%m%dT%H%M%SZ").to_string();
            let backup_directory = format!("{}.backup_{}", output_directory.display(), created_ts);
            tracing::info!(
                "backup {} -> {}",
                output_directory.display(),
                backup_directory
            );
            fs::rename(output_directory, backup_directory)?;
        }
        let _ = create_all(output_directory, true);

        let d_org = d_org.as_ref();
        let section = self
            .compiler
            .compile_section(d_org)
            .expect("no Document compiled");

        tracing::debug!("docs={:?}", section.documents);
        let g = section.build_graph();
        let g_dot = Dot::new(&g.graph);
        tracing::debug!("Basic DOT format:\n{:?}\n", g_dot);
        tracing::debug!("{:#?}", g.graph);

        let toc_node = TocNode::from_section(&section);
        let toc = TableOfContents::new(toc_node.children);

        tracing::debug!("toc={:?}", toc);
        tracing::debug!("toc_html={:}", toc.to_html_nav(Some("git.html")));

        self.render_content_section(&section, Some(toc))?;

        let static_directory_from = section
            .file_info
            .full_path
            .parent()
            .expect("must have parent directory")
            .join("static");
        let static_directory_to = Path::new(&self.config.output_directory);
        if static_directory_from.is_dir() {
            tracing::debug!(from=?static_directory_from.display(), to=?static_directory_to.display());
            let _ = copy_directory(&static_directory_from, static_directory_to);
        }
        tracing::info!("build done");

        Ok(self.config.output_directory.clone())
    }

    // image and other files
    fn render_content_section(
        &mut self,
        section: &ContentSection,
        toc: Option<TableOfContents>,
    ) -> std::io::Result<String> {
        for doc in &section.documents {
            self.render_document(doc, toc.clone())?;
        }

        for subsection in &section.subsections {
            self.render_content_section(subsection, toc.clone())?;
        }

        // copy other files
        tracing::trace!("section_info={:?}", section.file_info);
        let d_output = if let Some(relative_path) = &section.file_info.relative_path {
            Path::new(&self.config.output_directory).join(relative_path)
        } else {
            tracing::warn!(
                "no 'content' found in {}, write to '{}' directly",
                &section.file_info.full_path.display(),
                self.config.output_directory
            );
            Path::new(&self.config.output_directory).to_path_buf()
        };

        if !d_output.is_dir() {
            fs::create_dir_all(&d_output)?;
        }

        let directory = &section.file_info.full_path;
        for entry in fs::read_dir(directory)? {
            let from = entry?.path();
            let from_filename = from.file_name().expect("xx").to_string_lossy().to_string();

            if from.is_file()
                && from.extension() != Some(OsStr::new("org"))
                && (!from_filename.starts_with(&['.', '#']))
            {
                let to = d_output.join(from.file_name().unwrap());
                tracing::trace!(from=?from, to=?to, "copy");
                fs::copy(from, to)?;
            }
        }

        Ok(String::new())
    }

    fn render_document(
        &mut self,
        document: &Document,
        toc: Option<TableOfContents>,
    ) -> std::io::Result<String> {
        tracing::trace!(full_path=?document.file_info.full_path, "render_document():");

        let toc = if let Some(toc) = toc {
            toc.to_html_nav(Some(document.html_path().as_str()))
        } else {
            "".to_string()
        };

        let org_file = &document.ast;

        self.footnote_defintions = org_file.footnote_definitions.clone();
        let mut output = String::new();
        if let Some(section) = &org_file.zeroth_section {
            output.push_str(&self.render_section(section));
        }

        for subtree in &org_file.heading_subtrees {
            output.push_str(&self.render_heading_subtree(subtree));
        }

        let title = match org_file.keywords.get("title") {
            Some(objects) => objects
                .iter()
                .map(|e| self.render_object(e))
                .collect::<String>(),
            None => String::from(""),
        };

        let post_amble = format!(r##"<div id="postamble" class="status"> </div>"##,);

        let automatic_equation_numbering = true;
        let aen = if automatic_equation_numbering {
            r##"<script>
    window.MathJax = {
      tex: {
       tags: 'ams'
      }
    };

    window.onload = function() {
      document.querySelectorAll('div.code pre.src code').forEach(el => {
        hljs.highlightElement(el);
      });
    };
    </script>"##
        } else {
            r##"<script>
    window.onload = function() {
      document.querySelectorAll('div.code pre.src code').forEach(el => {
        hljs.highlightElement(el);
      });
    };
    </script>"##
        };

        let myhtml = format!(
            r##"<!DOCTYPE html>
<html>
  <head>
    <meta http-equiv="Content-Type" content="text/html;charset=utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="generator" content="Org Mode">

    <title>{}</title>

    <script defer src="https://cdn.jsdelivr.net/npm/mathjax@4/tex-mml-chtml.js"></script>

    <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/styles/default.min.css">
    <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/highlight.min.js"></script>

    <style type="text/css">
    {}
    </style>
    {}
  </head>
  <body>

  <div id="content" class="content">
    <div id="table-of-contents" role="doc-toc">
      <div id="text-table-of-contents" role="doc-toc">
      {}
      </div>
    </div>
  {}
  </div>
  {}
  </body>
</html>"##,
            title, self.config.css, aen, toc, output, post_amble
        );

        let f_output_html = Path::new(&self.config.output_directory).join(document.html_path());
        let d_output_html = f_output_html
            .parent()
            .expect("should have parent directory");
        if !d_output_html.is_dir() {
            fs::create_dir_all(&d_output_html)?;
        }
        tracing::trace!(f_output_html = ?f_output_html.display());
        fs::write(&f_output_html, &myhtml)?;

        let f_ast = f_output_html.parent().unwrap().join(
            f_output_html
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
                .replace(".html", "_ast.json"),
        );
        fs::write(&f_ast, format!("{:#?}", document.ast));
        let f_syntax = f_output_html.parent().unwrap().join(
            f_output_html
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
                .replace(".html", "_syntax.json"),
        );
        fs::write(&f_syntax, format!("{:#?}", document.syntax_tree));

        Ok(myhtml)
    }

    fn render_section(&mut self, section: &Section) -> String {
        section
            .elements
            .iter()
            .map(|c| self.render_element(c))
            .collect::<String>()
    }

    fn render_heading_subtree(&mut self, heading: &HeadingSubtree) -> String {
        let title = heading
            .title
            .iter()
            .map(|e| self.render_object(e))
            .collect::<String>();

        if heading.is_commented {
            // respect comment in heading
            return String::from("");
        }

        let todo_html = if let Some(todo) = &heading.keyword {
            let class_1 = match todo.as_str().to_uppercase().as_str() {
                "DONE" => "done",
                "TODO" => "todo",
                _ => "todo",
            };
            format!(
                r#"<span class="{} {}">{}</span> "#,
                class_1,
                escape_html(todo),
                escape_html(todo)
            )
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
            if title == "Footnotes" {
                let elements = self
                    .footnote_defintions
                    .iter()
                    .map(|e| element::Element::FootnoteDefinition(e.clone()))
                    .collect::<Vec<_>>();
                let section = Section { elements };
                self.render_section(&section)
            } else {
                self.render_section(&section)
            }
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
  {section}
  {content}
</div>
 "##,
            level = heading.level + 1,
            title = escape_html(&title),
            todo = todo_html,
            tags = tags_html,
            section = section_html,
            content = content
        )
    }

    fn render_element(&mut self, element: &Element) -> String {
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
            Element::Comment(_) => self.render_comment(),
            Element::FixedWidth(fixed_width) => self.render_fixed_width(fixed_width),

            Element::Item(item) => self.render_item(item),
            Element::FootnoteDefinition(footnote_definition) => {
                // String::from("")
                self.render_footnote_definition(footnote_definition)
            }
            Element::HorizontalRule(_) => self.render_horizontal_rule(),
            Element::Keyword(keyword) => self.render_keyword(keyword),
            Element::LatexEnvironment(env) => self.render_latex_environment(env),

            _ => String::from(""),
            // AstElement::List(list) => self.render_list(list),

            // AstElement::HorizontalRule => "<hr/>\n".to_string(),
            // ... 其他元素渲染
        }
    }

    fn render_table(&mut self, table: &Table) -> String {
        let caption = if table.caption.len() > 0 {
            self.table_counter = self.table_counter + 1;
            format!(
                r##"<caption class="t-above"> <span class="table-number">Table {}:</span> {} </caption>"##,
                self.table_counter,
                table
                    .caption
                    .iter()
                    .map(|e| self.render_object(e))
                    .collect::<String>()
            )
        } else {
            String::from("")
        };

        let header = table
            .header
            .is_empty()
            .not()
            .then(|| {
                format!(
                    r##"<thead>
{}
</thead>"##,
                    table
                        .header
                        .iter()
                        .map(|r| self.render_table_row(r))
                        .collect::<String>()
                )
            })
            .unwrap_or_default();

        format!(
            r##"
 <table border="2">
 {}
 {}
 {}</table>
 "##,
            caption,
            header,
            table
                .rows
                .iter()
                .map(|r| self.render_table_row(r))
                .collect::<String>()
        )
    }

    fn render_table_row(&self, table_row: &TableRow) -> String {
        match table_row.row_type {
            TableRowType::Data | TableRowType::Header => format!(
                "  <tr>{}</tr>\n",
                table_row
                    .cells
                    .iter()
                    .map(|e| self.render_object(&e))
                    .collect::<String>()
            ),

            _ => String::new(),
        }
    }

    fn render_drawer(&mut self, drawer: &Drawer) -> String {
        drawer
            .contents
            .iter()
            .map(|c| self.render_element(c))
            .collect()
    }

    fn render_comment(&self) -> String {
        String::from("")
    }

    pub(crate) fn render_object(&self, object: &Object) -> String {
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

            Object::GeneralLink(GeneralLink {
                protocol,
                description,
                path,
                is_image,
            }) => {
                let desc = if description.len() == 0 {
                    path
                } else {
                    &description
                        .iter()
                        .map(|e| self.render_object(e))
                        .collect::<String>()
                };

                if protocol == "fuzzy" {
                    format!(r##"<a href="#{}">{}</a>"##, path, desc)
                } else if description.len() == 0 && *is_image {
                    let path_html = if path.starts_with("file:") {
                        path.strip_prefix("file:").unwrap()
                    } else {
                        path
                    };

                    format!(
                        r##"<img src="{}" alt="{}">"##,
                        path_html,
                        path.split("/").last().expect("todo")
                    )
                } else {
                    format!(r##"<a href="{}">{}</a>"##, path, desc)
                }
            }

            Object::TableCell(table_cell) => {
                let contents = table_cell
                    .contents
                    .iter()
                    .map(|e| self.render_object(e))
                    .collect::<String>();

                match table_cell.cell_type {
                    TableCellType::Header => format!(r##" <th>{}</th> "##, contents),
                    TableCellType::Data => format!(r##" <td>{}</td> "##, contents),
                }
            }

            Object::Target(text) => {
                format!(r##"<a id="{text}"></a>"##)
            }

            Object::Timestamp(text) => {
                format!(
                    r##"<span class="timestamp-wrapper">
  <span class="timestamp">{}
  </span>
</span>
"##,
                    text.replace("--", "-")
                )
            }

            Object::FootnoteReference {
                label,
                nid: _,
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

            Object::RadioTarget(objects) => {
                let inner: String = objects.iter().map(|o| self.render_object(o)).collect();
                format!(r##"<a id="{inner}">{inner}</a>"##)
            }

            Object::RadioLink(objects) => {
                let inner: String = objects.iter().map(|o| self.render_object(o)).collect();
                format!(r##"<a href="#{inner}">{inner}</a>"##)
            }

            Object::Subscript(objects) => {
                let inner: String = objects.iter().map(|o| self.render_object(o)).collect();
                format!(r##"<sub>{}</sub>"##, inner)
            }

            Object::Superscript(objects) => {
                let inner: String = objects.iter().map(|o| self.render_object(o)).collect();
                format!(r##"<sup>{}</sup>"##, inner)
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

                None => String::from(content),
            },

            Object::StatisticsCookie(value) => {
                format!("{}", value)
            }

            Object::ExportSnippet { backend, value } => {
                if backend == "html" {
                    format!("{}", value)
                } else {
                    // ignore other backend
                    format!("")
                }
            }

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
    // image
    // table
    fn render_paragraph(&mut self, paragraph: &Paragraph) -> String {
        let caption = paragraph
            .affiliated_keywords
            .iter()
            .filter(|e| e.key.to_uppercase() == "CAPTION")
            .map(|e| {
                e.value
                    .iter()
                    .map(|ee| self.render_object(ee))
                    .collect::<Vec<String>>()
                    .join("")
            })
            .collect::<Vec<String>>()
            .join(" ");

        let is_figure_only_paragrah = paragraph.objects.iter().count() == 1
            && paragraph
                .objects
                .iter()
                .filter(|e| match e {
                    Object::GeneralLink(GeneralLink {
                        protocol,
                        path,
                        description,
                        is_image,
                    }) if description.len() == 0 && *is_image => true,
                    _ => false,
                })
                .count()
                == 1
            && paragraph
                .objects
                .iter()
                .all(|e| matches!(e, Object::GeneralLink { .. }));

        let contents: String = paragraph
            .objects
            .iter()
            .map(|object| self.render_object(object))
            .collect();

        if is_figure_only_paragrah {
            let attr_html = paragraph
                .affiliated_keywords
                .iter()
                .filter(|e| e.key.to_uppercase() == "ATTR_HTML")
                .map(|e| {
                    e.value
                        .iter()
                        .map(|ee| self.render_object(ee))
                        .collect::<Vec<String>>()
                        .join("")
                })
                .collect::<Vec<String>>()
                .iter()
                .flat_map(|e| {
                    e.split(":")
                        .map(|ee| ee.trim())
                        .filter(|ee| ee.len() > 0)
                        .map(|ee| ee.split_once(" "))
                })
                .filter(|e| e.is_some())
                .map(|e| (e.unwrap().0.to_string(), e.unwrap().1.to_string()))
                .collect::<HashMap<String, String>>()
                .iter()
                .map(|(k, v)| format!(r##"{k}="{v}""##))
                .collect::<Vec<String>>()
                .join(" ");

            self.figure_counter = self.figure_counter + 1;

            let path = match &paragraph.objects[0] {
                Object::GeneralLink(GeneralLink { path, .. }) => path,
                _ => unreachable!(),
            };

            let path_html = if path.starts_with("file:") {
                path.strip_prefix("file:").unwrap()
            } else {
                path
            };

            format!(
                r##"<div class="figure">
<p> <img src="{}" {}></p>
<p> <span class="figure-number">Figure {}: </span> {}</p>
</div>
"##,
                path_html, attr_html, self.figure_counter, caption,
            )
        } else {
            format!(
                r##"<p>{}
</p>
"##,
                contents
            )
        }
    }

    // fixme: link: collect all footnotes into a div
    fn render_footnote_definition(&mut self, footnote_definition: &FootnoteDefinition) -> String {
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
  <a id="fn.{label}">{nid}</a>: {c}
  <div class="footpara" role="doc-footnote">
   {label} := {def}
  </div>
</div>
"##,
            label = footnote_definition.label,
            nid = footnote_definition.nid,
            c = c,
            def = footnote_definition
                .contents
                .iter()
                .map(|e| self.render_element(e))
                .collect::<String>()
                .replace("<p>", r##"<p class="footpara">"##)
        )
    }

    fn render_center_block(&mut self, block: &CenterBlock) -> String {
        format!(
            r##"<div class="org-center">
{}</div>
"##,
            block
                .contents
                .iter()
                .map(|e| self.render_element(e))
                .collect::<String>()
        )
    }

    fn render_quote_block(&mut self, block: &QuoteBlock) -> String {
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

    fn render_special_block(&mut self, block: &SpecialBlock) -> String {
        format!(
            r##"<div class="{}">
{}</div>
"##,
            block.name,
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

    fn render_src_block(&self, block: &SrcBlock) -> String {
        let s = block
            .contents
            .iter()
            .map(|e| self.render_object(e))
            .collect::<String>();
        format!(
            r##"<div class="code org-src-container"><pre class="src src-{}"><code class="language-{}">{}</code></pre></div>"##,
            block.language, block.language, s
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

    fn render_comment_block(&self, _block: &CommentBlock) -> String {
        format!(r##""##)
    }

    fn render_fixed_width(&self, block: &FixedWidth) -> String {
        format!(
            r##"<pre class="example">{}</pre>
"##,
            block.text.replace("\n", "<br>\n")
        )
    }

    fn render_keyword(&self, keyword: &Keyword) -> String {
        match keyword.key.as_str() {
            "title" => format!(
                r##"<h1 class="title">{}</h1>"##,
                keyword
                    .value
                    .iter()
                    .map(|e| self.render_object(e))
                    .collect::<String>()
            ),
            _ => format!(r##""##),
        }
    }

    fn render_list(&mut self, list: &List) -> String {
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
                    r##"<dl>
{}</dl>
"##,
                    list.items
                        .iter()
                        .map(|i| self.render_item(&i))
                        .collect::<String>()
                )
            }
        }
    }

    fn render_item(&mut self, item: &Item) -> String {
        let checkbox_html = match &item.checkbox {
            None => String::from(""),
            Some(e) => format!("<code>{e}</code>"),
        };

        let contents_html = if item.contents.len() == 1 {
            item.contents
                .iter()
                .map(|i| self.render_element(&i))
                .collect::<String>()
                .as_str()
                // .trim_prefix("<p>")
                // .trim_suffix("</p>")
                .replacen("<p>", "", 1)
                .replacen("</p>", "", 1)
                .to_string()
        } else {
            item.contents
                .iter()
                .map(|i| self.render_element(&i))
                .collect::<String>()
        };

        let tag_html = item
            .tag
            .iter()
            .map(|e| self.render_object(e))
            .collect::<String>();

        format!(
            r##"  <dt>{} {}</dt> <dd>{}</dd>
"##,
            checkbox_html, tag_html, contents_html,
        )

        //         match &item.tag {
        //             None => format!(
        //                 r##"  <li>
        // {} {}  </li>
        // "##,
        //                 checkbox_html, contents_html
        //             ),

        //             Some(tag) => {
        //                 format!(
        //                     r##"  <dt>{} {}</dt> <dd>{}</dd>
        // "##,
        //                     checkbox_html, tag, contents_html,
        //                 )
        //             }
        //         }
    }

    fn render_horizontal_rule(&self) -> String {
        format!(
            r##"<hr>
"##
        )
    }

    fn render_latex_environment(&self, latex_environment: &LatexEnvironment) -> String {
        format!(
            r##"{}
"##,
            latex_environment.text
        )
    }
}

// HTML转义工具函数
fn escape_html(text: &str) -> String {
    // html_escape::encode_text(text).to_string()
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::{Renderer, RendererConfig};
    use crate::compiler::Compiler;
    use tracing_subscriber::FmtSubscriber;

    #[test]
    fn test_render_document() {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("set global subscripber failed");

        let f_org = "tests/test.org";
        let mut renderer = Renderer::new(RendererConfig::default());
        renderer.build_file(f_org);
    }

    #[test]
    fn test_render_content_section() {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(tracing::Level::TRACE)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("set global subscripber failed");

        let d_org = "tests";
        // let d_org = "/tmp/content";
        let d_org = "/home/touch/note/content";
        let mut renderer = Renderer::new(RendererConfig::default());
        renderer.build(d_org);
        // let section = compiler.compile_section(d_org).expect("no Document compiled");
        // renderer.render_content_section(&section);
    }
}
