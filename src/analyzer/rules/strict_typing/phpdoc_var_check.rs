use super::DiagnosticRule;
use super::helpers::{
    TypeHint, child_by_kind, diagnostic_for_node, extract_array_elements,
    extract_array_key_value_pairs, is_type_compatible, literal_type, node_text,
    variable_name_text, walk_node,
};
use crate::analyzer::phpdoc::{TypeExpression, extract_phpdoc_for_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

pub struct PhpDocVarCheckRule;

impl PhpDocVarCheckRule {
    pub fn new() -> Self {
        Self
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
            TypeExpression::ShapedArray(fields) => {
                // Convert shaped array to TypeHint
                let hint_fields: Option<Vec<_>> = fields
                    .iter()
                    .map(|(name, type_expr)| {
                        Self::type_expression_to_hint(type_expr).map(|hint| (name.clone(), hint))
                    })
                    .collect();
                hint_fields.map(TypeHint::ShapedArray)
            }
            _ => None,
        }
    }

    /// Check array elements match the expected array type
    fn check_array_elements(
        array_node: tree_sitter::Node,
        expected_type: &TypeHint,
        type_expr: &TypeExpression,
        parsed: &parser::ParsedSource,
        diagnostics: &mut Vec<crate::analyzer::Diagnostic>,
    ) {
        // Check if this is a shaped array type
        if let TypeHint::ShapedArray(expected_fields) = expected_type {
            Self::check_shaped_array_elements(
                array_node,
                expected_fields,
                type_expr,
                parsed,
                diagnostics,
            );
            return;
        }

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
                                "Cannot infer type of array element for {}; expected element type '{}'",
                                array_type_name, expected_name
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
                                "Array element type '{}' conflicts with expected element type '{}' for {}",
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
        diagnostics: &mut Vec<crate::analyzer::Diagnostic>,
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
                                "Cannot infer type of array key for {}; expected key type '{}'",
                                array_type_name,
                                Self::type_hint_to_string(expected_key)
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
                                "Array key type '{}' conflicts with expected key type '{}' for {}",
                                Self::type_hint_to_string(&key_type),
                                Self::type_hint_to_string(expected_key),
                                array_type_name
                            ),
                        ));
                    }
                }
            }

            // Check value type
            // Special case: if the value is an array literal and expected_value is an array type,
            // recursively validate the nested array
            if value_node.kind() == "array_creation_expression"
                && matches!(expected_value, TypeHint::Array(_) | TypeHint::GenericArray { .. })
            {
                // Recursively check nested array elements
                Self::check_array_elements(
                    value_node,
                    expected_value,
                    type_expr,
                    parsed,
                    diagnostics,
                );
            } else if let Some(value_type) = value_type_opt {
                if value_type == TypeHint::Unknown {
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        value_node,
                        Severity::Error,
                        format!(
                            "Cannot infer type of array value for {}; expected value type '{}'",
                            array_type_name,
                            Self::type_hint_to_string(expected_value)
                        ),
                    ));
                } else if !is_type_compatible(&value_type, expected_value) {
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        value_node,
                        Severity::Error,
                        format!(
                            "Array value type '{}' conflicts with expected value type '{}' for {}",
                            Self::type_hint_to_string(&value_type),
                            Self::type_hint_to_string(expected_value),
                            array_type_name
                        ),
                    ));
                }
            }
        }
    }

    /// Check shaped array (array{name: string, age: int}) fields
    /// Validates that each field exists and has the correct type, order-independent
    fn check_shaped_array_elements(
        array_node: tree_sitter::Node,
        expected_fields: &[(String, TypeHint)],
        type_expr: &TypeExpression,
        parsed: &parser::ParsedSource,
        diagnostics: &mut Vec<crate::analyzer::Diagnostic>,
    ) {
        let array_type_name = Self::type_expression_to_string(type_expr);

        // Extract all key-value pairs from the array
        let pairs = extract_array_key_value_pairs(array_node, parsed);

        // Build a map of actual field names to their values for easy lookup
        use std::collections::HashMap;
        let mut actual_fields: HashMap<String, (tree_sitter::Node, Option<TypeHint>)> = HashMap::new();

        for (key_node_opt, _key_type_opt, value_node, value_type_opt) in pairs {
            if let Some(key_node) = key_node_opt {
                // Extract the field name from the key (should be a string)
                if let Some(field_name) = node_text(key_node, parsed) {
                    // Remove quotes from string keys
                    let field_name = field_name.trim_matches('"').trim_matches('\'');
                    actual_fields.insert(field_name.to_string(), (value_node, value_type_opt));
                }
            }
        }


        // Check each expected field
        for (expected_name, expected_type) in expected_fields {

            if let Some((value_node, value_type_opt)) = actual_fields.get(expected_name) {
                // Field exists, check its type
                if let Some(value_type) = value_type_opt {
                    if *value_type == TypeHint::Unknown {
                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            *value_node,
                            Severity::Error,
                            format!(
                                "Cannot infer type of field '{}' in {}; expected type '{}'",
                                expected_name,
                                array_type_name,
                                Self::type_hint_to_string(expected_type)
                            ),
                        ));
                    } else if !is_type_compatible(value_type, expected_type) {
                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            *value_node,
                            Severity::Error,
                            format!(
                                "Field '{}' has type '{}' but expected type '{}' in {}",
                                expected_name,
                                Self::type_hint_to_string(value_type),
                                Self::type_hint_to_string(expected_type),
                                array_type_name
                            ),
                        ));
                    }
                }
            } else {
                // Field is missing
                diagnostics.push(diagnostic_for_node(
                    parsed,
                    array_node,
                    Severity::Error,
                    format!(
                        "Missing required field '{}' in {}",
                        expected_name,
                        array_type_name
                    ),
                ));
            }
        }

        // Check for unexpected fields
        for (actual_name, (value_node, _)) in &actual_fields {
            if !expected_fields.iter().any(|(name, _)| name == actual_name) {
                diagnostics.push(diagnostic_for_node(
                    parsed,
                    *value_node,
                    Severity::Error,
                    format!(
                        "Unexpected field '{}' in {}",
                        actual_name,
                        array_type_name
                    ),
                ));
            }
        }
    }
}

impl DiagnosticRule for PhpDocVarCheckRule {
    fn name(&self) -> &str {
        "strict_typing/phpdoc_var_check"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check class properties and constants with @var tags
        walk_node(parsed.tree.root_node(), &mut |node| {
            if !matches!(node.kind(), "property_declaration" | "const_declaration") {
                return;
            }

            // Extract @var PHPDoc
            if let Some(phpdoc) = extract_phpdoc_for_node(node, parsed) {
                if let Some(var_tag) = phpdoc.var_tag {
                    // Find the property or const initializer
                    for i in 0..node.named_child_count() {
                        if let Some(child) = node.named_child(i) {
                            // Handle both property_element and const_element
                            if matches!(child.kind(), "property_element" | "const_element") {
                                // Check if there's a property_initializer or const_element value
                                let initializer_opt = if child.kind() == "property_element" {
                                    child_by_kind(child, "property_initializer")
                                } else {
                                    // For const_element, the value is the second child after the name
                                    child.named_child(1)
                                };

                                if let Some(initializer) = initializer_opt {
                                    // Get the value node
                                    let value_node_opt = if child.kind() == "property_element" {
                                        initializer.named_child(0)
                                    } else {
                                        // For const_element, the initializer IS the value
                                        Some(initializer)
                                    };

                                    if let Some(value_node) = value_node_opt {
                                        // Check if it's an array and validate elements
                                        if value_node.kind() == "array_creation_expression" {
                                            if let Some(expected_type) =
                                                Self::type_expression_to_hint(&var_tag.type_expr)
                                            {
                                                Self::check_array_elements(
                                                    value_node,
                                                    &expected_type,
                                                    &var_tag.type_expr,
                                                    parsed,
                                                    &mut diagnostics,
                                                );
                                            }
                                        } else {
                                            // Get the literal type of the value
                                            if let Some(actual_type) = literal_type(value_node) {
                                                // Get the expected type from @var
                                                if let Some(expected_type) =
                                                    Self::type_expression_to_hint(&var_tag.type_expr)
                                                {
                                                    // Check if types are compatible
                                                    if !is_type_compatible(&actual_type, &expected_type) {
                                                        let expected_name =
                                                            Self::type_expression_to_string(
                                                                &var_tag.type_expr,
                                                            );
                                                        let actual_name =
                                                            Self::type_hint_to_string(&actual_type);

                                                        diagnostics.push(diagnostic_for_node(
                                                            parsed,
                                                            value_node,
                                                            Severity::Error,
                                                            format!(
                                                                "@var type '{}' conflicts with assigned value type '{}'",
                                                                expected_name, actual_name
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
                    }
                }
            }
        });

        // Check inline @var assignments
        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "expression_statement" {
                return;
            }

            let Some(phpdoc) = extract_phpdoc_for_node(node, parsed) else {
                return;
            };
            let Some(var_tag) = phpdoc.var_tag else {
                return;
            };

            let Some(assign) = child_by_kind(node, "assignment_expression") else {
                return;
            };

            let Some(value_node) = assign.child_by_field_name("right") else {
                return;
            };

            if let Some(expected_type) = Self::type_expression_to_hint(&var_tag.type_expr) {
                // Validate variable name matches if specified
                if let Some(expected_name) = var_tag.name.as_ref() {
                    if let Some(left_node) = assign.child_by_field_name("left") {
                        if let Some(variable_name) = variable_name_text(left_node, parsed) {
                            if &variable_name != expected_name {
                                return;
                            }
                        } else {
                            return;
                        }
                    } else {
                        return;
                    }
                }

                // Check if it's an array and validate elements
                if value_node.kind() == "array_creation_expression" {
                    Self::check_array_elements(
                        value_node,
                        &expected_type,
                        &var_tag.type_expr,
                        parsed,
                        &mut diagnostics,
                    );
                } else if let Some(actual_type) = literal_type(value_node) {
                    // Check non-array literal types
                    if !is_type_compatible(&actual_type, &expected_type) {
                        let expected_name_str = Self::type_expression_to_string(&var_tag.type_expr);
                        let actual_name_str = Self::type_hint_to_string(&actual_type);

                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            value_node,
                            Severity::Error,
                            format!(
                                "@var type '{}' conflicts with assigned value type '{}'",
                                expected_name_str, actual_name_str
                            ),
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
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_has_diagnostics, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_var_generic_array_conflict() {
        let source = r#"<?php

class Test {
    /**
     * @var array<string, int>
     */
    private $map = [
        "key1" => 123,
        999 => 456,
        "key2" => "wrong"
    ];
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
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
    fn test_var_generic_array_matches() {
        let source = r#"<?php

class Test {
    /**
     * @var array<string, int>
     */
    private $map = ["key1" => 123, "key2" => 456];

    /**
     * @var array<int, string>
     */
    private $names = [0 => "Alice", 1 => "Bob"];
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_var_object_array_conflict() {
        let source = r#"<?php

class User {
    public $name;
}

class Admin {
    public $role;
}

class TestObjectArrayConflict {
    /**
     * @var User[]
     */
    private $users = [new User(), new Admin()];

    /**
     * @var Admin[]
     */
    private $admins = [new User(), new Admin()];
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(&diagnostics, &[
            "error: Array element type 'Admin' conflicts with expected element type 'User' for User[]",
            "error: Array element type 'User' conflicts with expected element type 'Admin' for Admin[]",
        ]);
    }

    #[test]
    fn test_var_object_array_matches() {
        let source = r#"<?php

class User {
    public $name;
}

class Admin {
    public $role;
}

class TestObjectArrayMatches {
    /**
     * @var User[]
     */
    private $users = [new User(), new User()];

    /**
     * @var Admin[]
     */
    private $admins = [new Admin()];
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_correct_property_type() {
        let source = r#"<?php
class CorrectProperty {
    /**
     * @var string
     */
    private $name = "test";
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_wrong_property_type() {
        let source = r#"<?php
class WrongPropertyType {
    /**
     * @var string
     */
    private $name = 123;  // Error: assigning int to string property
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: @var type 'string' conflicts with assigned value type 'int'"]
        );
    }

    #[test]
    fn test_wrong_inline_var() {
        let source = r#"<?php
function wrongInlineVar() {
    /** @var string $value */
    $value = 123;  // Error: assigning int to string variable
    echo $value;
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: @var type 'string' conflicts with assigned value type 'int'"]
        );
    }

    #[test]
    fn test_wrong_array_element_type() {
        let source = r#"<?php
function wrongArrayElementType() {
    /** @var User[] $users */
    $users = [1, 2, 3];  // Error: array contains int, expected User[]
    return $users;
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_has_diagnostics(&diagnostics, "Array element");
    }

    #[test]
    fn test_wrong_assoc_array_type() {
        let source = r#"<?php
function wrongAssocArrayType() {
    /** @var array<string, int> $scores */
    $scores = [1 => "wrong"];  // Error: int key and string value, expected string key and int value
    return $scores;
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_has_diagnostics(&diagnostics, "array");
    }

    #[test]
    fn test_multiple_properties_wrong_type() {
        let source = r#"<?php
class MultipleProperties {
    /**
     * @var int
     */
    private $id;

    /**
     * @var string
     */
    private $name;

    /**
     * @var bool
     */
    private $active = "yes";  // Error: string assigned to bool property
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: @var type 'bool' conflicts with assigned value type 'string'"]
        );
    }

    #[test]
    fn test_property_type_change() {
        let source = r#"<?php
class PropertyTypeChange {
    /**
     * @var string
     */
    private $value;

    public function __construct() {
        $this->value = "initial";
    }

    public function setValue() {
        $this->value = 999;  // Error: assigning int to string property
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // This rule only checks property initializers, not reassignments in methods
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_static_property_wrong_type() {
        let source = r#"<?php
class StaticProperty {
    /**
     * @var int
     */
    private static $counter = 0;

    public static function increment() {
        self::$counter = "wrong";  // Error: assigning string to int property
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // This rule only checks property initializers
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_constant_wrong_type() {
        let source = r#"<?php
class ConstantType {
    /**
     * @var int
     */
    public const MAX_SIZE = "100";  // Error: string assigned to int constant
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: @var type 'int' conflicts with assigned value type 'string'"]
        );
    }

    #[test]
    fn test_nested_array_var() {
        let source = r#"<?php
function nestedArrayVar() {
    /** @var array<string, array<int, User>> $userGroups */
    $userGroups = [
        "admin" => [new User()],
        "guest" => [1, 2, 3]  // Error: int[] instead of User[]
    ];
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_has_diagnostics(&diagnostics, "array");
    }

    #[test]
    fn test_array_destructuring() {
        let source = r#"<?php
function arrayDestructuring() {
    /** @var array{id: int, name: string} $data */
    $data = ["id" => 1, "name" => "test"];

    /** @var array{id: int, name: string} $wrong */
    $wrong = ["id" => "one", "name" => 123];  // Error: id should be int, name should be string
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_has_diagnostics(&diagnostics, "array");
    }

    #[test]
    fn test_global_variable_wrong_type() {
        let source = r#"<?php
/** @var string $globalString */
$globalString = 123;  // Error: int assigned to string variable
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: @var type 'string' conflicts with assigned value type 'int'"]
        );
    }

    // Tests from phpdoc_var_scenarios

    #[test]
    fn test_scenario_01_correct_property() {
        let source = r#"<?php
// Scenario: Property with correct @var type
// Expected: No errors

class CorrectProperty {
    /**
     * @var string
     */
    private $name = "test";

    /**
     * @var int
     */
    private $age = 25;

    /**
     * @var bool
     */
    private $active = true;
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_02_wrong_property_type() {
        let source = r#"<?php
// Scenario: Property assigned wrong type vs @var
// Expected: Error on line 8 (string property with int value)

class WrongPropertyType {
    /**
     * @var string
     */
    private $name = 123;  // Error: int assigned to string property
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: @var type 'string' conflicts with assigned value type 'int'"]
        );
    }

    #[test]
    fn test_scenario_03_inline_var_cast() {
        let source = r#"<?php
// Scenario: Inline @var type casting in assignment
// Expected: No errors - valid type narrowing

function createDate(): object {
    return new \DateTime();
}

function inlineVarCast() {
    /** @var \DateTime $date */
    $date = createDate();
    $date->format('Y-m-d');  // OK: $date is known to be DateTime
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_04_wrong_inline_var() {
        let source = r#"<?php
// Scenario: Inline @var claims wrong type
// Expected: Error on line 5 (assigning int to string variable)

function wrongInlineVar() {
    /** @var string $value */
    $value = 123;  // Error: assigning int to string variable
    echo $value;
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: @var type 'string' conflicts with assigned value type 'int'"]
        );
    }

    #[test]
    fn test_scenario_05_reassignment_violation() {
        let source = r#"<?php
// Scenario: Variable reassigned to incompatible type after @var
// Expected: Error on reassignment (implementation detects this)

function reassignmentAfterVar() {
    /** @var string $text */
    $text = "hello";
    $text = 456;  // Error: int assigned to string variable
    echo $text;
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // The rule detects the reassignment because it's within the scope
        // where the @var declaration is still active
        assert_diagnostics_exact(
            &diagnostics,
            &["error: @var type 'string' conflicts with assigned value type 'int'"]
        );
    }

    #[test]
    fn test_scenario_06_generic_array() {
        let source = r#"<?php
// Scenario: @var with generic array type
// Expected: No errors - correct User[] array

class User {}

class UserCollection {
    /**
     * @var User[]
     */
    private $users = [];

    public function addUser(User $user): void {
        $this->users[] = $user;
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_07_var_union_matches() {
        let source = r#"<?php
// Scenario: @var union type with values that match one of the union members
// Expected: No errors

class User {}
class Admin {}

class TestVarUnionMatches {
    /**
     * @var int|string
     */
    private $intOrString = 123;

    /**
     * @var int|string
     */
    private $intOrString2 = "hello";

    /**
     * @var User|Admin
     */
    private $userOrAdmin;

    /**
     * @var int|string|bool
     */
    private $multiType = true;
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_08_var_union_conflict() {
        let source = r#"<?php
// Scenario: @var union type conflicts with assigned value type
// Expected: Errors on lines 9, 15, 21

class User {}
class Admin {}

class TestVarUnionConflict {
    /**
     * @var int|string
     */
    private $wrongType = true;

    /**
     * @var int|string
     */
    private $wrongType2 = 1.5;

    /**
     * @var User|Admin
     */
    private $wrongObject = 123;
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &[
                "error: @var type 'int|string' conflicts with assigned value type 'bool'",
                "error: @var type 'int|string' conflicts with assigned value type 'float'",
                "error: @var type 'User|Admin' conflicts with assigned value type 'int'",
            ]
        );
    }

    #[test]
    fn test_scenario_09_var_array_simple() {
        let source = r#"<?php
// Scenario: @var with simple array types (int[], string[])
// Expected: No errors for empty arrays

class TestSimpleArrays {
    /**
     * @var int[]
     */
    private $integers = [];

    /**
     * @var string[]
     */
    private $strings = [];

    /**
     * @var bool[]
     */
    private $flags = [];
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_10_var_array_elements_match() {
        let source = r#"<?php
// Scenario: @var array type with matching element types
// Expected: No errors - all elements match the declared type

class TestArrayElementsMatch {
    /**
     * @var int[]
     */
    private $integers = [1, 2, 3];

    /**
     * @var string[]
     */
    private $strings = ["hello", "world"];

    /**
     * @var bool[]
     */
    private $flags = [true, false, true];
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_scenario_11_var_array_elements_conflict() {
        let source = r#"<?php
// Scenario: @var array type with mismatched element types
// Expected: Errors on lines 11, 17, 23

class TestArrayElementsConflict {
    /**
     * @var int[]
     */
    private $integers = [1, "string", 3]; // Error: string in int array

    /**
     * @var string[]
     */
    private $strings = ["hello", 123]; // Error: int in string array

    /**
     * @var bool[]
     */
    private $flags = [true, "false"]; // Error: string in bool array
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocVarCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &[
                "error: Array element type 'string' conflicts with expected element type 'int' for int[]",
                "error: Array element type 'int' conflicts with expected element type 'string' for string[]",
                "error: Array element type 'string' conflicts with expected element type 'bool' for bool[]",
            ]
        );
    }
}
