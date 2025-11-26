use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, literal_type, walk_node, TypeHint};
use crate::analyzer::phpdoc::{extract_phpdoc_for_node, TypeExpression};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{parser, Severity};

pub struct PhpDocVarCheckRule;

impl PhpDocVarCheckRule {
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
                _ => None,
            },
            TypeExpression::Nullable(inner) => Self::type_expression_to_hint(inner),
            _ => None,
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
                                if let Some(initializer) = child_by_kind(child, "property_initializer") {
                                    // Get the value node (skip the = sign)
                                    if let Some(value_node) = initializer.named_child(0) {
                                        // Get the literal type of the value
                                        if let Some(actual_type) = literal_type(value_node) {
                                            // Get the expected type from @var
                                            if let Some(expected_type) = Self::type_expression_to_hint(&var_tag.type_expr) {
                                                // Check if types match
                                                if actual_type != expected_type {
                                                    let expected_name = match &var_tag.type_expr {
                                                        TypeExpression::Simple(s) => s.clone(),
                                                        TypeExpression::Nullable(inner) => {
                                                            if let TypeExpression::Simple(s) = inner.as_ref() {
                                                                format!("?{}", s)
                                                            } else {
                                                                "unknown".to_string()
                                                            }
                                                        }
                                                        _ => "unknown".to_string(),
                                                    };

                                                    let actual_name = match actual_type {
                                                        TypeHint::Int => "int",
                                                        TypeHint::String => "string",
                                                        TypeHint::Bool => "bool",
                                                        TypeHint::Float => "float",
                                                        _ => "unknown",
                                                    };

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
        });

        diagnostics
    }
}
