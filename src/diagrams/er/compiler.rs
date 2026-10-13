//! Compile Mermaid ER syntax into graph-family IR.

use crate::graph::{Edge, Graph, Node, Shape};
use crate::mermaid::er::ErModel;

pub fn compile(model: &ErModel) -> Graph {
    let mut graph = Graph::new(model.direction);
    for entity in &model.entities {
        let mut rows = vec![entity.name.clone()];
        if !entity.attributes.is_empty() {
            rows.push(Node::SEPARATOR.to_string());
            rows.extend(entity.attributes.iter().cloned());
        }
        graph.add_node(
            Node::new(&entity.name)
                .with_label(rows.join("\n"))
                .with_shape(Shape::Rectangle),
        );
    }
    for relation in &model.relations {
        let mut edge = Edge::new(&relation.from, &relation.to)
            .with_arrows(relation.start, relation.end)
            .with_stroke(relation.stroke)
            .with_label(&relation.label);
        edge.tail_label = relation.start.er_cardinality().map(str::to_string);
        edge.head_label = relation.end.er_cardinality().map(str::to_string);
        graph.add_edge(edge);
    }
    graph
}
