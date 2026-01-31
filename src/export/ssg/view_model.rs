use crate::compiler::ast_builder::element::Table;
use crate::export::ssg::renderer::{Renderer, RendererContext};

#[derive(serde::Serialize)]
pub struct TableViewModel {
    pub table_number: Option<usize>,
    pub has_caption: bool,
    pub caption: String,
    pub has_header: bool,
    pub header_rows: Vec<String>,
    pub body_rows: Vec<String>,
}

impl TableViewModel {
    pub fn from_ast(table: &Table, context: &mut RendererContext) -> Self {
        let has_caption = !table.caption.is_empty();
        let table_number = if has_caption {
            context.table_counter += 1;
            Some(context.table_counter)
        } else {
            None
        };
        let caption = table
            .caption
            .iter()
            .map(|e| Renderer::render_object(e))
            .collect::<String>();

        let has_header = !table.header.is_empty();
        let header_rows = table
            .header
            .iter()
            .map(|r| Renderer::render_table_row(r))
            .collect();
        let body_rows = table
            .rows
            .iter()
            .map(|e| Renderer::render_table_row(e))
            .collect();

        Self {
            table_number,

            has_caption,
            caption,

            has_header,
            header_rows,

            body_rows,
        }
    }

    pub fn from_ast_v2(table: &Table, context: &mut RendererContext) -> Self {
        let has_caption = !table.caption.is_empty();
        let table_number = if has_caption {
            context.table_counter += 1;
            Some(context.table_counter)
        } else {
            None
        };
        let caption = table
            .caption
            .iter()
            .map(|e| Renderer::render_object(e))
            .collect::<String>();

        let has_header = !table.header.is_empty();
        let header_rows = table
            .header
            .iter()
            .map(|r| Renderer::render_table_row(r))
            .collect();
        let body_rows = table
            .rows
            .iter()
            .map(|e| Renderer::render_table_row(e))
            .collect();

        Self {
            table_number,

            has_caption,
            caption,

            has_header,
            header_rows,

            body_rows,
        }
    }
}
