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
