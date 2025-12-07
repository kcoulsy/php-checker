use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use std::collections::HashSet;

pub struct DuplicateDeclarationRule;

impl DuplicateDeclarationRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for DuplicateDeclarationRule {
    fn name(&self) -> &str {
        "sanity/duplicate_declaration"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut seen = HashSet::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_definition" {
                return;
            }

            let name_node = match child_by_kind(node, "name") {
                Some(name_node) => name_node,
                None => return,
            };

            let name = match node_text(name_node, parsed) {
                Some(name) => name,
                None => return,
            };

            if seen.contains(&name) {
                diagnostics.push(diagnostic_for_node(
                    parsed,
                    name_node,
                    Severity::Error,
                    format!("duplicate declaration of \"{name}\""),
                ));
            } else {
                seen.insert(name);
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
    fn test_duplicate_declaration() {
        let source = r#"<?php

function helper(): void
{
}

function helper(): void
{
}

"#;

        let parsed = parse_php(source);
        let rule = DuplicateDeclarationRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &["error: duplicate declaration of \"helper\""]);
    }

    #[test]
    fn test_duplicate_declaration_valid() {
        let source = r#"<?php
function helper1(): void
{
}

function helper2(): void
{
}
"#;

        let parsed = parse_php(source);
        let rule = DuplicateDeclarationRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }
}
