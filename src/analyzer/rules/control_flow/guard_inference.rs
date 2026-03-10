use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, variable_name_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use std::collections::HashSet;
use tree_sitter::Node;

/// Guard functions that assert a variable is null/falsy.
/// When used as `if (is_null($x)) { ... }`, method calls on `$x` inside the body are bugs.
static NULL_GUARD_FUNCTIONS: &[&str] = &["is_null"];

pub struct GuardInferenceRule;

impl GuardInferenceRule {
    pub fn new() -> Self {
        Self
    }

    /// Extract the variable name from a unary guard call like `is_null($x)`.
    /// Returns `Some(var_name)` if the function name is a null guard and has one var argument.
    fn extract_null_guarded_var<'a>(
        call: Node<'a>,
        parsed: &parser::ParsedSource,
    ) -> Option<String> {
        let name_node = child_by_kind(call, "name")?;
        let name = node_text(name_node, parsed)?;

        if !NULL_GUARD_FUNCTIONS.contains(&name.as_str()) {
            return None;
        }

        let args = child_by_kind(call, "arguments")?;
        // Find the first argument
        for i in 0..args.named_child_count() {
            let arg = args.named_child(i)?;
            if arg.kind() == "argument" {
                // Look for variable_name inside the argument
                for j in 0..arg.named_child_count() {
                    if let Some(child) = arg.named_child(j) {
                        if child.kind() == "variable_name" {
                            return variable_name_text(child, parsed);
                        }
                    }
                }
            }
        }

        None
    }

    /// Check if a compound_statement body contains method calls on a variable known to be null.
    fn find_null_usages_in_body(
        body: Node,
        null_vars: &HashSet<String>,
        diagnostics: &mut Vec<crate::analyzer::Diagnostic>,
        parsed: &parser::ParsedSource,
    ) {
        walk_node(body, &mut |node| {
            if node.kind() == "member_call_expression"
                || node.kind() == "member_access_expression"
                || node.kind() == "null_safe_member_call_expression"
            {
                // The object (left side) should not be a known-null variable
                if let Some(object) = node.named_child(0) {
                    if object.kind() == "variable_name" {
                        if let Some(var_name) = variable_name_text(object, parsed) {
                            if null_vars.contains(&var_name) {
                                let start = node.start_position();
                                diagnostics.push(diagnostic_for_node(
                                    parsed,
                                    node,
                                    Severity::Warning,
                                    format!(
                                        "variable ${var_name} is null in this branch; method/property access will fail at {}:{}",
                                        start.row + 1,
                                        start.column + 1
                                    ),
                                ));
                            }
                        }
                    }
                }
            }
        });
    }
}

impl DiagnosticRule for GuardInferenceRule {
    fn name(&self) -> &str {
        "control_flow/guard_inference"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "if_statement" {
                return;
            }

            // Get the condition — it's in a parenthesized_expression
            let condition = match child_by_kind(node, "parenthesized_expression") {
                Some(c) => c,
                None => return,
            };

            // Look for a direct function_call_expression inside the condition
            let call = match child_by_kind(condition, "function_call_expression") {
                Some(c) => c,
                None => return,
            };

            let guarded_var = match Self::extract_null_guarded_var(call, parsed) {
                Some(v) => v,
                None => return,
            };

            // Now look at the TRUE branch (compound_statement)
            let body = match child_by_kind(node, "compound_statement") {
                Some(b) => b,
                None => return,
            };

            let mut null_vars = HashSet::new();
            null_vars.insert(guarded_var);

            Self::find_null_usages_in_body(body, &null_vars, &mut diagnostics, parsed);
        });

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{
        assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule,
    };

    #[test]
    fn test_method_call_on_null_flagged() {
        let source = r#"<?php
if (is_null($x)) {
    $x->method();
}
"#;
        let parsed = parse_php(source);
        let rule = GuardInferenceRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_diagnostics_exact(
            &diagnostics,
            &["warning: variable $x is null in this branch; method/property access will fail at 3:4"],
        );
    }

    #[test]
    fn test_property_access_on_null_flagged() {
        let source = r#"<?php
if (is_null($user)) {
    echo $user->name;
}
"#;
        let parsed = parse_php(source);
        let rule = GuardInferenceRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_diagnostics_exact(
            &diagnostics,
            &["warning: variable $user is null in this branch; method/property access will fail at 3:9"],
        );
    }

    #[test]
    fn test_null_check_no_method_call_ok() {
        let source = r#"<?php
if (is_null($x)) {
    echo "x is null";
    return;
}
$x->method();
"#;
        let parsed = parse_php(source);
        let rule = GuardInferenceRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_is_string_guard_not_flagged() {
        let source = r#"<?php
if (is_string($x)) {
    $x->method();
}
"#;
        let parsed = parse_php(source);
        let rule = GuardInferenceRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        // is_string is not a null guard — not flagged by this rule
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_no_if_statement_ok() {
        let source = r#"<?php
$x = null;
$x->method();
"#;
        let parsed = parse_php(source);
        let rule = GuardInferenceRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        // We only flag inside is_null guards, not general null usage
        assert_no_diagnostics(&diagnostics);
    }
}
