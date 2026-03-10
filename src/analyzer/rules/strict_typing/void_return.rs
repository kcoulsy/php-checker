use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use tree_sitter::Node;

pub struct VoidReturnRule;

impl VoidReturnRule {
    pub fn new() -> Self {
        Self
    }

    fn is_void_return_type(func_node: Node, parsed: &parser::ParsedSource) -> bool {
        if let Some(return_type) = child_by_kind(func_node, "union_type") {
            if let Some(text) = node_text(return_type, parsed) {
                return text.trim() == "void";
            }
        }
        false
    }
}

impl DiagnosticRule for VoidReturnRule {
    fn name(&self) -> &str {
        "strict_typing/void_return"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "return_statement" {
                return;
            }
            // Only flag returns that have a value (not bare `return;`)
            if node.named_child_count() == 0 {
                return;
            }

            // Walk up to find the nearest enclosing function/method
            let mut current = node;
            while let Some(parent) = current.parent() {
                if matches!(
                    parent.kind(),
                    "function_definition" | "method_declaration" | "arrow_function"
                ) {
                    if Self::is_void_return_type(parent, parsed) {
                        let start = node.start_position();
                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            node,
                            Severity::Error,
                            format!(
                                "function declared void must not return a value at {}:{}",
                                start.row + 1,
                                start.column + 1
                            ),
                        ));
                    }
                    break;
                }
                current = parent;
            }
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
    fn test_void_function_returns_value() {
        let source = r#"<?php
function test(): void {
    return 42;
}
"#;
        let parsed = parse_php(source);
        let rule = VoidReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_diagnostics_exact(
            &diagnostics,
            &["error: function declared void must not return a value at 3:4"],
        );
    }

    #[test]
    fn test_void_method_returns_value() {
        let source = r#"<?php
class Foo {
    public function bar(): void {
        return "hello";
    }
}
"#;
        let parsed = parse_php(source);
        let rule = VoidReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_diagnostics_exact(
            &diagnostics,
            &["error: function declared void must not return a value at 4:8"],
        );
    }

    #[test]
    fn test_void_function_bare_return_ok() {
        let source = r#"<?php
function test(): void {
    if (true) {
        return;
    }
    doSomething();
}
"#;
        let parsed = parse_php(source);
        let rule = VoidReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_non_void_function_return_ok() {
        let source = r#"<?php
function test(): int {
    return 42;
}
"#;
        let parsed = parse_php(source);
        let rule = VoidReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_nested_void_function() {
        let source = r#"<?php
function outer(): int {
    function inner(): void {
        return 1;
    }
    return 42;
}
"#;
        let parsed = parse_php(source);
        let rule = VoidReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        // Only the inner function's return should be flagged
        assert_diagnostics_exact(
            &diagnostics,
            &["error: function declared void must not return a value at 4:8"],
        );
    }

    #[test]
    fn test_no_return_type_ok() {
        let source = r#"<?php
function test() {
    return 42;
}
"#;
        let parsed = parse_php(source);
        let rule = VoidReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }
}
