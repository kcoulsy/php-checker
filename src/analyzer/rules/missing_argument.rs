use super::DiagnosticRule;
use super::helpers::{
    child_by_kind, collect_function_signatures, diagnostic_for_node, node_text, walk_node,
};
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

    fn run(&self, parsed: &parser::ParsedSource) -> Vec<crate::analyzer::Diagnostic> {
        let signatures = collect_function_signatures(parsed);
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_call_expression" {
                return;
            }

            let name_node = match child_by_kind(node, "name") {
                Some(name_node) => name_node,
                None => return,
            };

            let name = match node_text(name_node, parsed) {
                Some(name) => name,
                None => return,
            };

            let signature = match signatures.get(&name) {
                Some(signature) => signature,
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

            if count < signature.params.len() {
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
