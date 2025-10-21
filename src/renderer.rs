//!
use crate::parser::syntax::SyntaxNode;
use rowan::GreenNode;
pub mod html;

// pub struct RenderConfig {}

pub struct Render {
    // from_format: String,
    // to_format: String
}

impl Render {
    pub fn new() -> Self {
        // if from_format != "org" {
        //     panic!("only org supported")
        // }

        // if to_format != "html" {
        //     panic!("only html supported")
        // }

        Self {}
    }

    pub fn render(&self, root: &GreenNode) -> String {
        let syntax_node = SyntaxNode::new_root(root.clone());
        Self::render_node(&syntax_node)
    }

    fn render_node(_node: &SyntaxNode) -> String {
        let mut result = String::new();
        result.push_str("todo");
        result
    }
}
