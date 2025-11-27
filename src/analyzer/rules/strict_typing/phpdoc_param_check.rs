use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, type_hint_from_parameter, walk_node, TypeHint};
use crate::analyzer::phpdoc::{extract_phpdoc_for_node, TypeExpression};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{parser, Severity};

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
            TypeExpression::Nullable(inner) => Self::type_expression_to_hint(inner),
            _ => None,
        }
    }

    /// Get parameter name from a parameter node
    fn get_param_name(param_node: tree_sitter::Node, parsed: &parser::ParsedSource) -> Option<String> {
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
                            if !matches!(param_node.kind(), "simple_parameter" | "variadic_parameter" | "property_promotion_parameter") {
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

                                    let phpdoc_hint = Self::type_expression_to_hint(expected_type_expr);

                                    // Check for conflict
                                    if let Some(phpdoc) = phpdoc_hint {
                                        if native_hint != phpdoc {
                                            let expected_name = match expected_type_expr {
                                                TypeExpression::Simple(s) => s.clone(),
                                                _ => "unknown".to_string(),
                                            };

                                            let native_type_str = match &native_hint {
                                                TypeHint::Int => "int",
                                                TypeHint::String => "string",
                                                TypeHint::Bool => "bool",
                                                TypeHint::Float => "float",
                                                TypeHint::Object(name) => name.as_str(),
                                                TypeHint::Unknown => "unknown",
                                            };

                                            // Find the type node for error reporting
                                            let type_node = child_by_kind(param_node, "primitive_type")
                                                .or_else(|| child_by_kind(param_node, "named_type"))
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
