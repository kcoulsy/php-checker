use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

pub struct ArithmeticTypeRule;

impl ArithmeticTypeRule {
    pub fn new() -> Self {
        Self
    }

    fn is_array_literal(node: tree_sitter::Node) -> bool {
        matches!(
            node.kind(),
            "array_creation_expression" | "array"
        )
    }

    fn is_numeric_literal(node: tree_sitter::Node) -> bool {
        matches!(node.kind(), "integer" | "float")
    }

    fn is_string_literal(node: tree_sitter::Node) -> bool {
        matches!(node.kind(), "string" | "encapsed_string")
    }
}

impl DiagnosticRule for ArithmeticTypeRule {
    fn name(&self) -> &str {
        "strict_typing/arithmetic_type"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "binary_expression" {
                return;
            }

            // Get operator
            let operator = node
                .children(&mut node.walk())
                .find(|c| !c.is_named())
                .and_then(|c| node_text(c, parsed));

            let op = match operator.as_deref() {
                Some(op) => op,
                None => return,
            };

            // Only check arithmetic operators (not + for arrays since array + array is valid)
            let is_numeric_op = matches!(op, "-" | "*" | "/" | "%" | "**");
            // + is special: array + array is valid in PHP (union), but array + number is not
            let is_plus = op == "+";

            if !is_numeric_op && !is_plus {
                return;
            }

            // Get left and right operands (first and last named children)
            let left = node.named_child(0);
            let right = node.named_child(1);

            let (left, right) = match (left, right) {
                (Some(l), Some(r)) => (l, r),
                _ => return,
            };

            // Flag: array on one side, number/string on the other with numeric operator
            if is_numeric_op || is_plus {
                if Self::is_array_literal(left)
                    && (Self::is_numeric_literal(right) || Self::is_string_literal(right))
                {
                    let start = node.start_position();
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        node,
                        Severity::Error,
                        format!(
                            "arithmetic operation between array and scalar is invalid at {}:{}",
                            start.row + 1,
                            start.column + 1
                        ),
                    ));
                    return;
                }
                if Self::is_array_literal(right)
                    && (Self::is_numeric_literal(left) || Self::is_string_literal(left))
                {
                    let start = node.start_position();
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        node,
                        Severity::Error,
                        format!(
                            "arithmetic operation between scalar and array is invalid at {}:{}",
                            start.row + 1,
                            start.column + 1
                        ),
                    ));
                    return;
                }
            }

            // Flag: string minus/multiply/divide/modulo — these coerce but are almost always bugs
            if is_numeric_op && op != "+" {
                if Self::is_string_literal(left) && Self::is_numeric_literal(right) {
                    let start = node.start_position();
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        node,
                        Severity::Warning,
                        format!(
                            "arithmetic operation on string literal with '{op}' is likely a bug at {}:{}",
                            start.row + 1,
                            start.column + 1
                        ),
                    ));
                    return;
                }
                if Self::is_numeric_literal(left) && Self::is_string_literal(right) {
                    let start = node.start_position();
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        node,
                        Severity::Warning,
                        format!(
                            "arithmetic operation on string literal with '{op}' is likely a bug at {}:{}",
                            start.row + 1,
                            start.column + 1
                        ),
                    ));
                }
            }
        });

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{
        assert_diagnostics_exact, assert_has_diagnostics, assert_no_diagnostics, parse_php,
        run_rule,
    };

    #[test]
    fn test_array_plus_number_flagged() {
        let source = r#"<?php
$x = [1, 2, 3] + 5;
"#;
        let parsed = parse_php(source);
        let rule = ArithmeticTypeRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_has_diagnostics(&diagnostics, "array + scalar");
    }

    #[test]
    fn test_number_minus_array_flagged() {
        let source = r#"<?php
$x = 5 - [1, 2, 3];
"#;
        let parsed = parse_php(source);
        let rule = ArithmeticTypeRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_has_diagnostics(&diagnostics, "scalar - array");
    }

    #[test]
    fn test_string_minus_number_flagged() {
        let source = r#"<?php
$x = "hello" - 3;
"#;
        let parsed = parse_php(source);
        let rule = ArithmeticTypeRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_has_diagnostics(&diagnostics, "string - number");
    }

    #[test]
    fn test_valid_arithmetic_ok() {
        let source = r#"<?php
$x = 5 + 3;
$y = 10 - 2;
$z = 4 * 3;
$w = 10 / 2;
$m = 7 % 3;
"#;
        let parsed = parse_php(source);
        let rule = ArithmeticTypeRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_array_union_ok() {
        let source = r#"<?php
$x = [1, 2] + [3, 4];
"#;
        let parsed = parse_php(source);
        let rule = ArithmeticTypeRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_string_concat_ok() {
        let source = r#"<?php
$x = "hello" . " world";
"#;
        let parsed = parse_php(source);
        let rule = ArithmeticTypeRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }
}
