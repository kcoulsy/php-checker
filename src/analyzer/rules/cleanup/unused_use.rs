use super::DiagnosticRule;
use super::helpers::{diagnostic_for_span, node_text, walk_node};
use crate::analyzer::project::{ProjectContext, UseInfo};
use crate::analyzer::{Severity, parser};
use std::collections::HashMap;
use tree_sitter::Node;

pub struct UnusedUseRule;

impl UnusedUseRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for UnusedUseRule {
    fn name(&self) -> &str {
        "cleanup/unused_use"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let scope = match context.scope_for(&parsed.path) {
            Some(scope) if !scope.uses.is_empty() => scope,
            _ => return Vec::new(),
        };

        let mut unused: HashMap<String, UseInfo> = scope.uses.clone();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if is_use_clause(node) {
                return;
            }

            if matches!(node.kind(), "qualified_name" | "namespace_name" | "name") {
                if let Some(text) = node_text(node, parsed) {
                    if let Some(first) = text.split('\\').next() {
                        unused.remove(first);
                    }
                }
            }
        });

        unused
            .into_iter()
            .map(|(alias, info)| {
                diagnostic_for_span(
                    parsed,
                    info.span,
                    Severity::Warning,
                    format!("unused import alias `{alias}`"),
                )
            })
            .collect()
    }
}

fn is_use_clause(mut node: Node) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "namespace_use_declaration" | "namespace_use_clause" | "namespace_aliasing_clause" => {
                return true;
            }
            _ => node = parent,
        }
    }

    false
}
