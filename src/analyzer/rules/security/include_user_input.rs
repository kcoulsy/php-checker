use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use tree_sitter::Node;

const SUPERGLOBALS: &[&str] = &[
    "$_GET",
    "$_POST",
    "$_REQUEST",
    "$_SERVER",
    "$_COOKIE",
    "$_FILES",
    "$_ENV",
];

pub struct IncludeUserInputRule;

impl IncludeUserInputRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for IncludeUserInputRule {
    fn name(&self) -> &str {
        "security/include_user_input"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| match node.kind() {
            "include_expression"
            | "require_expression"
            | "include_once_expression"
            | "require_once_expression" => {
                if contains_superglobal(node, parsed) {
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        node,
                        Severity::Warning,
                        "including user input is dangerous",
                    ));
                }
            }
            _ => {}
        });

        diagnostics
    }
}

fn contains_superglobal<'a>(node: Node<'a>, parsed: &parser::ParsedSource) -> bool {
    let mut found = false;
    walk_node(node, &mut |child| {
        if child.kind() == "variable_name" {
            if let Some(text) = node_text(child, parsed) {
                if SUPERGLOBALS.contains(&text.as_str()) {
                    found = true;
                }
            }
        }
    });
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, parse_php, run_rule};

    #[test]
    fn test_include_user_input_file() {
        let source = r#"<?php

$page = $_GET['page'];
include $page;
<?php

include $_GET['file'];

"#;

        let parsed = parse_php(source);
        let rule = IncludeUserInputRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &["warning: including user input is dangerous"]);
    }
}
