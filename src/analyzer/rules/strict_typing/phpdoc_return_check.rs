use super::helpers::{child_by_kind, diagnostic_for_node, node_text, walk_node, TypeHint};
use crate::analyzer::phpdoc::{extract_phpdoc_for_node, TypeExpression};
use crate::analyzer::rules::DiagnosticRule;
use crate::analyzer::{parser, project::ProjectContext, Diagnostic, Severity};

/// Validates that @return types match native return type hints
///
/// This rule checks for conflicts between PHPDoc @return declarations and native PHP return type hints.
///
/// # Examples
///
/// ```php
/// // ✗ Error: @return type conflicts with native return type
/// /**
///  * @return string
///  */
/// function test(): int {
///     return 42;
/// }
///
/// // ✓ OK: @return type matches native type
/// /**
///  * @return int
///  */
/// function test(): int {
///     return 42;
/// }
/// ```
pub struct PhpDocReturnCheckRule;

impl PhpDocReturnCheckRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for PhpDocReturnCheckRule {
    fn name(&self) -> &str {
        "strict_typing/phpdoc_return_check"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if !matches!(node.kind(), "function_definition" | "method_declaration") {
                return;
            }

            // Extract PHPDoc comment
            let Some(phpdoc) = extract_phpdoc_for_node(node, parsed) else {
                return;
            };

            // Get @return tag
            let Some(return_tag) = &phpdoc.return_tag else {
                return;
            };

            // Get native return type hint
            let Some(native_type_node) = child_by_kind(node, "return_type")
                .or_else(|| child_by_kind(node, "union_type"))
                .or_else(|| child_by_kind(node, "intersection_type"))
            else {
                return;
            };

            // Extract the actual type from return_type node (it may have a child like primitive_type)
            let actual_type_node = child_by_kind(native_type_node, "primitive_type")
                .or_else(|| child_by_kind(native_type_node, "named_type"))
                .or_else(|| child_by_kind(native_type_node, "optional_type"))
                .unwrap_or(native_type_node);

            let Some(native_type_text) = node_text(actual_type_node, parsed) else {
                return;
            };

            // Convert native type to TypeHint
            let native_hint = text_to_type_hint(&native_type_text);

            // Convert PHPDoc type to TypeHint
            let phpdoc_hint = type_expression_to_hint(&return_tag.type_expr);

            // Check for conflicts
            if let (Some(native), Some(phpdoc)) = (native_hint, phpdoc_hint) {
                if native != phpdoc {
                    let message = format!(
                        "@return type '{}' conflicts with native return type hint '{}'",
                        type_expression_to_string(&return_tag.type_expr),
                        native_type_text
                    );

                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        actual_type_node,
                        Severity::Error,
                        message,
                    ));
                }
            }
        });

        diagnostics
    }
}

fn text_to_type_hint(text: &str) -> Option<TypeHint> {
    match text {
        "int" => Some(TypeHint::Int),
        "string" => Some(TypeHint::String),
        "bool" | "boolean" => Some(TypeHint::Bool),
        "float" | "double" => Some(TypeHint::Float),
        // Anything else is treated as an object type (class/interface name)
        _ => Some(TypeHint::Object(text.to_string())),
    }
}

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
        TypeExpression::Nullable(inner) => type_expression_to_hint(inner),
        _ => None,
    }
}

fn type_expression_to_string(expr: &TypeExpression) -> String {
    match expr {
        TypeExpression::Simple(s) => s.clone(),
        TypeExpression::Array(inner) => format!("{}[]", type_expression_to_string(inner)),
        TypeExpression::Generic { base, params } => {
            let params_str = params
                .iter()
                .map(type_expression_to_string)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", base, params_str)
        }
        TypeExpression::Union(types) => types
            .iter()
            .map(type_expression_to_string)
            .collect::<Vec<_>>()
            .join("|"),
        TypeExpression::Nullable(inner) => format!("?{}", type_expression_to_string(inner)),
        TypeExpression::Mixed => "mixed".to_string(),
        TypeExpression::Void => "void".to_string(),
        TypeExpression::Never => "never".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn parse_php(source: &str) -> parser::ParsedSource {
        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser.set_language(tree_sitter_php::language()).unwrap();
        let tree = ts_parser.parse(source, None).unwrap();

        parser::ParsedSource {
            path: std::path::PathBuf::from("test.php"),
            source: Arc::new(source.to_string()),
            tree,
        }
    }

    #[test]
    fn test_return_type_conflict() {
        let source = r#"<?php
/**
 * @return string
 */
function test(): int {
    return 42;
}
"#;

        let parsed = parse_php(source);
        let context = ProjectContext::new();

        let rule = PhpDocReturnCheckRule::new();
        let diagnostics = rule.run(&parsed, &context);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0]
            .message
            .contains("@return type 'string' conflicts with native return type hint 'int'"));
    }

    #[test]
    fn test_return_type_matches() {
        let source = r#"<?php
/**
 * @return int
 */
function test(): int {
    return 42;
}
"#;

        let parsed = parse_php(source);
        let context = ProjectContext::new();

        let rule = PhpDocReturnCheckRule::new();
        let diagnostics = rule.run(&parsed, &context);

        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_method_return_type_conflict() {
        let source = r#"<?php
class Test {
    /**
     * @return bool
     */
    public function check(): int {
        return 1;
    }
}
"#;

        let parsed = parse_php(source);
        let context = ProjectContext::new();

        let rule = PhpDocReturnCheckRule::new();
        let diagnostics = rule.run(&parsed, &context);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("@return type 'bool'"));
    }
}
