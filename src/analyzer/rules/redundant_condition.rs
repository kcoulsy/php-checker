use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::{Severity, parser};
use std::collections::{HashMap, HashSet};

pub struct RedundantConditionRule;

impl RedundantConditionRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for RedundantConditionRule {
    fn name(&self) -> &str {
        "redundant-condition"
    }

    fn run(&self, parsed: &parser::ParsedSource) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen_by_parent: HashMap<usize, HashSet<String>> = HashMap::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "if_statement" {
                return;
            }

            let parenthesized = match child_by_kind(node, "parenthesized_expression") {
                Some(expr) => expr,
                None => return,
            };

            let condition = parenthesized.child(1);
            if condition.is_none() {
                return;
            }
            let condition = condition.unwrap();
            let text = match node_text(condition, parsed) {
                Some(text) => text,
                None => return,
            };

            let parent_id = node.parent().map(|parent| parent.id()).unwrap_or(0);
            let seen = seen_by_parent.entry(parent_id).or_default();

            if seen.contains(&text) {
                diagnostics.push(diagnostic_for_node(
                    parsed,
                    condition,
                    Severity::Error,
                    format!("redundant condition \"{text}\" repeats an earlier guard"),
                ));
            } else {
                seen.insert(text);
            }
        });

        diagnostics
    }
}
