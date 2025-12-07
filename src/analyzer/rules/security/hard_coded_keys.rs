use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

const KEY_INDICATORS: &[&str] = &[
    "key",
    "secret",
    "token",
    "api_key",
    "apikey",
    "encryption",
    "cipher",
];

pub struct HardCodedKeysRule;

impl HardCodedKeysRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for HardCodedKeysRule {
    fn name(&self) -> &str {
        "security/hard_coded_keys"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "string" && node.kind() != "encapsed_string" {
                return;
            }

            if let Some(text) = node_text(node, parsed) {
                // Skip obviously non-keys (too short, contains spaces, etc.)
                if text.len() < 8 || text.contains(' ') || text.contains('\n') {
                    return;
                }

                // Look for patterns that suggest encryption keys
                if is_potential_key(&text) {
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        node,
                        Severity::Error,
                        "potential hard-coded encryption key detected, consider using environment variables or secure key management",
                    ));
                }
            }
        });

        diagnostics
    }
}

fn is_potential_key(text: &str) -> bool {
    // Check for common key patterns:
    // - Hexadecimal strings (common for keys)
    // - Base64-like strings
    // - Long alphanumeric strings
    // - Strings containing key-related keywords

    let text_lower = text.to_lowercase();

    // Check for key indicator words
    if KEY_INDICATORS
        .iter()
        .any(|indicator| text_lower.contains(indicator))
    {
        return true;
    }

    // Check for hexadecimal patterns (common in keys)
    if text.len() >= 16 && text.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }

    // Check for base64-like patterns
    if text.len() >= 16
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    {
        // Additional check for base64 padding
        if text.ends_with('=') || (text.len() % 4 == 0) {
            return true;
        }
    }

    // Check for long random-looking strings
    if text.len() >= 20 && text.chars().all(|c| c.is_ascii_alphanumeric()) {
        // Count different character types to detect randomness
        let has_lower = text.chars().any(|c| c.is_ascii_lowercase());
        let has_upper = text.chars().any(|c| c.is_ascii_uppercase());
        let has_digit = text.chars().any(|c| c.is_ascii_digit());

        // If it has mixed case and digits, likely a key
        if has_lower && has_upper && has_digit {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_hard_coded_keys_file() {
        let source = r#"<?php

// Hard-coded encryption key - should trigger error
$key = "hardcodedkey123456789012345";
"#;

        let parsed = parse_php(source);
        let rule = HardCodedKeysRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &["error: potential hard-coded encryption key detected, consider using environment variables or secure key management"]);
    }
}
