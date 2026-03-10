use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use tree_sitter::Node;

const MAX_CHAIN_LENGTH: usize = 4;

pub struct MethodChainLengthRule;

impl MethodChainLengthRule {
    pub fn new() -> Self {
        Self
    }

    /// Count the total method call chain length starting from this node downward.
    fn chain_length(node: Node) -> usize {
        let mut depth = 1;
        let mut current = node;
        loop {
            // The object (first named child) of a member_call_expression is what the
            // method is called on. If it's another member_call_expression, the chain continues.
            if let Some(object) = current.named_child(0) {
                if matches!(
                    object.kind(),
                    "member_call_expression" | "null_safe_member_call_expression"
                ) {
                    depth += 1;
                    current = object;
                    continue;
                }
            }
            break;
        }
        depth
    }
}

impl DiagnosticRule for MethodChainLengthRule {
    fn name(&self) -> &str {
        "cleanup/method_chain_length"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if !matches!(
                node.kind(),
                "member_call_expression" | "null_safe_member_call_expression"
            ) {
                return;
            }

            // Only process the outermost call in the chain (parent is not a chain call)
            let parent_is_chain = node.parent().map_or(false, |p| {
                matches!(
                    p.kind(),
                    "member_call_expression" | "null_safe_member_call_expression"
                )
            });

            if parent_is_chain {
                return;
            }

            let length = Self::chain_length(node);
            if length > MAX_CHAIN_LENGTH {
                let start = node.start_position();
                diagnostics.push(diagnostic_for_node(
                    parsed,
                    node,
                    Severity::Warning,
                    format!(
                        "method chain is too long ({length} calls); consider breaking it into smaller steps at {}:{}",
                        start.row + 1,
                        start.column + 1
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
    use crate::analyzer::rules::test_utils::{
        assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule,
    };

    #[test]
    fn test_long_chain_flagged() {
        let source = r#"<?php
$result = $a->foo()->bar()->baz()->qux()->quux();
"#;
        let parsed = parse_php(source);
        let rule = MethodChainLengthRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_diagnostics_exact(
            &diagnostics,
            &["warning: method chain is too long (5 calls); consider breaking it into smaller steps at 2:10"],
        );
    }

    #[test]
    fn test_chain_at_limit_ok() {
        let source = r#"<?php
$result = $a->foo()->bar()->baz()->qux();
"#;
        let parsed = parse_php(source);
        let rule = MethodChainLengthRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_short_chain_ok() {
        let source = r#"<?php
$result = $a->foo()->bar();
"#;
        let parsed = parse_php(source);
        let rule = MethodChainLengthRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_single_call_ok() {
        let source = r#"<?php
$result = $a->foo();
"#;
        let parsed = parse_php(source);
        let rule = MethodChainLengthRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }
}
