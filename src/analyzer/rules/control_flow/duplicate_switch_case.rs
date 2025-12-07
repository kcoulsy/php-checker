use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Diagnostic, Severity, parser};
use std::collections::HashSet;
use tree_sitter::Node;

use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text};

pub struct DuplicateSwitchCaseRule;

impl DuplicateSwitchCaseRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for DuplicateSwitchCaseRule {
    fn name(&self) -> &str {
        "control_flow/duplicate_switch_case"
    }

    fn run(&self, parsed: &parser::ParsedSource, _context: &ProjectContext) -> Vec<Diagnostic> {
        let mut visitor = DuplicateSwitchVisitor::new(parsed);
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct DuplicateSwitchVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> DuplicateSwitchVisitor<'a> {
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

        let mut seen = HashSet::new();
        let mut cursor = block.walk();

        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "case_statement" {
                    for idx in 0..child.named_child_count() {
                        let label = match child.named_child(idx) {
                            Some(label) => label,
                            None => continue,
                        };

                        if let Some((key, display)) = literal_case_value(label, self.parsed) {
                            if seen.contains(&key) {
                                self.diagnostics.push(diagnostic_for_node(
                                    self.parsed,
                                    label,
                                    Severity::Warning,
                                    format!("duplicate switch case {}", display),
                                ));
                            } else {
                                seen.insert(key);
                            }
                            break;
                        }
                    }
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
}

fn literal_case_value(node: Node, parsed: &parser::ParsedSource) -> Option<(String, String)> {
    match node.kind() {
        "string" | "encapsed_string" => node_text(node, parsed).map(|text| {
            let trimmed = text.trim_matches(|c| c == '\'' || c == '"');
            (format!("str:{}", trimmed), format!("'{}'", trimmed))
        }),
        "integer" => node_text(node, parsed).map(|value| (format!("int:{}", value), value)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_duplicate_switch_case() {
        let source = r#"<?php

$value = 'alpha';

switch ($value) {
    case 'alpha':
        echo 'first alpha';
        break;
    case 'beta':
        echo 'beta';
        break;
    case 'alpha':
        echo 'duplicate alpha';
        break;
    case 1:
        break;
    case 1:
        break;
}

"#;

        let parsed = parse_php(source);
        let rule = DuplicateSwitchCaseRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &[
            "warning: duplicate switch case 'alpha'",
            "warning: duplicate switch case 1",
        ]);
    }

    #[test]
    fn test_duplicate_switch_case_valid() {
        let source = r#"<?php
$value = 'alpha';

switch ($value) {
    case 'alpha':
        echo 'alpha';
        break;
    case 'beta':
        echo 'beta';
        break;
}
"#;

        let parsed = parse_php(source);
        let rule = DuplicateSwitchCaseRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }
}
