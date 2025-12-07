use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

const SENSITIVE_SUBSTRINGS: &[&str] = &["password", "passwd", "token", "api_key", "secret"];

pub struct HardCodedCredentialsRule;

impl HardCodedCredentialsRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for HardCodedCredentialsRule {
    fn name(&self) -> &str {
        "security/hard_coded_credentials"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "string" {
                return;
            }

            if let Some(text) = node_text(node, parsed) {
                let lowered = text.to_lowercase();
                if SENSITIVE_SUBSTRINGS
                    .iter()
                    .any(|substr| lowered.contains(substr))
                {
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        node,
                        Severity::Warning,
                        "hard-coded credential or token detected",
                    ));
                }
            }
        });

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, parse_php, run_rule};

    #[test]
    fn test_hard_credentials_file() {
        let source = r#"<?php

db_connect('super-secret-password');
call_api('token', 'my-api-key');

"#;

        let parsed = parse_php(source);
        let rule = HardCodedCredentialsRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Expected: 2 warnings for hard-coded credentials
        assert_diagnostics_exact(&diagnostics, &[
            "warning: hard-coded credential or token detected",
            "warning: hard-coded credential or token detected",
        ]);
    }
}
