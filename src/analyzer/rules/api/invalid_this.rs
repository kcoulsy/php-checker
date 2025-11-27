use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

pub struct InvalidThisRule;

impl InvalidThisRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for InvalidThisRule {
    fn name(&self) -> &str {
        "api/invalid_this"
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

            let name = match node_text(node, parsed) {
                Some(name) => name.trim_start_matches('$').to_string(),
                None => return,
            };

            if name != "this" {
                return;
            }

            let mut parent = node;
            let mut found_class = false;
            let mut in_static_method = false;

            while let Some(p) = parent.parent() {
                match p.kind() {
                    "method_declaration" => {
                        if child_by_kind(p, "static_modifier").is_some() {
                            in_static_method = true;
                        }
                        parent = p;
                    }
                    "class_declaration" | "enum_declaration" => {
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

            if in_static_method {
                diagnostics.push(diagnostic_for_node(
                    parsed,
                    node,
                    Severity::Error,
                    "$this cannot be used in static context",
                ));
            }
        });

        diagnostics
    }
}
