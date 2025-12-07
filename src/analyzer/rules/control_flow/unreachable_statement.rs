use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Diagnostic, Severity, parser};
use tree_sitter::Node;

use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node};

pub struct UnreachableStatementRule;

impl UnreachableStatementRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for UnreachableStatementRule {
    fn name(&self) -> &str {
        "control_flow/unreachable_statement"
    }

    fn run(&self, parsed: &parser::ParsedSource, _context: &ProjectContext) -> Vec<Diagnostic> {
        let mut visitor = UnreachableStatementVisitor::new(parsed);
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct UnreachableStatementVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> UnreachableStatementVisitor<'a> {
    fn new(parsed: &'a parser::ParsedSource) -> Self {
        Self {
            parsed,
            diagnostics: Vec::new(),
        }
    }

    fn visit(&mut self, node: Node<'a>) {
        if node.kind() == "switch_statement" {
            self.inspect_switch(node);
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.visit(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn inspect_switch(&mut self, switch_node: Node<'a>) {
        let block = match child_by_kind(switch_node, "switch_block") {
            Some(block) => block,
            None => return,
        };

        let mut cursor = block.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "case_statement" {
                    self.check_unreachable_statements(child);
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn check_unreachable_statements(&mut self, case_node: Node) {
        let mut cursor = case_node.walk();
        let mut encountered_control_flow = false;

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                match child.kind() {
                    "case" | ":" => {} // Skip case label
                    "break_statement" | "return_statement" | "continue_statement"
                    | "throw_statement" | "goto_statement" => {
                        if encountered_control_flow {
                            let stmt_type = match child.kind() {
                                "break_statement" => "break",
                                "return_statement" => "return",
                                "continue_statement" => "continue",
                                "throw_statement" => "throw",
                                "goto_statement" => "goto",
                                _ => "statement",
                            };
                            self.diagnostics.push(diagnostic_for_node(
                                self.parsed,
                                child,
                                Severity::Warning,
                                format!("unreachable {} statement", stmt_type),
                            ));
                        } else {
                            encountered_control_flow = true;
                        }
                    }
                    "comment" => {} // Skip comments
                    _ => {}         // Other statements are allowed
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_unreachable_statement() {
        let source = r#"<?php

function test_impossible_break(): void {
    $value = 1;
    switch ($value) {
        case 1:
            echo "one";
            break;
            break;
        case 2:
            echo "two";
            return;
            break;
    }
}

function test_impossible_return(): void {
    $value = 1;
    switch ($value) {
        case 1:
            echo "one";
            return;
            return;
        case 2:
            echo "two";
            return;
    }
}

"#;

        let parsed = parse_php(source);
        let rule = UnreachableStatementRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &[
            "warning: unreachable break statement",
            "warning: unreachable break statement",
            "warning: unreachable return statement",
        ]);
    }

    #[test]
    fn test_unreachable_statement_valid() {
        let source = r#"<?php
function test_valid(): void {
    $value = 1;
    switch ($value) {
        case 1:
            echo "one";
            break;
        case 2:
            echo "two";
            return;
    }
}
"#;

        let parsed = parse_php(source);
        let rule = UnreachableStatementRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }
}
