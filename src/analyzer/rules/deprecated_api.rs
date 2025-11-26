use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

const DEPRECATED_APIS: &[&str] = &[
    "mysql_query",
    "mysql_connect",
    "mysql_pconnect",
    "each",
    "create_function",
];

pub struct DeprecatedApiRule;

impl DeprecatedApiRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for DeprecatedApiRule {
    fn name(&self) -> &str {
        "deprecated-api"
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

            if let Some(name_node) = node.child_by_field_name("name") {
                if let Some(name) = node_text(name_node, parsed) {
                    if DEPRECATED_APIS.contains(&name.as_str()) {
                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            name_node,
                            Severity::Warning,
                            format!("{} is deprecated; use modern alternatives", name),
                        ));
                    }
                }
            }
        });

        diagnostics
    }
}
