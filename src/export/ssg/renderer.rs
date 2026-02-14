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
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

use chrono::{Datelike, Local};
use html_escape;
use serde::Deserialize;

use crate::compiler::ast_builder::element::{
    self, CenterBlock, Drawer, Element, ExampleBlock, ExportBlock, FixedWidth, FootnoteDefinition,
    HeadingSubtree, Id, Item, Keyword, LatexEnvironment, List, ListType, OrgFile, Paragraph,
    QuoteBlock, Section, SpecialBlock, SrcBlock, Table, TableRow, TableRowType, VerseBlock,
};
use crate::compiler::ast_builder::object::{GeneralLink, Object, TableCellType};
use crate::constants::entity::ENTITYNAME_TO_HTML;
use crate::export::ssg::site::{Page, PageId, Site};
use crate::export::ssg::toc::{TableOfContents, TocNode};
use crate::export::ssg::view_model::{PageNavContext, TableViewModel};

fn hash_string_to_usize(input: &str) -> usize {
    let mut hasher = DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish() as usize
}

pub struct RendererContext {
    tera: tera::Tera,
    // table counter in sit or page?
    pub table_counter: usize,
    pub figure_counter: usize,
    pub toc: TableOfContents,
    pub pageid_to_url: HashMap<PageId, String>,
    pub roamid_to_url: HashMap<String, String>,
    pub prev_head_level: Vec<u8>,
}

impl Default for RendererContext {
    fn default() -> Self {
        let tera_dir = "src/export/ssg/templates/**/*.html";
        let mut tera = match tera::Tera::new(tera_dir) {
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
            pageid_to_url: HashMap::default(),
            roamid_to_url: HashMap::default(),
            prev_head_level: vec![0],
        }
    }
}

// context: prev / next
pub struct Renderer {
    config: RendererConfig,
    footnote_defintions: Vec<FootnoteDefinition>,
    context: RendererContext,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(default)]
pub struct RendererConfig {
    pub output_directory: PathBuf,
    pub input_directory: PathBuf,
    pub automatic_equaiton_numbering: bool,

    /// debug mode: true will generate syntax_tree/ast data in json format
    pub debug: bool,
    pub bgcolor_for_white: Vec<String>,
}

impl Default for RendererConfig {
    fn default() -> Self {
        Self {
            output_directory: "public".into(),
            input_directory: "content".into(),
            automatic_equaiton_numbering: true,
            debug: false,
            bgcolor_for_white: [
                "#330000", "#331900", "#333300", "#193300", "#003300", "#003319", "#003333",
                "#001933", "#000033", "#190033", "#330033", "#330019", "#4D0000", "#4D2600",
                "#4D4D00", "#264D00", "#004D00", "#004D26", "#004D4D", "#00264D", "#00004D",
                "#26004D", "#4D004D", "#4D0026", "#660000", "#663300", "#666600", "#336600",
                "#006600", "#006633", "#006666", "#003366", "#000066", "#330066", "#660066",
                "#660033",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }
}

fn load_templates_from_dir(dir: &str) -> std::io::Result<HashMap<String, String>> {
    let mut templates = HashMap::new();
    let path = Path::new(dir);

    if !path.exists() {
        return Ok(templates);
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |ext| ext == "html") {
            if let Some(stem) = path.file_name().and_then(|s| s.to_str()) {
                let content = fs::read_to_string(&path)?;
                templates.insert(stem.to_string(), content);
            }
        }
    }

    Ok(templates)
}

fn build_merged_tera(default_dir: &str, user_dir: &str) -> std::io::Result<tera::Tera> {
    let mut tera = tera::Tera::default();
    let mut all_templates = HashMap::new();

    let default_tmpls = load_templates_from_dir(default_dir)?;
    all_templates.extend(default_tmpls);

    let user_tmpls = load_templates_from_dir(user_dir)?;
    all_templates.extend(user_tmpls);

    let name = "macros.tera.html";
    if let Some(content) = all_templates.remove(name) {
        tera.add_raw_template(name, &content)
            .expect("add raw template");
    }

    for (name, content) in all_templates {
        tera.add_raw_template(&name, &content)
            .expect("add raw template");
    }

    Ok(tera)
}

impl Default for Renderer {
    fn default() -> Self {
        let config = RendererConfig::default();
        let input_directory = std::path::Path::new(&config.input_directory);

        let default_dir = "src/export/ssg/templates";
        let binding = input_directory
            .parent()
            .expect("parent should exists")
            .join("templates");
        let user_dir = binding.to_str().expect("todo");
        let mut tera = build_merged_tera(default_dir, user_dir).expect("");
        tera.autoescape_on(vec![]);

        Self {
            config,
            context: RendererContext {
                tera,
                ..RendererContext::default()
            },
            footnote_defintions: vec![],
        }
    }
}

impl Renderer {
    fn get_toc_of_page(&mut self, page: &Page) -> TableOfContents {
        self.context.prev_head_level = vec![0];
        let mut children = vec![];
        for heading_subtree in page.ast.heading_subtrees.iter() {
            if !heading_subtree.is_commented {
                children.push(self.get_toc_of_heading_subtree(heading_subtree));
            }
        }
        TableOfContents::new(children)
    }

    fn get_toc_of_heading_subtree(&mut self, heading: &HeadingSubtree) -> TocNode {
        if heading.level > self.context.prev_head_level.len() as u8 {
            self.context.prev_head_level.push(1);
        } else if heading.level == self.context.prev_head_level.len() as u8 {
            let tmp = self.context.prev_head_level.pop().unwrap();
            self.context.prev_head_level.push(tmp + 1);
        } else {
            while heading.level < self.context.prev_head_level.len() as u8 {
                let _ = self.context.prev_head_level.pop().unwrap();
            }
            let tmp = self.context.prev_head_level.pop().unwrap();
            self.context.prev_head_level.push(tmp + 1);
        }
        let index = self
            .context
            .prev_head_level
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join(".");

        let title = heading
            .title
            .iter()
            .map(|e| self.render_object(e))
            .collect::<String>();
        let title = format!("{} {}", index, title);

        let path = format!("#{}", heading.id());
        let level = heading.level;

        let mut children = vec![];
        for sub_heading in heading.sub_heading_subtrees.iter() {
            if !sub_heading.is_commented {
                children.push(self.get_toc_of_heading_subtree(sub_heading));
            }
        }

        TocNode {
            title,
            path,
            level: level.into(),
            children,
        }
    }

    pub fn slugify(s: String) -> String {
        s.to_ascii_lowercase()
            .split(&['-', '_', ' '])
            .collect::<Vec<_>>()
            .join("-")
    }

    pub fn new(config: RendererConfig) -> Self {
        let input_directory = std::path::Path::new(&config.input_directory);
        let default_dir = "src/export/ssg/templates";
        let binding = input_directory
            .parent()
            .expect("parent should exists")
            .join("templates");
        let user_dir = binding.to_str().expect("todo");
        let mut tera = build_merged_tera(default_dir, user_dir).expect("");
        tera.autoescape_on(vec![]);

        Self {
            config,
            context: RendererContext {
                tera,
                ..RendererContext::default()
            },
            footnote_defintions: vec![],
        }
    }

    pub fn render_site(&mut self, site: &Site) {
        tracing::debug!("  render site todo");
        self.context.toc = site.toc();
        self.context.pageid_to_url = site.pageid_to_url.clone();
        self.context.roamid_to_url = site.knowledge_graph.id_to_url.clone();

        for (_id, page) in site.pages.iter() {
            self.render_page(page).expect("render_page should success");
        }

        // render tags
        for (tag, page_ids) in site.tag_index.iter() {
            let pages = page_ids
                .iter()
                .map(|e| site.pages.get(e).expect("get page"))
                .collect::<Vec<_>>();
            let mut ctx = tera::Context::new();
            ctx.insert("tag", &tag);

            let url_titles = pages
                .iter()
                .map(|p| (p.url.clone(), p.title.clone()))
                .collect::<Vec<(String, String)>>();
            ctx.insert("url_titles", &url_titles);
            ctx.insert("is_home", &false);
            let now = Local::now();
            let current_year = now.year();
            ctx.insert("current_year", &current_year);

            ctx.insert("toc", &self.context.toc.to_html_nav(None));

            let html = self
                .context
                .tera
                .render("tag.tera.html", &ctx)
                .unwrap_or_else(|err| format!("Template rendering page failed: {}", err));

            let f_html = self
                .config
                .output_directory
                .join(format!("tags/{tag}.html").as_str());
            let d_html = f_html.parent().expect("should have parent directory");
            if !d_html.is_dir() {
                fs::create_dir_all(d_html).expect("create dir");
            }
            fs::write(&f_html, &html).expect("write file");
        }
    }

    pub fn render_page_inner(&mut self, page: &Page) -> String {
        self.context.figure_counter = 0;
        self.context.table_counter = 0;

        let page_nav_context = PageNavContext::from_page(page, &self.context.pageid_to_url);
        let mut ctx = tera::Context::from_serialize(page_nav_context)
            .expect("render_page: from serialize failed");
        ctx.insert("title", &page.title);
        let id = page.ast.properties.get("ID");
        ctx.insert("id", &id);

        let is_home = if page.url == "/index.html" {
            true
        } else {
            false
        };
        ctx.insert("is_home", &is_home);

        let now = Local::now();
        let current_year = now.year();
        ctx.insert("current_year", &current_year);

        let create_date = page.created_ts.map(|e| e.format("%Y-%m-%d").to_string());
        let last_modified_date = page
            .last_modified_ts
            .map(|e| e.format("%Y-%m-%d").to_string());

        ctx.insert("create_date", &create_date);
        ctx.insert("last_modified_date", &last_modified_date);

        ctx.insert(
            "toc",
            &self.context.toc.to_html_nav(Some(page.url.as_str())), // FIXME: this is slow
        );

        ctx.insert(
            "automatic_equaiton_numbering",
            &self.config.automatic_equaiton_numbering,
        );

        let content = self.render_org_file(&page.ast); // 7ms
        ctx.insert("content", &content);

        let toc = self.get_toc_of_page(page).to_html_nav(None);
        ctx.insert("toc_of_current_page", &toc);

        let n_color = self.config.bgcolor_for_white.len();
        let tags = page
            .tags
            .iter()
            .map(|e| {
                (
                    e.to_string(),
                    self.config.bgcolor_for_white[hash_string_to_usize(e) % n_color].clone(),
                )
            })
            .collect::<Vec<_>>();
        tracing::trace!("tags={tags:?}");
        ctx.insert("tags", &tags);

        let html = self
            .context
            .tera
            .render("page.tera.html", &ctx)
            .unwrap_or_else(|err| format!("Template rendering page failed: {}", err));

        html
    }

    fn render_page(&mut self, page: &Page) -> std::io::Result<String> {
        let html = self.render_page_inner(page);

        let f_html = self.config.output_directory.join(page.html_path.as_str());
        let d_html = f_html.parent().expect("should have parent directory");
        if !d_html.is_dir() {
            fs::create_dir_all(d_html)?;
        }
        fs::write(&f_html, &html)?;

        Ok(String::from(""))
    }

    // todo: use tera template
    fn render_org_file(&mut self, org_file: &OrgFile) -> String {
        self.context.prev_head_level = vec![0];

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
        if heading.level > self.context.prev_head_level.len() as u8 {
            self.context.prev_head_level.push(1);
        } else if heading.level == self.context.prev_head_level.len() as u8 {
            let tmp = self.context.prev_head_level.pop().unwrap();
            self.context.prev_head_level.push(tmp + 1);
        } else {
            while heading.level < self.context.prev_head_level.len() as u8 {
                let _ = self.context.prev_head_level.pop().unwrap();
            }
            let tmp = self.context.prev_head_level.pop().unwrap();
            self.context.prev_head_level.push(tmp + 1);
        }
        let index = self
            .context
            .prev_head_level
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<String>>()
            .join(".");

        let title = heading
            .title
            .iter()
            .map(|e| self.render_object(e))
            .collect::<String>();

        if heading.is_commented {
            // respect comment in heading
            return String::from("");
        }

        let id_html = if let Some(id) = heading.properties.get("ID") {
            format!(r##"id="{}""##, id)
        } else {
            format!(r##"id="{}""##, heading.id())
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
            r##"<section class="outline-{level}">
  <h{level} {id_html}> {index} {todo} {title} {tags} </h{level}>
  {section}
  {content}
</section>
"##,
            index = index,
            level = heading.level,
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
            Element::ExampleBlock(example_block) => self.render_example_block(example_block),
            Element::ExportBlock(export_block) => self.render_export_block(export_block),
            Element::CommentBlock(_comment_block) => Self::render_comment_block(),
            Element::SrcBlock(src_block) => self.render_src_block(src_block),
            Element::VerseBlock(verse_block) => self.render_verse_block(verse_block),

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

    pub fn get_table_vm(&mut self, table: &Table) -> TableViewModel {
        let has_caption = !table.caption.is_empty();
        let table_number = if has_caption {
            self.context.table_counter += 1;
            Some(self.context.table_counter)
        } else {
            None
        };
        let caption = table
            .caption
            .iter()
            .map(|e| self.render_object(e))
            .collect::<String>();

        let has_header = !table.header.is_empty();
        let header_rows = table
            .header
            .iter()
            .map(|r| self.render_table_row(r))
            .collect();
        let body_rows = table
            .rows
            .iter()
            .map(|e| self.render_table_row(e))
            .collect();

        TableViewModel {
            table_number,

            has_caption,
            caption,

            has_header,
            header_rows,

            body_rows,
        }
    }

    fn render_table(&mut self, table: &Table) -> String {
        let table_view_model = self.get_table_vm(table);
        let ctx = tera::Context::from_serialize(&table_view_model)
            .expect("render_table: from serialize failed");
        self.context
            .tera
            .render("table.tera.html", &ctx)
            .unwrap_or_else(|err| format!("Template rendering table failed: {}", err))
    }

    pub(crate) fn render_table_row(&self, table_row: &TableRow) -> String {
        match table_row.row_type {
            TableRowType::Data | TableRowType::Header => format!(
                "<tr>{}</tr>\n",
                table_row
                    .cells
                    .iter()
                    .map(|e| self.render_object(&e))
                    .collect::<String>()
            ),

            _ => String::new(),
        }
    }

    fn render_comment() -> String {
        String::from("")
    }

    pub(crate) fn render_object(&self, object: &Object) -> String {
        match object {
            Object::Text(text) => html_escape::encode_text(text).to_string(),

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
                format!(r##"<code class="inline">{}</code>"##, inner)
            }

            Object::Verbatim(objects) => {
                let inner: String = objects.iter().map(|o| self.render_object(o)).collect();
                format!(r##"<code class="inline">{}</code>"##, inner)
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
                        r##"<figure><img src="{}" alt="{}" /></figure>"##,
                        path_html,
                        path.split("/").last().expect("todo")
                    )

                    // format!(
                    //     r##"<img src="{}" alt="{}">"##,
                    //     path_html,
                    //     path.split("/").last().expect("todo")
                    // )
                } else if protocol == "id" {
                    // fixme: if roam id is in other file: in general link, page/url info is missed?
                    let id = path.strip_prefix("id:").expect("id:");

                    let href = if let Some(url) = self.context.roamid_to_url.get(id) {
                        format!("{}", url)
                    } else {
                        tracing::warn!("no url found for {}", id);
                        format!("#{}", id)
                    };
                    format!(r##"<a href="{}">{}</a>"##, href, desc)
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

    fn render_example_block(&self, block: &ExampleBlock) -> String {
        format!(
            r##"<pre class="example">{}</pre>"##,
            block
                .contents
                .iter()
                .map(|e| self.render_object(e))
                .collect::<String>()
        )
    }

    fn render_verse_block(&self, block: &VerseBlock) -> String {
        format!(
            r##"<p class="verse">{}</p>"##,
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

        // strip common start spaces
        let n_start_common_spaces = s
            .lines()
            .filter(|line| line.trim().chars().count() > 0)
            .map(|line| line.chars().take_while(|&c| c == ' ').count())
            .min();
        let s = if let Some(n) = n_start_common_spaces {
            let prefix = " ".repeat(n);
            s.lines()
                .map(|line| line.strip_prefix(&prefix).unwrap_or(line))
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            s
        };

        if let Some(v) = &block.exports {
            match v.as_str() {
                "code" | "both" => {
                    format!(
                        r##"<div class="code org-src-container"><pre class="src src-{}"><code class="language-{} block">{}</code></pre></div>"##,
                        block.language, block.language, s
                    )
                }
                _ => {
                    format!("")
                }
            }
        } else {
            format!(
                r##"<div class="code org-src-container"><pre class="src src-{}"><code class="language-{} block">{}</code></pre></div>"##,
                block.language, block.language, s
            )
        }
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

    fn render_drawer(&mut self, _drawer: &Drawer) -> String {
        String::new()
        // drawer
        //     .contents
        //     .iter()
        //     .map(|c| self.render_element(c))
        //     .collect()
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
                        protocol: _,
                        path: _,
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
                r##"<figure> <img src="{}" alt="{}" {}></p> <figcaption>Figure {}: {}</figcaption> </figure>"##,
                path_html,
                path.split("/").last().expect("todo"),
                attr_html,
                self.context.figure_counter,
                caption
            )

        //             format!(
        //                 r##"<div class="figure">
        // <p> <img src="{}" {}></p>
        // <p> <span class="figure-number">Figure {}: </span> {}</p>
        // </div>
        // "##,
        //                 path_html, attr_html, self.context.figure_counter, caption,
        //             )
        } else {
            format!(r##"<p>{}</p>"##, contents)
        }
    }

    // fixme: link: collect all footnotes into a div
    fn render_footnote_definition(&mut self, footnote_definition: &FootnoteDefinition) -> String {
        let c = if footnote_definition.rids.len() == 1 {
            format!(
                r##"<sup> <a class="footnum" href="#fnr.{label}.{rid}" role="doc-backlink">^</a> </sup> "##,
                label = footnote_definition.label,
                rid = 1
            )
        } else {
            footnote_definition
                .rids
                .iter()
                .map(|rid| {
                    format!(
                        r##"<sup><a class="footnum" href="#fnr.{label}.{rid}" role="doc-backlink">{rid}</a>  </sup>"##,
                        label = footnote_definition.label,
                        rid = rid
                    )
                })
                .collect::<String>()
        };

        format!(
            r##"<div class="footdef"><a id="fn.{label}">{nid}</a>: {c}<div class="footpara" role="doc-footnote">{label} := {def}</div></div>"##,
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
            r##"<div class="org-center">{}</div>"##,
            block
                .contents
                .iter()
                .map(|e| self.render_element(e))
                .collect::<String>()
        )
    }

    fn render_quote_block(&mut self, block: &QuoteBlock) -> String {
        format!(
            r##"<blockquote>{}</blockquote>"##,
            block
                .contents
                .iter()
                .map(|e| self.render_element(e))
                .collect::<String>()
        )
    }

    fn render_special_block(&mut self, block: &SpecialBlock) -> String {
        let maybe_note = if block.name == "note" {
            r##"<p class="note admonition-title" >Note</p>"##
        } else {
            ""
        };

        format!(
            r##"{}<div class="{}">{}</div>"##,
            maybe_note,
            block.name,
            block
                .contents
                .iter()
                .map(|e| self.render_element(e))
                .collect::<String>()
        )
    }

    fn render_keyword(&self, keyword: &Keyword) -> String {
        // title has been rendered in render_org_file() with id
        match keyword.key.to_ascii_uppercase().as_str() {
            "TITLE" => format!(""),
            _ => format!(""),
        }
    }

    fn render_list(&mut self, list: &List) -> String {
        match list.list_type {
            ListType::Unordered => {
                format!(
                    r##"<ul>{}</ul>"##,
                    list.items
                        .iter()
                        .map(|i| self.render_item(&i))
                        .collect::<String>()
                )
            }

            ListType::Ordered => {
                format!(
                    r##"<ol>{}</ol>"##,
                    list.items
                        .iter()
                        .map(|i| self.render_item(&i))
                        .collect::<String>()
                )
            }

            ListType::Descriptive => {
                format!(
                    r##"<dl>{}</dl>"##,
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

        if tag_html.is_empty() {
            format!(r##"  <li> {} {}  </li> "##, checkbox_html, contents_html)
        } else {
            format!(
                r##"  <dt>{} {}</dt> <dd>{}</dd>"##,
                checkbox_html, tag_html, contents_html,
            )
        }
    }
}

// HTML转义工具函数
fn escape_html(text: &str) -> String {
    // html_escape::encode_text(text).to_string()
    text.to_string()
}
