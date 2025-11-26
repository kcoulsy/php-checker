use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use std::collections::HashSet;

pub struct DuplicateDeclarationRule;

impl DuplicateDeclarationRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for DuplicateDeclarationRule {
    fn name(&self) -> &str {
        "sanity/duplicate_declaration"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen = HashSet::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_definition" {
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

            if seen.contains(&name) {
                diagnostics.push(diagnostic_for_node(
                    parsed,
                    name_node,
                    Severity::Error,
                    format!("duplicate declaration of \"{name}\""),
                ));
            } else {
                seen.insert(name);
            }
        });

        diagnostics
    }
}
