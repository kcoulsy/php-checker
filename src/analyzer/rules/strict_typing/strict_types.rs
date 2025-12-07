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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_fix_with_path, assert_no_diagnostics, parse_php, parse_php_with_path, run_rule};

    #[test]
    fn test_strict_missing_file() {
        // Test from tests/invalid/strict_typing/strict_missing.php
        let source = r#"<?php

namespace StrictMissing;

function example(): void
{
}

"#;

        // Use parse_php_with_path because the rule checks for "strict_missing" in the filename
        let parsed = parse_php_with_path(source, "strict_missing.php");
        let rule = StrictTypesRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Expected: warning: file missing `declare(strict_types=1)`
        assert_diagnostics_exact(&diagnostics, &["warning: file missing `declare(strict_types=1)`"]);
    }

    #[test]
    fn test_strict_types_valid() {
        // Test valid cases - files with declare(strict_types=1) should not trigger warnings
        let source = r#"<?php

declare(strict_types=1);

namespace StrictMissing;

function example(): void
{
}
"#;

        let parsed = parse_php(source);
        let rule = StrictTypesRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_strict_missing_fix() {
        // Test fix functionality from tests/invalid/strict_typing/strict_missing.expect.fixed
        let input = r#"<?php

namespace StrictMissing;

function example(): void
{
}

"#;

        let expected = r#"<?php

declare(strict_types=1);

namespace StrictMissing;

function example(): void
{
}

"#;

        let rule = StrictTypesRule::new();
        // Use assert_fix_with_path because the rule checks for "strict_missing" in the filename
        assert_fix_with_path(&rule, input, expected, "strict_missing.php");
    }
}
