use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, literal_type, node_text, variable_name_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use std::collections::HashMap;

pub struct ImpossibleComparisonRule;

impl ImpossibleComparisonRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for ImpossibleComparisonRule {
    fn name(&self) -> &str {
        "control_flow/impossible_comparison"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut var_types = HashMap::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() == "assignment_expression" {
                if let (Some(var_node), Some(value_node)) = (node.child(0), node.child(2)) {
                    if let Some(name) = variable_name_text(var_node, parsed) {
                        if let Some(ty) = literal_type(value_node) {
                            var_types.insert(name, ty);
                        }
                    }
                }
            }

            if node.kind() != "binary_expression" {
                return;
            }

            let operator = node.child(1);
            if operator.map_or(true, |op| op.kind() != "===") {
                return;
            }

            let left = node.child(0);
            let right = node.child(2);
            if left.is_none() || right.is_none() {
                return;
            }
            let left = left.unwrap();
            let right = right.unwrap();

            let var_name = match variable_name_text(left, parsed) {
                Some(name) => name,
                None => return,
            };

            let left_type = match var_types.get(&var_name) {
                Some(ty) => ty.clone(),
                None => return,
            };

            let right_type = match literal_type(right) {
                Some(ty) => ty,
                None => return,
            };

            if left_type != right_type {
                let expression = node_text(node, parsed).unwrap_or_else(|| "expression".into());
                diagnostics.push(diagnostic_for_node(
                    parsed,
                    node,
                    Severity::Error,
                    format!("comparison \"{expression}\" is always false due to type difference"),
                ));
            }
        });

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_impossible_comparison() {
        let source = r#"<?php

$flag = true;

if ($flag === 5) {
    echo "never happens";
}

"#;

        let parsed = parse_php(source);
        let rule = ImpossibleComparisonRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &["error: comparison \"$flag === 5\" is always false due to type difference"]);
    }

    #[test]
    fn test_impossible_comparison_valid() {
        let source = r#"<?php
$flag = true;

if ($flag === true) {
    echo "happens";
}
"#;

        let parsed = parse_php(source);
        let rule = ImpossibleComparisonRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }
}
