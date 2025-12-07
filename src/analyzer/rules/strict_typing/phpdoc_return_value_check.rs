use super::helpers::{
    TypeHint, child_by_kind, diagnostic_for_node, extract_array_elements,
    extract_array_key_value_pairs, infer_type, is_type_compatible, walk_node,
};
use crate::analyzer::phpdoc::{TypeExpression, extract_phpdoc_for_node};
use crate::analyzer::rules::DiagnosticRule;
use crate::analyzer::{Diagnostic, Severity, parser, project::ProjectContext};

/// Validates that actual return values match @return types
///
/// This rule checks that the values returned from functions match their @return PHPDoc declarations.
///
/// # Examples
///
/// ```php
/// // ✗ Error: Return value conflicts with @return type
/// /**
///  * @return int
///  */
/// function test() {
///     return "string";  // Error: string instead of int
/// }
///
/// // ✓ OK: Return value matches @return type
/// /**
///  * @return int
///  */
/// function test() {
///     return 42;  // OK
/// }
/// ```
pub struct PhpDocReturnValueCheckRule;

impl PhpDocReturnValueCheckRule {
    pub fn new() -> Self {
        Self
    }

    /// Convert PHPDoc TypeExpression to our internal TypeHint
    fn type_expression_to_hint(expr: &TypeExpression) -> Option<TypeHint> {
        match expr {
            TypeExpression::Simple(s) => match s.as_str() {
                "int" | "integer" => Some(TypeHint::Int),
                "string" => Some(TypeHint::String),
                "bool" | "boolean" => Some(TypeHint::Bool),
                "float" | "double" => Some(TypeHint::Float),
                _ => Some(TypeHint::Object(s.clone())),
            },
            TypeExpression::Nullable(inner) => {
                Self::type_expression_to_hint(inner).map(|t| TypeHint::Nullable(Box::new(t)))
            }
            TypeExpression::Union(types) => {
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
                Self::type_expression_to_hint(inner).map(|t| TypeHint::Array(Box::new(t)))
            }
            TypeExpression::Generic { base, params } => {
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

    fn type_expression_to_string(expr: &TypeExpression) -> String {
        match expr {
            TypeExpression::Simple(s) => s.trim_start_matches('\\').to_string(),
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

    fn type_hint_to_string(hint: &TypeHint) -> String {
        match hint {
            TypeHint::Int => "int".to_string(),
            TypeHint::String => "string".to_string(),
            TypeHint::Bool => "bool".to_string(),
            TypeHint::Float => "float".to_string(),
            TypeHint::Object(name) => name.clone(),
            TypeHint::Nullable(inner) => format!("?{}", Self::type_hint_to_string(inner)),
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

    /// Check array elements match the expected array type
    fn check_array_elements(
        array_node: tree_sitter::Node,
        expected_type: &TypeHint,
        type_expr: &TypeExpression,
        parsed: &parser::ParsedSource,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        // Check if this is a generic array type
        if let TypeHint::GenericArray {
            key: expected_key,
            value: expected_value,
        } = expected_type
        {
            Self::check_generic_array_elements(
                array_node,
                expected_key,
                expected_value,
                type_expr,
                parsed,
                diagnostics,
            );
            return;
        }

        // Extract the expected element type from simple array types
        let expected_elem_type = match expected_type {
            TypeHint::Array(elem_type) => Some(elem_type.as_ref()),
            _ => None,
        };

        if let Some(expected_elem) = expected_elem_type {
            // Extract all elements from the array
            let elements = extract_array_elements(array_node, parsed);

            for (elem_node, elem_type_opt) in elements {
                if let Some(elem_type) = elem_type_opt {
                    // Check if the type is unknown (couldn't be inferred)
                    if elem_type == TypeHint::Unknown {
                        let expected_name = Self::type_hint_to_string(expected_elem);
                        let array_type_name = Self::type_expression_to_string(type_expr);

                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            elem_node,
                            Severity::Error,
                            format!(
                                "Cannot infer type of array element; expected element type '{}' for @return type '{}'",
                                expected_name, array_type_name
                            ),
                        ));
                    } else if !is_type_compatible(&elem_type, expected_elem) {
                        // Check if element type is compatible with expected element type
                        let expected_name = Self::type_hint_to_string(expected_elem);
                        let actual_name = Self::type_hint_to_string(&elem_type);
                        let array_type_name = Self::type_expression_to_string(type_expr);

                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            elem_node,
                            Severity::Error,
                            format!(
                                "Array element type '{}' conflicts with expected element type '{}' for @return type '{}'",
                                actual_name, expected_name, array_type_name
                            ),
                        ));
                    }
                }
            }
        }
    }

    /// Check generic array (array<K, V>) key-value pairs
    fn check_generic_array_elements(
        array_node: tree_sitter::Node,
        expected_key: &TypeHint,
        expected_value: &TypeHint,
        type_expr: &TypeExpression,
        parsed: &parser::ParsedSource,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let pairs = extract_array_key_value_pairs(array_node, parsed);
        let array_type_name = Self::type_expression_to_string(type_expr);

        for (key_node_opt, key_type_opt, value_node, value_type_opt) in pairs {
            // Check key type
            if let Some(key_type) = key_type_opt {
                if key_type == TypeHint::Unknown {
                    if let Some(key_node) = key_node_opt {
                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            key_node,
                            Severity::Error,
                            format!(
                                "Cannot infer type of array key; expected key type '{}' for @return type '{}'",
                                Self::type_hint_to_string(expected_key),
                                array_type_name
                            ),
                        ));
                    }
                } else if !is_type_compatible(&key_type, expected_key) {
                    if let Some(key_node) = key_node_opt {
                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            key_node,
                            Severity::Error,
                            format!(
                                "Array key type '{}' conflicts with expected key type '{}' for @return type '{}'",
                                Self::type_hint_to_string(&key_type),
                                Self::type_hint_to_string(expected_key),
                                array_type_name
                            ),
                        ));
                    }
                }
            }

            // Check value type
            if let Some(value_type) = value_type_opt {
                if value_type == TypeHint::Unknown {
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        value_node,
                        Severity::Error,
                        format!(
                            "Cannot infer type of array value; expected value type '{}' for @return type '{}'",
                            Self::type_hint_to_string(expected_value),
                            array_type_name
                        ),
                    ));
                } else if !is_type_compatible(&value_type, expected_value) {
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        value_node,
                        Severity::Error,
                        format!(
                            "Array value type '{}' conflicts with expected value type '{}' for @return type '{}'",
                            Self::type_hint_to_string(&value_type),
                            Self::type_hint_to_string(expected_value),
                            array_type_name
                        ),
                    ));
                }
            }
        }
    }
}

impl DiagnosticRule for PhpDocReturnValueCheckRule {
    fn name(&self) -> &str {
        "strict_typing/phpdoc_return_value_check"
    }

    fn run(&self, parsed: &parser::ParsedSource, _context: &ProjectContext) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if !matches!(node.kind(), "function_definition" | "method_declaration") {
                return;
            }

            // Extract @return PHPDoc
            let Some(phpdoc) = extract_phpdoc_for_node(node, parsed) else {
                return;
            };

            let Some(return_tag) = &phpdoc.return_tag else {
                return;
            };

            // Get expected return type from @return
            let Some(expected_type) = Self::type_expression_to_hint(&return_tag.type_expr) else {
                return;
            };

            // Find the function body
            let Some(body) = child_by_kind(node, "compound_statement") else {
                return;
            };

            // Check all return statements in the function
            walk_node(body, &mut |ret_node| {
                if ret_node.kind() != "return_statement" {
                    return;
                }

                // Get the return value - use child_by_kind for more reliable detection
                // This works for both single-line and multi-line arrays
                let value_node = if let Some(array_node) = child_by_kind(ret_node, "array_creation_expression") {
                    Some(array_node)
                } else {
                    // For non-array returns, find the first non-comment named child
                    let mut found = None;
                    for idx in 0..ret_node.named_child_count() {
                        if let Some(child) = ret_node.named_child(idx) {
                            if child.kind() != "comment" {
                                found = Some(child);
                                break;
                            }
                        }
                    }
                    found
                };

                if let Some(value_node) = value_node {
                    // Check if this is an array literal and we expect an array type
                    if value_node.kind() == "array_creation_expression"
                        && matches!(expected_type, TypeHint::Array(_) | TypeHint::GenericArray { .. })
                    {
                        // Validate array elements (handles both simple and generic arrays)
                        Self::check_array_elements(
                            value_node,
                            &expected_type,
                            &return_tag.type_expr,
                            parsed,
                            &mut diagnostics,
                        );
                    } else {
                        // Infer the type of the return value
                        if let Some(actual_type) = infer_type(value_node, parsed) {
                            // Check if unknown type
                            if actual_type == TypeHint::Unknown {
                                diagnostics.push(diagnostic_for_node(
                                    parsed,
                                    value_node,
                                    Severity::Error,
                                    format!(
                                        "Cannot infer type of return value; expected @return type '{}'",
                                        Self::type_expression_to_string(&return_tag.type_expr)
                                    ),
                                ));
                            } else if !is_type_compatible(&actual_type, &expected_type) {
                                // Check if types are compatible
                                let actual_name = Self::type_hint_to_string(&actual_type);
                                let expected_name =
                                    Self::type_expression_to_string(&return_tag.type_expr);

                                diagnostics.push(diagnostic_for_node(
                                    parsed,
                                    value_node,
                                    Severity::Error,
                                    format!(
                                        "Return value type '{}' conflicts with @return type '{}'",
                                        actual_name, expected_name
                                    ),
                                ));
                            }
                        }
                    }
                }
            });
        });

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_has_diagnostics, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_return_array_conflict() {
        let source = r#"<?php

class TestReturnArrayConflict {
    /**
     * @return int[]
     */
    function getIntegers(): array {
        return [1, "string", 3];
    }

    /**
     * @return string[]
     */
    function getStrings(): array {
        return ["hello", 123];
    }

    /**
     * @return bool[]
     */
    function getFlags(): array {
        return [true, "false"];
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &[
            "error: Array element type 'string' conflicts with expected element type 'int' for @return type 'int[]'",
            "error: Array element type 'int' conflicts with expected element type 'string' for @return type 'string[]'",
            "error: Array element type 'string' conflicts with expected element type 'bool' for @return type 'bool[]'",
        ]);
    }

    #[test]
    fn test_return_array_matches() {
        let source = r#"<?php

class TestReturnArrayMatches {
    /**
     * @return int[]
     */
    function getIntegers(): array {
        return [1, 2, 3];
    }

    /**
     * @return string[]
     */
    function getStrings(): array {
        return ["hello", "world"];
    }

    /**
     * @return bool[]
     */
    function getFlags(): array {
        return [true, false, true];
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_return_generic_array_conflict() {
        let source = r#"<?php

class Test {
    /**
     * @return array<string, int>
     */
    public function getMap() {
        return [
            "key1" => 123,
            999 => 456,
            "key2" => "wrong"
        ];
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Should detect: wrong key type (int 999 instead of string) and wrong value type (string "wrong" instead of int)
        assert_has_diagnostics(&diagnostics, "generic array type conflicts");
        
        // Check that we have at least one error about key or value type conflict
        let has_key_error = diagnostics.iter().any(|d| d.message.contains("key type"));
        let has_value_error = diagnostics.iter().any(|d| d.message.contains("value type"));
        assert!(
            has_key_error || has_value_error,
            "Expected errors about key or value type conflicts, but got: {:?}",
            diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_return_generic_array_matches() {
        let source = r#"<?php

class Test {
    /**
     * @return array<string, int>
     */
    public function getMap() {
        return ["key1" => 123, "key2" => 456];
    }

    /**
     * @return array<int, string>
     */
    public function getNames() {
        return [0 => "Alice", 1 => "Bob"];
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_return_object_array_conflict() {
        let source = r#"<?php

class User {
    public $name;
}

class Admin {
    public $role;
}

class TestReturnObjectArrayConflict {
    /**
     * @return User[]
     */
    function getUsers(): array {
        return [new User(), new Admin()];
    }

    /**
     * @return Admin[]
     */
    function getAdmins(): array {
        return [new User(), new Admin()];
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &[
            "error: Array element type 'Admin' conflicts with expected element type 'User' for @return type 'User[]'",
            "error: Array element type 'User' conflicts with expected element type 'Admin' for @return type 'Admin[]'",
        ]);
    }

    #[test]
    fn test_return_object_array_matches() {
        let source = r#"<?php

class User {
    public $name;
}

class Admin {
    public $role;
}

class TestReturnObjectArrayMatches {
    /**
     * @return User[]
     */
    function getUsers(): array {
        return [new User(), new User()];
    }

    /**
     * @return Admin[]
     */
    function getAdmins(): array {
        return [new Admin()];
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_return_value_conflict() {
        let source = r#"<?php

class TestReturnValueConflict {
    /**
     * @return int
     */
    function getNumber(): int {
        return "not a number";
    }

    /**
     * @return string
     */
    function getName(): string {
        return 123;
    }

    /**
     * @return bool
     */
    function isValid(): bool {
        return "yes";
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &[
            "error: Return value type 'string' conflicts with @return type 'int'",
            "error: Return value type 'int' conflicts with @return type 'string'",
            "error: Return value type 'string' conflicts with @return type 'bool'",
        ]);
    }

    #[test]
    fn test_return_value_matches() {
        let source = r#"<?php

class TestReturnValueMatches {
    /**
     * @return int
     */
    function getNumber(): int {
        return 42;
    }

    /**
     * @return string
     */
    function getName(): string {
        return "Alice";
    }

    /**
     * @return bool
     */
    function isValid(): bool {
        return true;
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_correct_return_matching_native_type() {
        let source = r#"<?php
/**
 * @return int
 */
function correctReturn(): int {
    return 42;
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_wrong_return_value() {
        let source = r#"<?php
/**
 * @return int
 */
function wrongReturnValue(): int {
    return "not an int";  // Error: returning string when int expected
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: Return value type 'string' conflicts with @return type 'int'"]
        );
    }

    #[test]
    fn test_inconsistent_returns() {
        let source = r#"<?php
/**
 * @return int
 */
function inconsistentReturns(bool $condition) {
    if ($condition) {
        return 42;
    }
    return "string";  // Error: returning string when int expected
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: Return value type 'string' conflicts with @return type 'int'"]
        );
    }

    #[test]
    fn test_return_void_with_no_return() {
        let source = r#"<?php
/**
 * @return void
 */
function returnsVoid(): void {
    echo "side effect";
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_wrong_array_element_type() {
        let source = r#"<?php
/**
 * @return string[]
 */
function wrongArrayElements(): array {
    return [1, 2, 3];  // Error: returning int[] when string[] expected
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &[
                "error: Array element type 'int' conflicts with expected element type 'string' for @return type 'string[]'",
                "error: Array element type 'int' conflicts with expected element type 'string' for @return type 'string[]'",
                "error: Array element type 'int' conflicts with expected element type 'string' for @return type 'string[]'",
            ]
        );
    }

    #[test]
    fn test_wrong_assoc_array_return() {
        let source = r#"<?php
/**
 * @return array<string, int>
 */
function wrongAssocArrayReturn(): array {
    return [1 => "wrong"];  // Error: int key, string value instead of string key, int value
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Should detect both key and value type mismatches
        assert_has_diagnostics(&diagnostics, "array");
    }

    #[test]
    fn test_wrong_object_return() {
        let source = r#"<?php
/**
 * @return \DateTime
 */
function wrongObjectReturn(): \DateTime {
    return new \DateTimeImmutable();  // Error: DateTimeImmutable is not DateTime (even if related)
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: Return value type 'DateTimeImmutable' conflicts with @return type 'DateTime'"]
        );
    }

    #[test]
    fn test_mixed_return_becoming_specific() {
        let source = r#"<?php   
/**
 * @return string  // Claims string
 */
function claimsStringReturn() {  // No native hint (accepts mixed)
    return 123;  // Error: returning int when string expected
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: Return value type 'int' conflicts with @return type 'string'"]
        );
    }

    // Tests from phpdoc_return_scenarios

    #[test]
    fn test_scenario_09_return_value_matches() {
        let source = r#"<?php
// Scenario: @return type matches actual return values
// Expected: No errors

class TestReturnValueMatches {
    /**
     * @return int
     */
    function getNumber(): int {
        return 42;
    }

    /**
     * @return string
     */
    function getName(): string {
        return "Alice";
    }

    /**
     * @return bool
     */
    function isValid(): bool {
        return true;
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_10_return_value_conflict() {
        let source = r#"<?php
// Scenario: @return type conflicts with actual return values
// Expected: Errors on lines 11, 19, 27

class TestReturnValueConflict {
    /**
     * @return int
     */
    function getNumber(): int {
        return "not a number"; // Error: string instead of int
    }

    /**
     * @return string
     */
    function getName(): string {
        return 123; // Error: int instead of string
    }

    /**
     * @return bool
     */
    function isValid(): bool {
        return "yes"; // Error: string instead of bool
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocReturnValueCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &[
                "error: Return value type 'string' conflicts with @return type 'int'",
                "error: Return value type 'int' conflicts with @return type 'string'",
                "error: Return value type 'string' conflicts with @return type 'bool'",
            ]
        );
    }
}
