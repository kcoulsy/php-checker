use super::DiagnosticRule;
use super::helpers::{TypeHint, child_by_kind, diagnostic_for_node, literal_type, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use tree_sitter::Node;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReturnType {
    Void,
    Typed(TypeHint),
}

pub struct ConsistentReturnRule;

impl ConsistentReturnRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for ConsistentReturnRule {
    fn name(&self) -> &str {
        "strict_typing/consistent_return"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_definition" {
                return;
            }

            let body = match child_by_kind(node, "compound_statement") {
                Some(body) => body,
                None => return,
            };

            let mut return_types = Vec::new();

            walk_node(body, &mut |candidate| {
                if candidate.kind() == "return_statement" {
                    let return_type = analyze_return_type(candidate, parsed);
                    return_types.push((return_type, candidate));
                }
            });

            if return_types.len() <= 1 {
                return; // Need at least 2 returns to check consistency
            }

            // Check if all return types are the same
            let first_type = &return_types[0].0;
            for (return_type, return_node) in return_types.iter().skip(1) {
                if !types_compatible(first_type, return_type) {
                    let start = return_node.start_position();
                    let row = start.row + 1;
                    let column = start.column + 1;

                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        *return_node,
                        Severity::Error,
                        format!(
                            "inconsistent return type: expected {}, found {} at {row}:{column}",
                            type_description(&first_type),
                            type_description(return_type)
                        ),
                    ));
                }
            }
        });

        diagnostics
    }
}

fn analyze_return_type(return_node: Node, parsed: &parser::ParsedSource) -> ReturnType {
    // Check if there's an expression after 'return'
    for idx in 0..return_node.named_child_count() {
        if let Some(child) = return_node.named_child(idx) {
            // Try to determine the type using literal_type first
            if let Some(returned_type) = literal_type(child) {
                return ReturnType::Typed(returned_type);
            }
            // Try to determine the type of the expression directly
            if let Some(returned_type) = infer_expression_type(child, parsed) {
                return ReturnType::Typed(returned_type);
            }
        }
    }

    // No expression means void return
    ReturnType::Void
}

fn infer_expression_type(node: Node, _parsed: &parser::ParsedSource) -> Option<TypeHint> {
    match node.kind() {
        "string" | "encapsed_string" => Some(TypeHint::String),
        "integer" => Some(TypeHint::Int),
        "boolean" => Some(TypeHint::Bool),
        "variable_name" => {
            // For variables, we can't easily determine type statically
            // This could be extended with more sophisticated analysis
            None
        }
        "function_call_expression" => {
            // For function calls, we'd need to know the function's return type
            // This is complex and would require inter-procedural analysis
            None
        }
        "binary_expression" | "unary_expression" => {
            // For expressions, we'd need to evaluate the types
            // This could be extended with expression type inference
            None
        }
        _ => {
            // Try using the literal_type helper for other cases
            literal_type(node)
        }
    }
}

fn types_compatible(type1: &ReturnType, type2: &ReturnType) -> bool {
    match (type1, type2) {
        (ReturnType::Void, ReturnType::Void) => true,
        (ReturnType::Typed(t1), ReturnType::Typed(t2)) => t1 == t2,
        _ => false,
    }
}

fn type_description(return_type: &ReturnType) -> String {
    match return_type {
        ReturnType::Void => "void".to_string(),
        ReturnType::Typed(hint) => type_hint_to_string(hint),
    }
}

fn type_hint_to_string(hint: &TypeHint) -> String {
    match hint {
        TypeHint::Int => "int".to_string(),
        TypeHint::String => "string".to_string(),
        TypeHint::Bool => "bool".to_string(),
        TypeHint::Float => "float".to_string(),
        TypeHint::Object(class_name) => class_name.clone(),
        TypeHint::Nullable(inner) => format!("?{}", type_hint_to_string(inner)),
        TypeHint::Union(types) => types
            .iter()
            .map(type_hint_to_string)
            .collect::<Vec<_>>()
            .join("|"),
        TypeHint::Array(inner) => format!("{}[]", type_hint_to_string(inner)),
        TypeHint::GenericArray { key, value } => {
            format!(
                "array<{}, {}>",
                type_hint_to_string(key),
                type_hint_to_string(value)
            )
        }
        TypeHint::ShapedArray(fields) => {
            let fields_str = fields
                .iter()
                .map(|(name, hint)| {
                    format!("{}: {}", name, type_hint_to_string(hint))
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("array{{{}}}", fields_str)
        }
        TypeHint::Unknown => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_inconsistent_return_types() {
        let source = r#"<?php
function test() {
    return 1;
    return "string";
}
"#;

        let parsed = parse_php(source);
        let rule = ConsistentReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert!(!diagnostics.is_empty());
        assert!(diagnostics[0].message.contains("inconsistent return type"));
    }

    #[test]
    fn test_consistent_return_file() {
        // Test from tests/invalid/strict_typing/consistent_return.php
        let source = r#"<?php

// php-checker-ignore: strict_typing/force_return_type,strict_typing/missing_return,control_flow/unreachable

// Function with inconsistent return types - should trigger error
function inconsistentReturns(bool $flag) {
    if ($flag) {
        return 42;  // returns int
    } else {
        return "hello";  // returns string - inconsistent!
    }
}

// Function with consistent return types - should be OK
function consistentReturns(bool $flag) {
    if ($flag) {
        return 42;
    } else {
        return 24;
    }
}

// Function with mixed types including void - should trigger error
function mixedVoidReturns(bool $flag) {
    if ($flag) {
        return 42;  // returns int
    }
    // implicit void return - inconsistent!
}

// Function with only void returns - should be OK
function voidReturns(bool $flag) {
    if ($flag) {
        return;  // explicit void
    }
    // implicit void return
}

// Function with only one return - should be OK
function singleReturn() {
    return "single";
}

// Function with boolean returns - should be OK
function booleanReturns(bool $flag) {
    if ($flag) {
        return true;
    } else {
        return false;
    }
}

inconsistentReturns(true);
consistentReturns(true);
mixedVoidReturns(true);
voidReturns(true);
singleReturn();
booleanReturns(true);
"#;

        let parsed = parse_php(source);
        let rule = ConsistentReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Expected: error: inconsistent return type: expected int, found string at 10:9
        assert_diagnostics_exact(&diagnostics, &["error: inconsistent return type: expected int, found string at 10:9"]);
    }

    #[test]
    fn test_consistent_return_types_valid() {
        // Test valid cases - consistent return types should not trigger errors
        let source = r#"<?php
// Function with consistent return types - should be OK
function consistentReturns(bool $flag) {
    if ($flag) {
        return 42;
    } else {
        return 24;
    }
}

// Function with only void returns - should be OK
function voidReturns(bool $flag) {
    if ($flag) {
        return;  // explicit void
    }
    // implicit void return
}

// Function with only one return - should be OK
function singleReturn() {
    return "single";
}

// Function with boolean returns - should be OK
function booleanReturns(bool $flag) {
    if ($flag) {
        return true;
    } else {
        return false;
    }
}
"#;

        let parsed = parse_php(source);
        let rule = ConsistentReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }
}
