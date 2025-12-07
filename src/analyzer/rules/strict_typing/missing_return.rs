use super::DiagnosticRule;
use super::helpers::{
    child_by_kind, diagnostic_for_node, has_conditional_ancestor, node_text, walk_node,
};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use tree_sitter::Node;

pub struct MissingReturnRule;

impl MissingReturnRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for MissingReturnRule {
    fn name(&self) -> &str {
        "strict_typing/missing_return"
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

            let body = match child_by_kind(node, "compound_statement") {
                Some(body) => body,
                None => return,
            };

            let mut return_nodes = Vec::new();
            walk_node(body, &mut |candidate| {
                if candidate.kind() == "return_statement" {
                    return_nodes.push(candidate);
                }
            });

            if return_nodes.is_empty() {
                return;
            }

            // Check if there's an unconditional return (early return pattern)
            let has_unconditional = return_nodes
                .iter()
                .any(|r| !has_conditional_ancestor(*r, body));

            if has_unconditional {
                return;
            }

            // Check if all conditional branches return (e.g., if-else where both return)
            if all_conditional_branches_return(body, &return_nodes) {
                return;
            }

            let name_node = node.child_by_field_name("name").unwrap_or(node);
            let name = node_text(name_node, parsed).unwrap_or_else(|| "anonymous".into());
            let start = name_node.start_position();
            let row = start.row + 1;
            let column = start.column + 1;

            diagnostics.push(diagnostic_for_node(
                parsed,
                name_node,
                Severity::Error,
                format!("function {name} is missing a return on some paths at {row}:{column}"),
            ));
        });

        diagnostics
    }
}

/// Check if all branches of conditional statements (if-else) have return statements.
/// This handles cases like:
/// ```php
/// if ($flag) {
///     return 'a';
/// } else {
///     return 'b';
/// }
/// ```
fn all_conditional_branches_return(body: Node, return_nodes: &[Node]) -> bool {
    // Check all if statements in the body
    let mut if_statements = Vec::new();
    walk_node(body, &mut |node| {
        if node.kind() == "if_statement" {
            if_statements.push(node);
        }
    });

    if if_statements.is_empty() {
        return false; // No conditionals to check
    }

    // Check each if statement
    for if_stmt in if_statements {
        let mut has_if_return = false;
        let mut has_else_return = false;
        let mut has_else = false;

        // Check if branch
        if let Some(if_body) = child_by_kind(if_stmt, "compound_statement") {
            has_if_return = return_nodes.iter().any(|r| {
                r.start_byte() >= if_body.start_byte()
                    && r.end_byte() <= if_body.end_byte()
            });
        }

        // Check else/elseif branches
        for i in 0..if_stmt.named_child_count() {
            if let Some(child) = if_stmt.named_child(i) {
                if child.kind() == "else_clause" {
                    has_else = true;
                    if let Some(else_body) = child_by_kind(child, "compound_statement") {
                        has_else_return = return_nodes.iter().any(|r| {
                            r.start_byte() >= else_body.start_byte()
                                && r.end_byte() <= else_body.end_byte()
                        });
                    }
                } else if child.kind() == "elseif_clause" {
                    // For elseif, we need to check recursively
                    // For simplicity, if there's an elseif, we require it to have a return too
                    if let Some(elseif_body) = child_by_kind(child, "compound_statement") {
                        let elseif_has_return = return_nodes.iter().any(|r| {
                            r.start_byte() >= elseif_body.start_byte()
                                && r.end_byte() <= elseif_body.end_byte()
                        });
                        if !elseif_has_return {
                            return false;
                        }
                    }
                }
            }
        }

        // If there's an else clause, both if and else must return
        // If there's no else clause, the if returning is not enough (need return after if)
        if has_else {
            if !has_if_return || !has_else_return {
                return false;
            }
        } else {
            // No else clause - this if doesn't guarantee all paths return
            return false;
        }
    }

    // All if-else statements have returns in all branches
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_missing_return_file() {
        let source = r#"<?php

function maybeString(bool $flag)
{
    if ($flag) {
        return 'ok';
    }

    // Missing return for the `false` branch.
}

maybeString(false);

"#;

        let parsed = parse_php(source);
        let rule = MissingReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &["error: function maybeString is missing a return on some paths at 3:10"]);
    }

    #[test]
    fn test_missing_return_valid() {
        let source = r#"<?php
// Function with return on all paths - should be OK
function alwaysReturns(bool $flag): string {
    if ($flag) {
        return 'ok';
    } else {
        return 'not ok';
    }
}

// Function with single return - should be OK
function singleReturn(): string {
    return 'single';
}

// Function with void return type - should be OK (no return needed)
function voidFunction(): void {
    // no return needed
}

// Function with early return - should be OK
function earlyReturn(bool $flag): string {
    if ($flag) {
        return 'early';
    }
    return 'late';
}
"#;

        let parsed = parse_php(source);
        let rule = MissingReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }
}
