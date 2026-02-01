//! Render document for SSG
//! Site -> Output directory (Using Tera)

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
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use html_escape;

use crate::compiler::ast_builder::element::{
    self, CenterBlock, Drawer, Element, ExampleBlock, ExportBlock, FixedWidth, FootnoteDefinition,
    HeadingSubtree, Item, Keyword, LatexEnvironment, List, ListType, OrgFile, Paragraph,
    QuoteBlock, Section, SpecialBlock, SrcBlock, Table, TableRow, TableRowType, VerseBlock,
};
use crate::compiler::ast_builder::object::{GeneralLink, Object, TableCellType};
use crate::constants::entity::ENTITYNAME_TO_HTML;
use crate::export::ssg::site::{Page, PageId, Site};
use crate::export::ssg::toc::TableOfContents;
use crate::export::ssg::view_model::{PageNavContext, TableViewModel};

pub struct RendererContext {
    pub table_counter: usize,
    pub figure_counter: usize,
    pub tera: tera::Tera,
    pub toc: TableOfContents,
    pub pageid_url: HashMap<PageId, String>,
}

impl Default for RendererContext {
    fn default() -> Self {
        let mut tera = match tera::Tera::new("src/export/ssg/templates/**/*.html") {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("template error: {}", e);
                ::std::process::exit(1);
            }
        };
        tera.autoescape_on(vec![]);

        Self {
            tera,
            table_counter: 0,
            figure_counter: 0,
            toc: TableOfContents::default(),
            pageid_url: HashMap::new(),
        }
    }
}

// context: prev / next
pub struct Renderer {
    config: RendererConfig,
    footnote_defintions: Vec<FootnoteDefinition>,
    context: RendererContext,
}

#[derive(Debug, Clone)]
pub struct RendererConfig {
    pub output_directory: String,
    pub automatic_equaiton_numbering: bool,
    pub css: String, // path of css file
                     // pub class_prefix: String,
                     // pub highlight_code_blocks: bool,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            css: include_str!("static/default.css").to_string(),
            output_directory: "public".to_string(),
            automatic_equaiton_numbering: true,
        }
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            config: RendererConfig::default(),
            context: RendererContext::default(),
            footnote_defintions: vec![],
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
            footnote_defintions: vec![],
            context: RendererContext::default(),
        }
    }

    pub fn render_site(&mut self, site: &Site) {
        tracing::debug!("  render site todo");
        self.context.toc = site.toc();
        for (id, page) in site.pages.iter() {
            self.context.pageid_url.insert(id.clone(), page.url.clone());
        }

        for (_id, page) in site.pages.iter() {
            self.render_page(page).expect("render_page should success");
        }
    }

    fn render_page(&mut self, page: &Page) -> std::io::Result<String> {
        let page_nav_context = PageNavContext::from_page(page, &self.context.pageid_url);
        let mut ctx = tera::Context::from_serialize(page_nav_context)
            .expect("render_page: from serialize failed");
        ctx.insert("title", &page.title);
        ctx.insert(
            "toc",
            &self.context.toc.to_html_nav(Some(page.url.as_str())),
        );
        ctx.insert(
            "automatic_equaiton_numbering",
            &self.config.automatic_equaiton_numbering,
        );
        let content = self.render_org_file(&page.ast);
        ctx.insert("content", &content);

        let html = self
            .context
            .tera
            .render("page.tera.html", &ctx)
            .unwrap_or_else(|err| format!("Template rendering page failed: {}", err));

        let f_html = Path::new(&self.config.output_directory).join(page.html_path.as_str());
        let d_html = f_html.parent().expect("should have parent directory");
        if !d_html.is_dir() {
            fs::create_dir_all(d_html)?;
        }
        fs::write(&f_html, &html)?;

        let f_ast = f_html.parent().unwrap().join(
            f_html
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
                .replace(".html", "_ast.json"),
        );
        fs::write(&f_ast, format!("{:#?}", page.ast))?;
        let f_syntax = f_html.parent().unwrap().join(
            f_html
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string()
                .replace(".html", "_syntax.json"),
        );
        fs::write(&f_syntax, format!("{:#?}", page.syntax_tree))?;

        Ok(String::from(""))
    }

    fn render_org_file(&mut self, org_file: &OrgFile) -> String {
        self.footnote_defintions = org_file.footnote_definitions.clone();

        let mut output = String::new();
        if let Some(section) = &org_file.zeroth_section {
            output.push_str(&self.render_section(section));
        }

        for subtree in &org_file.heading_subtrees {
            output.push_str(&self.render_heading_subtree(subtree));
        }

        output
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
            .map(|e| Self::render_object(e))
            .collect::<String>();

        if heading.is_commented {
            // respect comment in heading
            return String::from("");
        }

        let id_html = if let Some(id) = heading.properties.get("ID") {
            format!(r##"id="{}""##, id)
        } else {
            format!("")
        };

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
  <h{level} {id_html}>
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
            content = content,
            id_html = id_html,
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
            Element::ExampleBlock(example_block) => Self::render_example_block(example_block),
            Element::ExportBlock(export_block) => Self::render_export_block(export_block),
            Element::CommentBlock(_comment_block) => Self::render_comment_block(),
            Element::SrcBlock(src_block) => Self::render_src_block(src_block),
            Element::VerseBlock(verse_block) => Self::render_verse_block(verse_block),

            Element::List(list) => self.render_list(list),
            Element::Comment(_) => Self::render_comment(),
            Element::FixedWidth(fixed_width) => Self::render_fixed_width(fixed_width),

            Element::Item(item) => self.render_item(item),
            Element::FootnoteDefinition(footnote_definition) => {
                // String::from("")
                self.render_footnote_definition(footnote_definition)
            }
            Element::HorizontalRule(_) => Self::render_horizontal_rule(),
            Element::Keyword(keyword) => self.render_keyword(keyword),
            Element::LatexEnvironment(env) => Self::render_latex_environment(env),

            _ => String::from(""),
            // AstElement::List(list) => self.render_list(list),

            // AstElement::HorizontalRule => "<hr/>\n".to_string(),
            // ... 其他元素渲染
        }
    }

    fn render_table(&mut self, table: &Table) -> String {
        let table_view_model = TableViewModel::from_ast_v2(table, &mut self.context);
        let ctx = tera::Context::from_serialize(&table_view_model)
            .expect("render_table: from serialize failed");
        self.context
            .tera
            .render("table.tera.html", &ctx)
            .unwrap_or_else(|err| format!("Template rendering table failed: {}", err))
    }

    pub(crate) fn render_table_row(table_row: &TableRow) -> String {
        match table_row.row_type {
            TableRowType::Data | TableRowType::Header => format!(
                "<tr>{}</tr>\n",
                table_row
                    .cells
                    .iter()
                    .map(|e| Self::render_object(&e))
                    .collect::<String>()
            ),

            _ => String::new(),
        }
    }

    fn render_comment() -> String {
        String::from("")
    }

    pub(crate) fn render_object(object: &Object) -> String {
        match object {
            Object::Text(text) => html_escape::encode_text(text).to_string(),

            Object::Bold(objects) => {
                let inner: String = objects.iter().map(|o| Self::render_object(o)).collect();
                format!("<b>{}</b>", inner)
            }
            Object::Italic(objects) => {
                let inner: String = objects.iter().map(|o| Self::render_object(o)).collect();
                format!("<i>{}</i>", inner)
            }

            Object::Underline(objects) => {
                let inner: String = objects.iter().map(|o| Self::render_object(o)).collect();
                format!(r##"<span class="underline">{}</span>"##, inner)
            }

            Object::Strikethrough(objects) => {
                let inner: String = objects.iter().map(|o| Self::render_object(o)).collect();
                format!(r##"<del>{}</del>"##, inner)
            }

            Object::Code(objects) => {
                let inner: String = objects.iter().map(|o| Self::render_object(o)).collect();
                format!(r##"<code>{}</code>"##, inner)
            }

            Object::Verbatim(objects) => {
                let inner: String = objects.iter().map(|o| Self::render_object(o)).collect();
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
                        .map(|e| Self::render_object(e))
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
                } else if protocol == "id" {
                    // fixme: if roam id is in other file
                    let href = format!("#{}", path.strip_prefix("id:").expect("id:"));
                    format!(r##"<a href="{}">{}</a>"##, href, desc)
                } else {
                    format!(r##"<a href="{}">{}</a>"##, path, desc)
                }
            }

            Object::TableCell(table_cell) => {
                let contents = table_cell
                    .contents
                    .iter()
                    .map(|e| Self::render_object(e))
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
                    r##"<span class="timestamp-wrapper"><span class="timestamp">{}</span></span>"##,
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
                    r##"<sup><a id="fnr.{label}.{label_rid}" class="footref" href="#fn.{label}" role="doc-backlink">{label}</a></sup>"##,
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
                let inner: String = objects.iter().map(|o| Self::render_object(o)).collect();
                format!(r##"<a id="{inner}">{inner}</a>"##)
            }

            Object::RadioLink(objects) => {
                let inner: String = objects.iter().map(|o| Self::render_object(o)).collect();
                format!(r##"<a href="#{inner}">{inner}</a>"##)
            }

            Object::Subscript(objects) => {
                let inner: String = objects.iter().map(|o| Self::render_object(o)).collect();
                format!(r##"<sub>{}</sub>"##, inner)
            }

            Object::Superscript(objects) => {
                let inner: String = objects.iter().map(|o| Self::render_object(o)).collect();
                format!(r##"<sup>{}</sup>"##, inner)
            }

            Object::LatexFragment {
                content,
                display_mode,
            } => match display_mode {
                Some(true) => {
                    format!(r##"\[ {} \]"##, content)
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

    fn render_example_block(block: &ExampleBlock) -> String {
        format!(
            r##"<pre class="example">{}</pre>"##,
            block
                .contents
                .iter()
                .map(|e| Self::render_object(e))
                .collect::<String>()
        )
    }

    fn render_verse_block(block: &VerseBlock) -> String {
        format!(
            r##"<p class="verse">{}</p>"##,
            block
                .contents
                .iter()
                .map(|e| Self::render_object(e))
                .collect::<String>()
        )
    }

    fn render_src_block(block: &SrcBlock) -> String {
        let s = block
            .contents
            .iter()
            .map(|e| Self::render_object(e))
            .collect::<String>();
        format!(
            r##"<div class="code org-src-container"><pre class="src src-{}"><code class="language-{}">{}</code></pre></div>"##,
            block.language, block.language, s
        )
    }

    // FIXME: only supoort html now
    fn render_export_block(block: &ExportBlock) -> String {
        format!(
            r##"{}"##,
            block
                .contents
                .iter()
                .map(|e| Self::render_object(e))
                .collect::<String>()
        )
    }

    fn render_comment_block() -> String {
        format!(r##""##)
    }

    fn render_fixed_width(block: &FixedWidth) -> String {
        format!(
            r##"<pre class="example">{}</pre>"##,
            block.text.replace("\n", "<br>\n")
        )
    }

    fn render_horizontal_rule() -> String {
        format!(r##"<hr>"##)
    }

    fn render_latex_environment(latex_environment: &LatexEnvironment) -> String {
        format!(r##"{}"##, latex_environment.text)
    }

    fn render_drawer(&mut self, drawer: &Drawer) -> String {
        drawer
            .contents
            .iter()
            .map(|c| self.render_element(c))
            .collect()
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
                    .map(|ee| Self::render_object(ee))
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
            .map(|object| Self::render_object(object))
            .collect();

        if is_figure_only_paragrah {
            let attr_html = paragraph
                .affiliated_keywords
                .iter()
                .filter(|e| e.key.to_uppercase() == "ATTR_HTML")
                .map(|e| {
                    e.value
                        .iter()
                        .map(|ee| Self::render_object(ee))
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

            self.context.figure_counter = self.context.figure_counter + 1;

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
                path_html, attr_html, self.context.figure_counter, caption,
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

    // FIXME: roam id is missed here!!
    fn render_keyword(&self, keyword: &Keyword) -> String {
        match keyword.key.as_str() {
            "title" => format!(
                r##"<h1 class="title">{}</h1>"##,
                keyword
                    .value
                    .iter()
                    .map(|e| Self::render_object(e))
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
            .map(|e| Self::render_object(e))
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
}

// HTML转义工具函数
fn escape_html(text: &str) -> String {
    // html_escape::encode_text(text).to_string()
    text.to_string()
}
