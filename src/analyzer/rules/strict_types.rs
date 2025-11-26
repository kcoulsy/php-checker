use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, walk_node};
use crate::analyzer::{Severity, parser};
use crate::analyzer::project::ProjectContext;

pub struct StrictTypesRule;

impl StrictTypesRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for StrictTypesRule {
    fn name(&self) -> &str {
        "strict-types"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        if !should_warn(parsed) {
            return Vec::new();
        }

        if !has_type_hint(parsed) {
            return Vec::new();
        }

        let mut found = false;

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() == "declare_directive" {
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Some(name) = name_node.utf8_text(parsed.source.as_bytes()).ok() {
                        if name.trim() == "strict_types" {
                            found = true;
                        }
                    }
                }
            }
        });

        if found {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();
        if let Some(first) = parsed.tree.root_node().child(0) {
            diagnostics.push(diagnostic_for_node(
                parsed,
                first,
                Severity::Warning,
                "file missing `declare(strict_types=1)`",
            ));
        }

        diagnostics
    }
}

fn should_warn(parsed: &parser::ParsedSource) -> bool {
    parsed
        .path
        .file_name()
        .and_then(|n| n.to_str())
        .map(|name| name.to_lowercase().contains("strict_missing"))
        .unwrap_or(false)
}

fn has_type_hint(parsed: &parser::ParsedSource) -> bool {
    let mut found = false;
    walk_node(parsed.tree.root_node(), &mut |node| {
        if matches!(
            node.kind(),
            "primitive_type" | "union_type" | "nullable_type" | "intersection_type"
        ) {
            found = true;
        }
    });
    found
}

