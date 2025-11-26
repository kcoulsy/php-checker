use super::DiagnosticRule;
use super::helpers::{
    child_by_kind, diagnostic_for_node, has_conditional_ancestor, node_text, walk_node,
};
use crate::analyzer::{Severity, parser};

pub struct MissingReturnRule;

impl MissingReturnRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for MissingReturnRule {
    fn name(&self) -> &str {
        "missing-return"
    }

    fn run(&self, parsed: &parser::ParsedSource) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_definition" {
                return;
            }

            let body = match child_by_kind(node, "compound_statement") {
                Some(body) => body,
                None => return,
            };

            let mut return_nodes = Vec::new();
            walk_node(body, &mut |candidate| {
                if candidate.kind() == "return_statement" {
                    return_nodes.push(candidate);
                }
            });

            if return_nodes.is_empty() {
                return;
            }

            let has_unconditional = return_nodes
                .iter()
                .any(|r| !has_conditional_ancestor(*r, body));

            if has_unconditional {
                return;
            }

            let name_node = node.child_by_field_name("name").unwrap_or(node);
            let name = node_text(name_node, parsed).unwrap_or_else(|| "anonymous".into());
            let start = name_node.start_position();
            let row = start.row + 1;
            let column = start.column + 1;

            diagnostics.push(diagnostic_for_node(
                parsed,
                name_node,
                Severity::Error,
                format!(
                    "function {name} is missing a return on some paths at {row}:{column}"
                ),
            ));
        });

        diagnostics
    }
}
