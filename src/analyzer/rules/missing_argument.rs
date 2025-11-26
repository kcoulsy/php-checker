use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

pub struct MissingArgumentRule;

impl MissingArgumentRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for MissingArgumentRule {
    fn name(&self) -> &str {
        "missing-argument"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_call_expression" {
                return;
            }

            let name_node =
                child_by_kind(node, "name").or_else(|| child_by_kind(node, "qualified_name"));
            let name_node = match name_node {
                Some(node) => node,
                None => return,
            };

            let name = match node_text(name_node, parsed) {
                Some(name) => name,
                None => return,
            };

            let symbol = match context.resolve_function_symbol(&name, parsed) {
                Some(symbol) => symbol,
                None => return,
            };

            let arguments = match child_by_kind(node, "arguments") {
                Some(arguments) => arguments,
                None => return,
            };

            let count = (0..arguments.named_child_count())
                .filter(|idx| {
                    arguments
                        .named_child(*idx)
                        .map(|child| child.kind() == "argument")
                        .unwrap_or(false)
                })
                .count();

            if count < symbol.required_params {
                diagnostics.push(diagnostic_for_node(
                    parsed,
                    name_node,
                    Severity::Error,
                    format!("missing required argument {} for {name}", count + 1),
                ));
            }
        });

        diagnostics
    }
}
