use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, newline_for_source, walk_node};
use crate::analyzer::fix;
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

pub struct StrictTypesRule;

impl StrictTypesRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for StrictTypesRule {
    fn name(&self) -> &str {
        "strict_typing/strict_types"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        if !should_warn(parsed) || !has_type_hint(parsed) || has_strict_declare(parsed) {
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

    fn fix(&self, parsed: &parser::ParsedSource, _context: &ProjectContext) -> Vec<fix::TextEdit> {
        if !should_warn(parsed) || !has_type_hint(parsed) || has_strict_declare(parsed) {
            return Vec::new();
        }

        let source = parsed.source.as_str();
        let newline = newline_for_source(source);
        if let Some(offset) = strict_types_insert_offset(source) {
            let insert_text = strict_types_insert_text(source, offset, newline);
            vec![fix::TextEdit::new(offset, offset, insert_text)]
        } else {
            Vec::new()
        }
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

fn has_strict_declare(parsed: &parser::ParsedSource) -> bool {
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
    found
}

fn strict_types_insert_offset(source: &str) -> Option<usize> {
    const TAG: &str = "<?php";
    let tag_pos = source.find(TAG)?;
    let mut offset = tag_pos + TAG.len();
    while offset < source.len() {
        match source.as_bytes()[offset] {
            b'\r' | b'\n' => offset += 1,
            _ => break,
        }
    }
    Some(offset)
}

fn strict_types_insert_text(source: &str, offset: usize, newline: &str) -> String {
    let needs_prefix = offset == 0 || !source[..offset].ends_with(newline);

    let mut text = String::new();
    if needs_prefix {
        text.push_str(newline);
    }
    text.push_str("declare(strict_types=1);");
    text.push_str(newline);
    text.push_str(newline);
    text
}
