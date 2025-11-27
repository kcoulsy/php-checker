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

                // Get the return value
                if let Some(value_node) = ret_node.named_child(0) {
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
