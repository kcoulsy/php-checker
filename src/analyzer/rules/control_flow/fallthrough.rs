use crate::analyzer::ignore::IgnoreState;
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Diagnostic, Severity, parser};
use tree_sitter::Node;

use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node};

pub struct FallthroughRule;

impl FallthroughRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for FallthroughRule {
    fn name(&self) -> &str {
        "control_flow/fallthrough"
    }

    fn run(&self, parsed: &parser::ParsedSource, _context: &ProjectContext) -> Vec<Diagnostic> {
        let mut visitor = FallthroughVisitor::new(parsed);
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct FallthroughVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> FallthroughVisitor<'a> {
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
        let mut case_nodes = Vec::new();

        // Collect all case statements
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "case_statement" {
                    case_nodes.push(child);
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }

        // Check for fall-through without comment
        for (i, case_node) in case_nodes.iter().enumerate() {
            // Not the last case and doesn't end with control flow and no ignore comment
            if i < case_nodes.len() - 1
                && !case_ends_with_control_flow(*case_node, self.parsed)
                && !case_has_ignore_comment(*case_node, self.parsed)
            {
                self.diagnostics.push(diagnostic_for_node(
                    self.parsed,
                    *case_node,
                    Severity::Warning,
                    "case falls through to next case without explicit comment".to_string(),
                ));
            }
        }
    }
}

fn case_ends_with_control_flow(case_node: Node, _parsed: &parser::ParsedSource) -> bool {
    // Find the last statement in the case
    let mut cursor = case_node.walk();
    let mut last_statement = None;

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            match child.kind() {
                "case" | ":" => {} // Skip case label
                _ => {
                    last_statement = Some(child);
                }
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    match last_statement {
        Some(stmt) => matches!(
            stmt.kind(),
            "break_statement"
                | "return_statement"
                | "continue_statement"
                | "throw_statement"
                | "goto_statement"
        ),
        None => false,
    }
}

fn case_has_ignore_comment(_case_node: Node, parsed: &parser::ParsedSource) -> bool {
    // Check if there's a php-checker-ignore comment for the fallthrough rule
    let ignore_state = IgnoreState::from_source(parsed.source.as_str());
    ignore_state.should_ignore("control_flow/fallthrough")
}
