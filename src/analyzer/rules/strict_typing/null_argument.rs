use super::DiagnosticRule;
use super::helpers::{TypeHint, child_by_kind, collect_function_signatures, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

pub struct NullArgumentRule;

impl NullArgumentRule {
    pub fn new() -> Self {
        Self
    }

    fn is_nullable(hint: &TypeHint) -> bool {
        matches!(hint, TypeHint::Nullable(_) | TypeHint::Unknown)
    }
}

impl DiagnosticRule for NullArgumentRule {
    fn name(&self) -> &str {
        "strict_typing/null_argument"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let signatures = collect_function_signatures(parsed);
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_call_expression" {
                return;
            }

            let name_node = match child_by_kind(node, "name") {
                Some(n) => n,
                None => return,
            };

            let name = match node_text(name_node, parsed) {
                Some(n) => n,
                None => return,
            };

            let signature = match signatures.get(&name) {
                Some(s) => s,
                None => return,
            };

            let arguments = match child_by_kind(node, "arguments") {
                Some(a) => a,
                None => return,
            };

            let mut arg_index = 0;
            for idx in 0..arguments.named_child_count() {
                let Some(argument_node) = arguments.named_child(idx) else {
                    continue;
                };

                if argument_node.kind() != "argument" {
                    continue;
                }

                if arg_index >= signature.params.len() {
                    break;
                }

                // Check if the argument is a null literal
                let is_null = (0..argument_node.named_child_count()).any(|i| {
                    argument_node
                        .named_child(i)
                        .map_or(false, |child| child.kind() == "null")
                }) || argument_node.child(0).map_or(false, |c| c.kind() == "null");

                if is_null {
                    let param_type = &signature.params[arg_index];
                    if !Self::is_nullable(param_type) {
                        let start = argument_node.start_position();
                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            argument_node,
                            Severity::Error,
                            format!(
                                "argument {} of {name} is non-nullable but received null at {}:{}",
                                arg_index + 1,
                                start.row + 1,
                                start.column + 1
                            ),
                        ));
                    }
                }

                arg_index += 1;
            }
        });

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
    fn test_null_passed_to_non_nullable_int() {
        let source = r#"<?php
function takesInt(int $value): void {}
takesInt(null);
"#;
        let parsed = parse_php(source);
        let rule = NullArgumentRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_diagnostics_exact(
            &diagnostics,
            &["error: argument 1 of takesInt is non-nullable but received null at 3:9"],
        );
    }

    #[test]
    fn test_null_passed_to_non_nullable_string() {
        let source = r#"<?php
function takesString(string $value): void {}
takesString(null);
"#;
        let parsed = parse_php(source);
        let rule = NullArgumentRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_diagnostics_exact(
            &diagnostics,
            &["error: argument 1 of takesString is non-nullable but received null at 3:12"],
        );
    }

    #[test]
    fn test_null_passed_to_nullable_ok() {
        let source = r#"<?php
function takesNullable(?string $value): void {}
takesNullable(null);
"#;
        let parsed = parse_php(source);
        let rule = NullArgumentRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_null_second_arg_non_nullable() {
        let source = r#"<?php
function test(?string $a, int $b): void {}
test(null, null);
"#;
        let parsed = parse_php(source);
        let rule = NullArgumentRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_diagnostics_exact(
            &diagnostics,
            &["error: argument 2 of test is non-nullable but received null at 3:11"],
        );
    }

    #[test]
    fn test_non_null_args_ok() {
        let source = r#"<?php
function test(int $a, string $b): void {}
test(1, "hello");
"#;
        let parsed = parse_php(source);
        let rule = NullArgumentRule::new();
        let diagnostics = run_rule(&rule, &parsed);
        assert_no_diagnostics(&diagnostics);
    }
}
