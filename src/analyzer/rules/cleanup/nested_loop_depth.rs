use super::DiagnosticRule;
use super::helpers::diagnostic_for_node;
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use tree_sitter::Node;

const MAX_LOOP_DEPTH: usize = 3;

pub struct NestedLoopDepthRule;

impl NestedLoopDepthRule {
    pub fn new() -> Self {
        Self
    }

    fn is_loop(kind: &str) -> bool {
        matches!(
            kind,
            "for_statement"
                | "foreach_statement"
                | "while_statement"
                | "do_statement"
        )
    }

    fn walk_with_depth(
        node: Node,
        depth: usize,
        diagnostics: &mut Vec<crate::analyzer::Diagnostic>,
        parsed: &parser::ParsedSource,
    ) {
        let current_depth = if Self::is_loop(node.kind()) {
            depth + 1
        } else {
            depth
        };

        if Self::is_loop(node.kind()) && current_depth > MAX_LOOP_DEPTH {
            let start = node.start_position();
            diagnostics.push(diagnostic_for_node(
                parsed,
                node,
                Severity::Warning,
                format!(
                    "loop is nested too deeply ({current_depth} levels); consider refactoring at {}:{}",
                    start.row + 1,
                    start.column + 1
                ),
            ));
            // Don't recurse further — we already flagged this level
            return;
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                Self::walk_with_depth(cursor.node(), current_depth, diagnostics, parsed);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
}

impl DiagnosticRule for NestedLoopDepthRule {
    fn name(&self) -> &str {
        "cleanup/nested_loop_depth"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();
        Self::walk_with_depth(parsed.tree.root_node(), 0, &mut diagnostics, parsed);
        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{
        assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule,
    };

    #[test]
    fn test_four_level_nesting_flagged() {
        let source = r#"<?php
foreach ($a as $x) {
    foreach ($x as $y) {
        foreach ($y as $z) {
            foreach ($z as $w) {
                echo $w;
            }
        }
    }
}
"#;
        let parsed = parse_php(source);
        let rule = NestedLoopDepthRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_diagnostics_exact(
            &diagnostics,
            &["warning: loop is nested too deeply (4 levels); consider refactoring at 5:12"],
        );
    }

    #[test]
    fn test_three_level_nesting_ok() {
        let source = r#"<?php
foreach ($a as $x) {
    foreach ($x as $y) {
        foreach ($y as $z) {
            echo $z;
        }
    }
}
"#;
        let parsed = parse_php(source);
        let rule = NestedLoopDepthRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_mixed_loop_types_flagged() {
        let source = r#"<?php
for ($i = 0; $i < 10; $i++) {
    while (true) {
        foreach ($a as $x) {
            do {
                echo $x;
            } while (false);
        }
    }
}
"#;
        let parsed = parse_php(source);
        let rule = NestedLoopDepthRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_diagnostics_exact(
            &diagnostics,
            &["warning: loop is nested too deeply (4 levels); consider refactoring at 5:12"],
        );
    }

    #[test]
    fn test_sequential_loops_not_flagged() {
        let source = r#"<?php
foreach ($a as $x) {
    echo $x;
}
foreach ($b as $y) {
    echo $y;
}
"#;
        let parsed = parse_php(source);
        let rule = NestedLoopDepthRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }
}
