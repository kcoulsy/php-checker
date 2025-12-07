use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

pub struct ForceReturnTypeRule;

impl ForceReturnTypeRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for ForceReturnTypeRule {
    fn name(&self) -> &str {
        "strict_typing/force_return_type"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_definition" {
                return;
            }

            // Check if function has a return type hint
            let has_return_type = child_by_kind(node, "union_type").is_some();

            if !has_return_type {
                let name_node = node.child_by_field_name("name").unwrap_or(node);
                let name = node_text(name_node, parsed).unwrap_or_else(|| "anonymous".into());
                let start = name_node.start_position();
                let row = start.row + 1;
                let column = start.column + 1;

                diagnostics.push(diagnostic_for_node(
                    parsed,
                    name_node,
                    Severity::Warning,
                    format!(
                        "function {name} should have an explicit return type at {row}:{column}"
                    ),
                ));
            }
        });

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_force_return_type_file() {
        let source = r#"<?php

// Function without return type - should trigger warning
function noReturnType() {
    return 42;
}

// Function with void return type - should be OK
function withVoidReturnType(): void {
    // no return needed
}

// Function with int return type - should be OK
function withIntReturnType(): int {
    return 42;
}

// Function with string return type - should be OK
function withStringReturnType(): string {
    return "hello";
}

noReturnType();
withVoidReturnType();
withIntReturnType();
withStringReturnType();
"#;

        let parsed = parse_php(source);
        let rule = ForceReturnTypeRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &["warning: function noReturnType should have an explicit return type at 4:10"]);
    }

    #[test]
    fn test_force_return_type_valid() {
        let source = r#"<?php
// Function with void return type - should be OK
function withVoidReturnType(): void {
    // no return needed
}

// Function with int return type - should be OK
function withIntReturnType(): int {
    return 42;
}

// Function with string return type - should be OK
function withStringReturnType(): string {
    return "hello";
}

// Function with bool return type - should be OK
function withBoolReturnType(): bool {
    return true;
}
"#;

        let parsed = parse_php(source);
        let rule = ForceReturnTypeRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }
}
