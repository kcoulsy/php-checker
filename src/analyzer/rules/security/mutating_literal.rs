use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
const MUTATING_FUNCTIONS: &[&str] = &[
    "array_pop",
    "array_shift",
    "array_push",
    "sort",
    "rsort",
    "asort",
    "ksort",
];

pub struct MutatingLiteralRule;

impl MutatingLiteralRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for MutatingLiteralRule {
    fn name(&self) -> &str {
        "security/mutating_literal"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_call_expression" {
                return;
            }

            let name_node = match child_by_kind(node, "name") {
                Some(node) => node,
                None => return,
            };

            if let Some(name) = node_text(name_node, parsed) {
                if !MUTATING_FUNCTIONS.contains(&name.as_str()) {
                    return;
                }
            } else {
                return;
            }

            let arguments = match child_by_kind(node, "arguments") {
                Some(arguments) => arguments,
                None => return,
            };

            for idx in 0..arguments.named_child_count() {
                if let Some(argument) = arguments.named_child(idx) {
                    if let Some(array_literal) =
                        child_by_kind(argument, "array_creation_expression")
                    {
                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            array_literal,
                            Severity::Warning,
                            format!(
                                "{} modifies its argument in place; avoid passing literals",
                                node_text(name_node, parsed).unwrap_or_default()
                            ),
                        ));
                    }
                }
            }
        });

        diagnostics
    }
}
