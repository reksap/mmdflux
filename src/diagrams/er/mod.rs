//! Entity relationship diagrams, compiled into graph-family payloads.

pub mod compiler;

use crate::errors::RenderError;
use crate::graph::Graph;
use crate::registry::{DiagramInstance, ParsedDiagram};

#[derive(Default)]
pub struct ErInstance;

pub fn detect(input: &str) -> bool {
    crate::mermaid::detect_diagram_type(input) == Some(crate::mermaid::DiagramType::Er)
}

impl DiagramInstance for ErInstance {
    fn parse(
        self: Box<Self>,
        input: &str,
    ) -> Result<Box<dyn ParsedDiagram>, Box<dyn std::error::Error + Send + Sync>> {
        let model = crate::mermaid::er::parse_er_diagram(input)?;
        Ok(Box::new(ParsedEr(compiler::compile(&model))))
    }
}

struct ParsedEr(Graph);

impl ParsedDiagram for ParsedEr {
    fn into_payload(self: Box<Self>) -> Result<crate::payload::Diagram, RenderError> {
        Ok(crate::payload::Diagram::Er(self.0))
    }
}
