use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use tree_sitter::Node;

pub struct InvalidThisRule;

impl InvalidThisRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for InvalidThisRule {
    fn name(&self) -> &str {
        "invalid-this"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "variable_name" {
                return;
            }

            if let Some(name) = node_text(node, parsed) {
                if name != "this" {
                    return;
                }
            } else {
                return;
            }

            let mut parent = node;
            let mut found_method = None;
            let mut found_class = false;

            while let Some(p) = parent.parent() {
                match p.kind() {
                    "method_declaration" => {
                        found_method = Some(p);
                        parent = p;
                    }
                    "class_declaration" => {
                        found_class = true;
                        break;
                    }
                    _ => parent = p,
                }
            }

            if !found_class {
                diagnostics.push(diagnostic_for_node(
                    parsed,
                    node,
                    Severity::Error,
                    "$this is not allowed outside of class scope",
                ));
                return;
            }

            if let Some(method) = found_method {
                if let Some(text) = node_text(method, parsed) {
                    if text.contains("static") {
                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            node,
                            Severity::Error,
                            "$this cannot be used in static context",
                        ));
                    }
                }
            }
        });

        diagnostics
    }
}
