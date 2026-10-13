//! Strict parser for the supported Mermaid entity relationship syntax.

use pest::Parser;
use pest_derive::Parser;

use crate::graph::{Arrow, Direction, Stroke};

#[derive(Parser)]
#[grammar = "mermaid/er/grammar.pest"]
struct ErParser;

/// Entity declarations in source order, including implicit endpoints.
#[derive(Debug, Default)]
pub struct ErModel {
    pub direction: Direction,
    pub entities: Vec<ErEntity>,
    pub relations: Vec<ErRelation>,
}

/// An entity with ordered attribute display rows.
#[derive(Debug)]
pub struct ErEntity {
    pub name: String,
    pub attributes: Vec<String>,
}

/// A relationship retaining endpoint cardinality and identifying status.
#[derive(Debug)]
pub struct ErRelation {
    pub from: String,
    pub to: String,
    pub start: Arrow,
    pub end: Arrow,
    pub stroke: Stroke,
    pub label: String,
}

fn unquote(text: &str) -> String {
    text.trim().trim_matches('"').to_string()
}

fn ensure_entity(model: &mut ErModel, name: String) -> usize {
    if let Some(index) = model.entities.iter().position(|e| e.name == name) {
        index
    } else {
        let index = model.entities.len();
        model.entities.push(ErEntity {
            name,
            attributes: Vec::new(),
        });
        index
    }
}

fn cardinality(text: &str) -> Arrow {
    match text {
        "||" => Arrow::ErOne,
        "|o" | "o|" => Arrow::ErZeroOrOne,
        "}|" | "|{" => Arrow::ErOneOrMore,
        "}o" | "o{" => Arrow::ErZeroOrMore,
        _ => unreachable!("grammar validates cardinality"),
    }
}

/// Parse an entire ER diagram. Unsupported or malformed statements are errors.
pub fn parse_er_diagram(input: &str) -> Result<ErModel, Box<dyn std::error::Error + Send + Sync>> {
    let input = crate::mermaid::strip_frontmatter(input);
    let mut model = ErModel::default();
    let diagram = ErParser::parse(Rule::diagram, input)?.next().unwrap();
    for statement in diagram.into_inner() {
        match statement.as_rule() {
            Rule::direction => {
                model.direction = match statement.as_str().split_whitespace().last().unwrap() {
                    "BT" => Direction::BottomTop,
                    "LR" => Direction::LeftRight,
                    "RL" => Direction::RightLeft,
                    _ => Direction::TopDown,
                };
            }
            Rule::entity => {
                let mut parts = statement.into_inner();
                let index = ensure_entity(&mut model, unquote(parts.next().unwrap().as_str()));
                for attribute in parts {
                    let row = attribute
                        .into_inner()
                        .map(|part| {
                            if part.as_rule() == Rule::keys {
                                part.into_inner()
                                    .map(|key| key.as_str())
                                    .collect::<Vec<_>>()
                                    .join(",")
                            } else {
                                part.as_str().to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    model.entities[index].attributes.push(row);
                }
            }
            Rule::relation => {
                let mut parts = statement.into_inner();
                let from = unquote(parts.next().unwrap().as_str());
                let start = cardinality(parts.next().unwrap().as_str());
                let stroke = if parts.next().unwrap().as_str() == "--" {
                    Stroke::Solid
                } else {
                    Stroke::Dashed
                };
                let end = cardinality(parts.next().unwrap().as_str());
                let to = unquote(parts.next().unwrap().as_str());
                let label = unquote(parts.next().unwrap().as_str());
                ensure_entity(&mut model, from.clone());
                ensure_entity(&mut model, to.clone());
                model.relations.push(ErRelation {
                    from,
                    to,
                    start,
                    end,
                    stroke,
                    label,
                });
            }
            _ => {}
        }
    }
    Ok(model)
}

#[cfg(test)]
mod tests {
    #[test]
    fn er_parser_merges_declarations_and_handles_frontmatter_comments() {
        let source = "---\ntitle: Orders\n---\n%% comment\nerDiagram\ndirection LR\n\"Sales Customer\" { int id PK, FK \"key\" }\n\"Sales Customer\" { string name }\n\"Sales Customer\" ||--o{ ORDER : \"places orders\"\n";
        let parsed = super::parse_er_diagram(source).unwrap();
        assert_eq!(parsed.entities.len(), 2);
        assert_eq!(
            parsed.entities[0].attributes,
            ["int id PK,FK \"key\"", "string name"]
        );
        assert_eq!(parsed.relations[0].label, "places orders");
        assert_eq!(parsed.direction, crate::graph::Direction::LeftRight);
    }
}
