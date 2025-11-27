use super::DiagnosticRule;
use super::helpers::{
    TypeHint, child_by_kind, diagnostic_for_node, literal_type, variable_name_text, walk_node,
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
                                if let Some(initializer) =
                                    child_by_kind(child, "property_initializer")
                                {
                                    // Get the value node (skip the = sign)
                                    if let Some(value_node) = initializer.named_child(0) {
                                        // Get the literal type of the value
                                        if let Some(actual_type) = literal_type(value_node) {
                                            // Get the expected type from @var
                                            if let Some(expected_type) =
                                                Self::type_expression_to_hint(&var_tag.type_expr)
                                            {
                                                // Check if types match
                                                if actual_type != expected_type {
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

            let Some(actual_type) = literal_type(value_node) else {
                return;
            };

            if let Some(expected_type) = Self::type_expression_to_hint(&var_tag.type_expr) {
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

                if actual_type != expected_type {
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
        });

        diagnostics
    }
}
