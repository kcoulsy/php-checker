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
            if let Some(value_type) = value_type_opt {
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

        // Check class properties with @var tags
        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "property_declaration" {
                return;
            }

            // Extract @var PHPDoc
            if let Some(phpdoc) = extract_phpdoc_for_node(node, parsed) {
                if let Some(var_tag) = phpdoc.var_tag {
                    // Find the property initializer
                    for i in 0..node.named_child_count() {
                        if let Some(child) = node.named_child(i) {
                            if child.kind() == "property_element" {
                                // Check if there's a property_initializer
                                if let Some(initializer) =
                                    child_by_kind(child, "property_initializer")
                                {
                                    // Get the value node (skip the = sign)
                                    if let Some(value_node) = initializer.named_child(0) {
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
