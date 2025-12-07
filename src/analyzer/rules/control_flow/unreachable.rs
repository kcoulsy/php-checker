use super::DiagnosticRule;
use super::helpers::diagnostic_for_node;
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use tree_sitter::Node;

pub struct UnreachableCodeRule;

impl UnreachableCodeRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for UnreachableCodeRule {
    fn name(&self) -> &str {
        "control_flow/unreachable"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut visitor = UnreachableVisitor::new(parsed);
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct UnreachableVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    diagnostics: Vec<crate::analyzer::Diagnostic>,
}

impl<'a> UnreachableVisitor<'a> {
    fn new(parsed: &'a parser::ParsedSource) -> Self {
        Self {
            parsed,
            diagnostics: Vec::new(),
        }
    }

    fn visit(&mut self, node: Node) {
        if node.kind() == "compound_statement" {
            self.inspect_compound(node);
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

    fn inspect_compound(&mut self, compound: Node) {
        let mut reachable = true;
        let mut cursor = compound.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.is_named() {
                    if !reachable {
                        let start = child.start_position();
                        let row = start.row + 1;
                        let column = start.column + 1;
                        self.diagnostics.push(diagnostic_for_node(
                            self.parsed,
                            child,
                            Severity::Warning,
                            format!("unreachable code after return at {row}:{column}"),
                        ));
                    }

                    if child.kind() == "return_statement" || child.kind() == "throw_statement" {
                        reachable = false;
                    }
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
    fn test_unreachable() {
        let source = r#"<?php

function alwaysReturnEarly(): void
{
    return;
    echo "this line is unreachable";
}

alwaysReturnEarly();

"#;

        let parsed = parse_php(source);
        let rule = UnreachableCodeRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &["warning: unreachable code after return at 6:5"]);
    }

    #[test]
    fn test_unreachable_valid() {
        let source = r#"<?php
function normalFunction(): void
{
    echo "this line is reachable";
    return;
}
"#;

        let parsed = parse_php(source);
        let rule = UnreachableCodeRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }
}
