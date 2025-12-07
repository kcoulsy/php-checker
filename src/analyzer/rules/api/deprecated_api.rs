use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

const DEPRECATED_APIS: &[&str] = &[
    "mysql_query",
    "mysql_connect",
    "mysql_pconnect",
    "each",
    "create_function",
];

pub struct DeprecatedApiRule;

impl DeprecatedApiRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for DeprecatedApiRule {
    fn name(&self) -> &str {
        "api/deprecated_api"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_call_expression" {
                return;
            }

            if let Some(name_node) = child_by_kind(node, "name") {
                if let Some(name) = node_text(name_node, parsed) {
                    if DEPRECATED_APIS.contains(&name.as_str()) {
                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            name_node,
                            Severity::Warning,
                            format!("{} is deprecated; use modern alternatives", name),
                        ));
                    }
                }
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
    fn test_deprecated_api() {
        let source = r#"<?php

mysql_connect('localhost', 'user', 'pass');
create_function('$a', 'return $a;');

"#;

        let parsed = parse_php(source);
        let rule = DeprecatedApiRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &[
            "warning: mysql_connect is deprecated; use modern alternatives",
            "warning: create_function is deprecated; use modern alternatives",
        ]);
    }

    #[test]
    fn test_deprecated_api_valid() {
        let source = r#"<?php
mysqli_connect('localhost', 'user', 'pass');
$func = function($a) { return $a; };
"#;

        let parsed = parse_php(source);
        let rule = DeprecatedApiRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }
}
