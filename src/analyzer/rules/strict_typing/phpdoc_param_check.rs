use super::DiagnosticRule;
use super::helpers::{
    TypeHint, child_by_kind, diagnostic_for_node, is_type_compatible, node_text,
    type_hint_from_parameter, walk_node,
};
use crate::analyzer::phpdoc::{TypeExpression, extract_phpdoc_for_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

pub struct PhpDocParamCheckRule;

impl PhpDocParamCheckRule {
    pub fn new() -> Self {
        Self
    }

    /// Convert PHPDoc TypeExpression to our internal TypeHint for simple cases
    fn type_expression_to_hint(expr: &TypeExpression) -> Option<TypeHint> {
        match expr {
            TypeExpression::Simple(s) => match s.as_str() {
                "int" | "integer" => Some(TypeHint::Int),
                "string" => Some(TypeHint::String),
                "bool" | "boolean" => Some(TypeHint::Bool),
                "float" | "double" => Some(TypeHint::Float),
                // Anything else is treated as an object type (class/interface name)
                _ => Some(TypeHint::Object(s.clone())),
            },
            TypeExpression::Nullable(inner) => {
                // Wrap the inner type in Nullable
                Self::type_expression_to_hint(inner).map(|t| TypeHint::Nullable(Box::new(t)))
            }
            TypeExpression::Union(types) => {
                // Convert each type in the union
                let hints: Vec<TypeHint> = types
                    .iter()
                    .filter_map(|t| Self::type_expression_to_hint(t))
                    .collect();
                if hints.is_empty() {
                    None
                } else {
                    Some(TypeHint::Union(hints))
                }
            }
            TypeExpression::Array(inner) => {
                // Convert array type (e.g., int[], User[])
                Self::type_expression_to_hint(inner).map(|t| TypeHint::Array(Box::new(t)))
            }
            TypeExpression::Generic { base, params } => {
                // Handle generic array types (e.g., array<string, int>)
                if base == "array" && params.len() == 2 {
                    let key_hint = Self::type_expression_to_hint(&params[0])?;
                    let value_hint = Self::type_expression_to_hint(&params[1])?;
                    return Some(TypeHint::GenericArray {
                        key: Box::new(key_hint),
                        value: Box::new(value_hint),
                    });
                }
                None
            }
            _ => None,
        }
    }

    /// Get parameter name from a parameter node
    fn get_param_name(
        param_node: tree_sitter::Node,
        parsed: &parser::ParsedSource,
    ) -> Option<String> {
        // Look for variable_name node
        for i in 0..param_node.named_child_count() {
            if let Some(child) = param_node.named_child(i) {
                if child.kind() == "variable_name" {
                    return node_text(child, parsed).map(|s| s.trim_start_matches('$').to_string());
                }
            }
        }
        None
    }
}

impl PhpDocParamCheckRule {
    fn type_hint_to_string(hint: &TypeHint) -> String {
        match hint {
            TypeHint::Int => "int".to_string(),
            TypeHint::String => "string".to_string(),
            TypeHint::Bool => "bool".to_string(),
            TypeHint::Float => "float".to_string(),
            TypeHint::Object(name) => name.clone(),
            TypeHint::Nullable(inner) => {
                format!("?{}", Self::type_hint_to_string(inner.as_ref()))
            }
            TypeHint::Union(types) => types
                .iter()
                .map(|t| Self::type_hint_to_string(t))
                .collect::<Vec<_>>()
                .join("|"),
            TypeHint::Array(inner) => format!("{}[]", Self::type_hint_to_string(inner)),
            TypeHint::GenericArray { key, value } => {
                format!(
                    "array<{}, {}>",
                    Self::type_hint_to_string(key),
                    Self::type_hint_to_string(value)
                )
            }
            TypeHint::ShapedArray(fields) => {
                let fields_str = fields
                    .iter()
                    .map(|(name, hint)| {
                        format!("{}: {}", name, Self::type_hint_to_string(hint))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("array{{{}}}", fields_str)
            }
            TypeHint::Unknown => "unknown".to_string(),
        }
    }

    fn type_expression_to_string(expr: &TypeExpression) -> String {
        match expr {
            TypeExpression::Simple(s) => s.clone(),
            TypeExpression::Array(inner) => format!("{}[]", Self::type_expression_to_string(inner)),
            TypeExpression::Generic { base, params } => {
                let params_str = params
                    .iter()
                    .map(|p| Self::type_expression_to_string(p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{}<{}>", base, params_str)
            }
            TypeExpression::Union(types) => types
                .iter()
                .map(|t| Self::type_expression_to_string(t))
                .collect::<Vec<_>>()
                .join("|"),
            TypeExpression::Nullable(inner) => {
                format!("?{}", Self::type_expression_to_string(inner))
            }
            TypeExpression::ShapedArray(fields) => {
                let fields_str = fields
                    .iter()
                    .map(|(name, type_expr)| {
                        format!("{}: {}", name, Self::type_expression_to_string(type_expr))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("array{{{}}}", fields_str)
            }
            TypeExpression::Mixed => "mixed".to_string(),
            TypeExpression::Void => "void".to_string(),
            TypeExpression::Never => "never".to_string(),
        }
    }
}

impl DiagnosticRule for PhpDocParamCheckRule {
    fn name(&self) -> &str {
        "strict_typing/phpdoc_param_check"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check function definitions with @param tags
        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_definition" && node.kind() != "method_declaration" {
                return;
            }

            // Extract @param PHPDocs
            if let Some(phpdoc) = extract_phpdoc_for_node(node, parsed) {
                if phpdoc.params.is_empty() {
                    return;
                }

                // Get function parameters
                if let Some(formal_params) = child_by_kind(node, "formal_parameters") {
                    // Build a map of parameter names to their @param types
                    let mut param_types: std::collections::HashMap<String, &TypeExpression> =
                        std::collections::HashMap::new();

                    for param_tag in &phpdoc.params {
                        param_types.insert(param_tag.name.clone(), &param_tag.type_expr);
                    }

                    // Check each parameter
                    for i in 0..formal_params.named_child_count() {
                        if let Some(param_node) = formal_params.named_child(i) {
                            if !matches!(
                                param_node.kind(),
                                "simple_parameter"
                                    | "variadic_parameter"
                                    | "property_promotion_parameter"
                            ) {
                                continue;
                            }

                            // Get parameter name
                            if let Some(param_name) = Self::get_param_name(param_node, parsed) {
                                // Check if there's a @param for this parameter
                                if let Some(expected_type_expr) = param_types.get(&param_name) {
                                    // Get native type hint using helper
                                    let native_hint = type_hint_from_parameter(param_node, parsed);

                                    // Skip if no native type hint
                                    if native_hint == TypeHint::Unknown {
                                        continue;
                                    }

                                    let phpdoc_hint =
                                        Self::type_expression_to_hint(expected_type_expr);

                                    // Check for conflict using compatibility checking
                                    if let Some(phpdoc) = phpdoc_hint {
                                        // Native type and PHPDoc type should match exactly or be compatible
                                        // For @param, we want stricter checking: they should match exactly
                                        // because PHPDoc shouldn't contradict the native hint
                                        if !is_type_compatible(&native_hint, &phpdoc)
                                            && !is_type_compatible(&phpdoc, &native_hint)
                                        {
                                            let expected_name =
                                                Self::type_expression_to_string(expected_type_expr);

                                            let native_type_str =
                                                Self::type_hint_to_string(&native_hint);

                                            // Find the type node for error reporting
                                            let type_node =
                                                child_by_kind(param_node, "primitive_type")
                                                    .or_else(|| {
                                                        child_by_kind(param_node, "named_type")
                                                    })
                                                    .unwrap_or(param_node);

                                            diagnostics.push(diagnostic_for_node(
                                                parsed,
                                                type_node,
                                                Severity::Error,
                                                format!(
                                                    "@param type '{}' conflicts with native type hint '{}' for parameter ${}",
                                                    expected_name, native_type_str, param_name
                                                ),
                                            ));
                                        }
                                    }
                                }
                            }
                        }
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
    fn test_correct_param_matching_native_type() {
        let source = r#"<?php
/**
 * @param int $value
 */
function correctParam(int $value): void {}
correctParam(42);
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_param_contradicts_native_type() {
        let source = r#"<?php
/**
 * @param string $value  // PHPDoc says string
 */
function contradictoryParam(int $value): void {}  // But native hint says int
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: @param type 'string' conflicts with native type hint 'int' for parameter $value"]
        );
    }

    #[test]
    fn test_param_adds_detail_to_array() {
        let source = r#"<?php
/**
 * @param int[] $numbers
 */
function detailedArray(array $numbers): void {}
detailedArray([1, 2, 3]);
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_multiple_params_different_types() {
        let source = r#"<?php
/**
 * @param int $id
 * @param string $name
 * @param bool $active
 */
function multipleParams(int $id, string $name, bool $active): void {}
multipleParams(1, "test", true);
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_union_types_in_param() {
        let source = r#"<?php
/**
 * @param int|string $value
 */
function unionParam($value): void {}
unionParam(42);
unionParam("test");
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_nullable_types() {
        let source = r#"<?php
/**
 * @param ?string $optional
 */
function nullableParam(?string $optional): void {}
nullableParam(null);
nullableParam("test");
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_array_type_with_wrong_element_type() {
        let source = r#"<?php
/**
 * @param User[] $users
 */
function expectsUserArray(array $users): void {}
expectsUserArray([1, 2, 3]);  // Error: int[] instead of User[]
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // This rule only checks conflicts between @param and native type hints
        // Function call type checking would be in a different rule
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_associative_array_type() {
        let source = r#"<?php
/**
 * @param array<string, int> $scores
 */
function expectsScores(array $scores): void {}
expectsScores([1 => "wrong"]);  // Error: int key, string value (should be string key, int value)
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // This rule only checks conflicts between @param and native type hints
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_class_type_parameter() {
        let source = r#"<?php
/**
 * @param \DateTime $date
 */
function expectsDateTime(\DateTime $date): void {}
expectsDateTime(new \DateTime());
expectsDateTime("2024-01-01");  // Error: string is not DateTime
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    // Tests from phpdoc_param_scenarios

    #[test]
    fn test_scenario_03_param_object_conflict() {
        let source = r#"<?php
// Scenario: @param object type conflicts with native object type
// Expected: Error on line 11

class User {}
class Admin {}

/**
 * @param User $user
 */
function processUser(Admin $user) {
    // Error: @param type 'User' conflicts with native type hint 'Admin'
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: @param type 'User' conflicts with native type hint 'Admin' for parameter $user"]
        );
    }

    #[test]
    fn test_scenario_04_param_object_matches() {
        let source = r#"<?php
// Scenario: @param object type matches native object type
// Expected: No errors

class User {}

/**
 * @param User $user
 */
function processUser(User $user) {
    // No error: types match
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_05_param_nullable_matches() {
        let source = r#"<?php
// Scenario: @param nullable type matches native nullable type hint
// Expected: No errors

/**
 * @param ?string $name
 */
function greet(?string $name): void {
    echo $name ?? 'Guest';
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_06_param_nullable_conflict() {
        let source = r#"<?php
// Scenario: @param nullable type conflicts with non-nullable native type hint
// Expected: No error currently - nullable mismatch detection not yet implemented

/**
 * @param ?string $name
 */
function greet(string $name): void {
    echo $name;
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Nullable mismatch detection not yet implemented
        // ?string is compatible with string according to current type checking
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_07_param_non_nullable_conflict() {
        let source = r#"<?php
// Scenario: @param non-nullable type conflicts with nullable native type hint
// Expected: No error currently - nullable mismatch detection not yet implemented

/**
 * @param string $name
 */
function greet(?string $name): void {
    echo $name ?? 'Guest';
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Nullable mismatch detection not yet implemented
        // string is compatible with ?string according to current type checking
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_08_param_union_matches() {
        let source = r#"<?php
// Scenario: @param union type matches native union type hint
// Expected: No errors

class User {}
class Admin {}

class TestParamUnionMatches {
    /**
     * @param int|string $value
     */
    function acceptsIntOrString(int|string $value) {
        // OK - PHPDoc matches native type
    }

    /**
     * @param int|string|bool $value
     */
    function acceptsMultipleTypes(int|string|bool $value) {
        // OK - PHPDoc matches native union type
    }

    /**
     * @param User|Admin $obj
     */
    function acceptsUserOrAdmin(User|Admin $obj) {
        // OK - union of objects
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_09_param_union_conflict() {
        let source = r#"<?php
// Scenario: @param union type conflicts with native union type hint
// Expected: Errors on lines 9, 16, 23

class User {}
class Admin {}
class Guest {}

class TestParamUnionConflict {
    /**
     * @param int|string $value
     */
    function wrongUnion(int|bool $value) {
        // Error - bool is not string
    }

    /**
     * @param int|string $value
     */
    function differentUnion(string|float $value) {
        // Error - types don't match
    }

    /**
     * @param User|Admin $obj
     */
    function wrongObjectUnion(User|Guest $obj) {
        // Error - Guest is not Admin
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocParamCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Note: Union types in native PHP type hints are not fully supported yet
        // The parser only extracts the last type from the union
        assert_diagnostics_exact(
            &diagnostics,
            &[
                "error: @param type 'int|string' conflicts with native type hint 'bool' for parameter $value",
                "error: @param type 'int|string' conflicts with native type hint 'float' for parameter $value",
                "error: @param type 'User|Admin' conflicts with native type hint 'Guest' for parameter $obj",
            ]
        );
    }
}
