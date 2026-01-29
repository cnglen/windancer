//! Compile the raw org file to `Document' containing AST, file_info and meta_data
//! one org file --parser--> GreenNode --SyntaxNode::new_root()--> SyntaxNode --ast_builder--> AST
pub mod ast_builder;
pub mod content;
pub mod org_roam;
pub mod parser;

use crate::compiler::ast_builder::AstBuilder;
use crate::compiler::content::{Document, DocumentMetadata, FileInfo, Section, SectionMetadata};
use crate::compiler::parser::syntax::{OrgSyntaxKind, SyntaxNode};
use crate::compiler::parser::{
    OrgParser, config::OrgParserConfig, config::OrgUseSubSuperscripts, get_text,
};
use rowan::WalkEvent;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::Path;

pub struct Compiler {
    parser: OrgParser,
    ast_builder: AstBuilder,
}

impl Compiler {
    pub fn new() -> Self {
        let config =
            OrgParserConfig::default().with_use_sub_superscripts(OrgUseSubSuperscripts::Brace);
        let parser = OrgParser::new(config);
        let ast_builder = AstBuilder::new();
        Self {
            parser,
            ast_builder,
        }
    }

    /// Compile `f_org` into `Document`
    pub fn compile_file<P: AsRef<Path>>(
        &self,
        f_org: P,
    ) -> Result<Document, Box<dyn std::error::Error>> {
        let f_org = f_org.as_ref();
        let syntax_tree = self.parser.parse(f_org);
        // tracing::trace!("syntax_tree:{:#?}", syntax_tree);

        let ast = self.ast_builder.build(&syntax_tree, f_org).expect("build");
        let file_info = FileInfo::from(f_org);
        let metadata = Self::get_metadata(&syntax_tree);
        let doc = Document {
            file_info,
            ast,
            metadata,
            syntax_tree,
        };

        Ok(doc)
    }

    // todo
    // collect all links
    // id 引用

    // 有一个SyntaxNode (rowan)
    // 如何收集RoamNode，构建一张基于RoamNode图, 并构建RoamNode的父子关系及ID引用关系。(a，a的儿子b,a的孙子c均是RoamNode, c引用了RoamNode x, a和x, b和x的关系该如何处理？)
    fn get_metadata(syntax_tree: &SyntaxNode) -> DocumentMetadata {
        let mut keyword = std::collections::HashMap::<String, Vec<String>>::new();
        let mut preorder = syntax_tree.preorder();
        while let Some(event) = preorder.next() {
            match event {
                WalkEvent::Enter(element) => {
                    if element.kind() == OrgSyntaxKind::Keyword {
                        let key = element
                            .first_child_by_kind(&|e| e == OrgSyntaxKind::KeywordKey)
                            .expect("must have KeywordKey")
                            .children_with_tokens()
                            .map(|e| e.as_token().expect("todo").text().to_string())
                            .collect::<String>()
                            .to_ascii_uppercase();

                        let value = element
                            .first_child_by_kind(&|e| e == OrgSyntaxKind::KeywordValue)
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

                        if (key != "MACRO") && (!value.is_empty()) {
                            if keyword.contains_key(&key) {
                                keyword.get_mut(&key).expect("has value").push(value);
                            } else {
                                keyword.insert(key, vec![value]);
                            }
                        }
                    }
                }

                _ => {}
            }
        }

        tracing::trace!("keyword={:?}", keyword);

        let title = keyword.remove("TITLE").map(|e| e.join(" "));
        let authors = keyword.remove("AUTHOR").unwrap_or(vec![]);
        let filetags = keyword.remove("FILETAGS").unwrap_or(vec![]);
        let category = keyword.remove("CATEGORY").unwrap_or(vec![]);
        let enable_render = keyword
            .remove("RENDER")
            .map(|e| {
                !e.into_iter()
                    .map(|ee| ee.to_uppercase())
                    .collect::<HashSet<String>>()
                    .contains("NIL")
            })
            .unwrap_or(true);

        DocumentMetadata {
            title,
            authors,
            filetags,
            category,
            enable_render,
            extra: keyword,
            ..DocumentMetadata::default()
        }
    }

    fn has_org_file<P: AsRef<Path>>(path: P) -> bool {
        use walkdir::WalkDir;
        for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
            if entry.metadata().unwrap().is_file() {
                if entry.path().extension().unwrap_or(OsStr::new("")) == "org" {
                    return true;
                }
            }
        }
        false
    }

    pub fn compile_section<P: AsRef<Path>>(
        &self,
        d_org: P,
    ) -> Result<Section, Box<dyn std::error::Error>> {
        let mut documents = vec![];
        let mut subsections = vec![];

        let file_info = FileInfo::from(&d_org);

        for entry in fs::read_dir(d_org)? {
            let entry = entry?;
            let path = entry.path();
            // tracing::debug!("compile_section: {}", path.display());
            let filename = path.file_name().expect("xx").to_string_lossy().to_string();

            if path.is_dir() && (!filename.starts_with(&['.', '#'])) {
                if Self::has_org_file(&path) {
                    tracing::debug!("compile_section@dir: {}", path.display());
                    subsections.push(self.compile_section(path)?);
                }
            } else if path.extension() == Some(OsStr::new("org"))
                && (!filename.starts_with(&['.', '#']))
            {
                tracing::debug!("compile_section@org: {}", path.display());
                documents.push(self.compile_file(path)?);
            }
        }

        // todo: other strategy of order
        // the index page should be placed at first place
        documents.sort_by_key(|doc|  -(doc.file_info.maybe_index as i32));

        Ok(Section {
            file_info,
            documents,
            subsections,
            metadata: SectionMetadata::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Compiler;
    use tracing_subscriber::FmtSubscriber;

    #[test]
    fn test_compile_file() {
        let f_org = "tests/test.org";
        let compiler = Compiler::new();
        let _doc = compiler.compile_file(f_org).expect("no Document compiled");
        // println!("{:#?}", _doc.metadata);
        println!("{:#?}", _doc.file_info);
    }

    #[test]
    fn test_compile_directory() {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(tracing::Level::DEBUG)
            .finish();
        tracing::subscriber::set_global_default(subscriber).expect("set global subscripber failed");

        let d_org = "tests";
        let d_org = "/tmp/content";
        let compiler = Compiler::new();
        let _sections = compiler
            .compile_section(d_org)
            .expect("no Document compiled");
        for (i, doc) in _sections.documents.into_iter().enumerate() {
            println!("{i}:\n  {:#?}\n  {:#?}", doc.file_info, doc.metadata);
        }
        for (i, sub) in _sections.subsections.into_iter().enumerate() {
            println!("{i}:\n  {:#?}\n", sub.metadata);
        }
    }
}
