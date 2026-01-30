use crate::compiler::ast_builder::element::Table;
use crate::export::ssg::renderer::ObjectRenderer;

#[derive(serde::Serialize)]
pub struct TableViewModel {
    pub number: Option<usize>,

    pub has_caption: bool,
    pub caption: String,

    pub has_header: bool,
    pub header_rows: Vec<String>,

    pub body_rows: Vec<String>,
}

impl TableViewModel {
    pub fn from_ast(table: &Table,
                    // ctx: &mut RenderContext
    ) -> Self {
        let has_caption = !table.caption.is_empty();
        let caption = table
            .caption
            .iter()
            .map(|e| ObjectRenderer::render_object(e))
            .collect::<String>();

        let has_header = !table.header.is_empty();
        let header_rows = table
            .header
            .iter()
            .map(|r| ObjectRenderer::render_table_row(r))
            .collect();
        let body_rows = table
            .rows
            .iter()
            .map(|e| ObjectRenderer::render_object(e))
            .collect();

        Self {
            number: Some(0),    // fixme

            has_caption,
            caption,

            has_header,
            header_rows,

            body_rows,
        }
    }
}
