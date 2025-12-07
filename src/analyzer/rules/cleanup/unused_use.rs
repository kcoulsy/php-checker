use super::DiagnosticRule;
use super::helpers::{diagnostic_for_span, node_text, walk_node};
use crate::analyzer::fix;
use crate::analyzer::project::{ProjectContext, UseInfo};
use crate::analyzer::{Severity, parser};
use std::collections::HashMap;
use tree_sitter::Node;

pub struct UnusedUseRule;

impl UnusedUseRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for UnusedUseRule {
    fn name(&self) -> &str {
        "cleanup/unused_use"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        unused_aliases(parsed, context)
            .into_iter()
            .map(|(alias, info)| {
                diagnostic_for_span(
                    parsed,
                    info.span,
                    Severity::Warning,
                    format!("unused import alias `{alias}`"),
                )
            })
            .collect()
    }

    fn fix(&self, parsed: &parser::ParsedSource, context: &ProjectContext) -> Vec<fix::TextEdit> {
        let source = parsed.source.as_str();

        unused_aliases(parsed, context)
            .into_iter()
            .filter(|(_, info)| !info.declaration_has_multiple_clauses)
            .map(|(_, info)| {
                let (start, end) =
                    fix::covering_line_range(source, info.clause_start, info.clause_end);
                fix::TextEdit::new(start, end, "")
            })
            .collect()
    }
}

fn is_use_clause(mut node: Node) -> bool {
    while let Some(parent) = node.parent() {
        match parent.kind() {
            "namespace_use_declaration" | "namespace_use_clause" | "namespace_aliasing_clause" => {
                return true;
            }
            _ => node = parent,
        }
    }

    false
}

fn unused_aliases<'a>(
    parsed: &'a parser::ParsedSource,
    context: &'a ProjectContext,
) -> Vec<(String, UseInfo)> {
    let scope = match context.scope_for(&parsed.path) {
        Some(scope) if !scope.uses.is_empty() => scope,
        _ => return Vec::new(),
    };

    let mut unused: HashMap<String, UseInfo> = scope.uses.clone();

    walk_node(parsed.tree.root_node(), &mut |node| {
        if is_use_clause(node) {
            return;
        }

        if matches!(node.kind(), "qualified_name" | "namespace_name" | "name") {
            if let Some(text) = node_text(node, parsed) {
                if let Some(first) = text.split('\\').next() {
                    unused.remove(first);
                }
            }
        }
    });

    unused
        .into_iter()
        .filter(|(alias, _)| !alias.starts_with('_'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_fix_with_context, assert_no_diagnostics, parse_php, run_rule, run_rule_with_context};

    #[test]
    fn test_unused_use() {
        let source = r#"<?php

use Multi\Service as Svc;
use Multi\Client;

Svc\takesTwo(1);

"#;

        let rule = UnusedUseRule::new();
        let diagnostics = run_rule_with_context(&rule, source);

        assert_diagnostics_exact(&diagnostics, &["warning: unused import alias `Client`"]);
    }

    #[test]
    fn test_unused_use_fix() {
        let input = r#"<?php

use Multi\Service as Svc;
use Multi\Client;

Svc\takesTwo(1);

"#;

        let expected = r#"<?php

use Multi\Service as Svc;

Svc\takesTwo(1);

"#;

        let rule = UnusedUseRule::new();
        assert_fix_with_context(&rule, input, expected);
    }

    #[test]
    fn test_unused_use_valid() {
        let source = r#"<?php

use Multi\Service as Svc;

Svc\takesTwo(1);
"#;

        let parsed = parse_php(source);
        let rule = UnusedUseRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }
}
