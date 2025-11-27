use super::helpers::{
    TypeHint, child_by_kind, diagnostic_for_node, is_type_compatible, node_text, walk_node,
};
use crate::analyzer::phpdoc::{TypeExpression, extract_phpdoc_for_node};
use crate::analyzer::rules::DiagnosticRule;
use crate::analyzer::{Diagnostic, Severity, parser, project::ProjectContext};

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

    fn run(&self, parsed: &parser::ParsedSource, _context: &ProjectContext) -> Vec<Diagnostic> {
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

            // Parse the native type hint into a TypeHint
            let native_hint = parse_native_type_hint(native_type_node, parsed);
            let Some(native_hint) = native_hint else {
                return;
            };

            // Convert PHPDoc type to TypeHint
            let phpdoc_hint = type_expression_to_hint(&return_tag.type_expr);

            // Check for conflicts
            if let Some(_phpdoc) = phpdoc_hint {
                if !is_compatible_return(&native_hint, &return_tag.type_expr) {
                    let native_type_display = type_hint_to_string(&native_hint);

                    let message = format!(
                        "@return type '{}' conflicts with native return type hint '{}'",
                        type_expression_to_string(&return_tag.type_expr),
                        native_type_display
                    );

                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        native_type_node,
                        Severity::Error,
                        message,
                    ));
                }
            }
        });

        diagnostics
    }
}

/// Check if PHPDoc type is compatible with native type hint
/// PHPDoc can be more specific than native type (e.g., array<int, string> vs array)
fn is_compatible_return(native: &TypeHint, phpdoc_expr: &TypeExpression) -> bool {
    // If we have a generic/array PHPDoc type and native is just "array", that's compatible
    // (PHPDoc is being more specific)
    if matches!(native, TypeHint::Object(name) if name == "array") {
        match phpdoc_expr {
            // array<...> is compatible with array
            TypeExpression::Generic { base, .. } if base == "array" => return true,
            // Type[] is compatible with array
            TypeExpression::Array(_) => return true,
            // array (simple) is compatible with array
            TypeExpression::Simple(s) if s == "array" => return true,
            _ => {}
        }
    }

    // For other cases, convert PHPDoc to TypeHint and use compatibility checking
    if let Some(phpdoc_hint) = type_expression_to_hint(phpdoc_expr) {
        // Check bidirectional compatibility for @return
        // Either they should match exactly or be compatible in some direction
        return is_type_compatible(native, &phpdoc_hint)
            || is_type_compatible(&phpdoc_hint, native);
    }

    false
}

/// Parse a native PHP type hint node into a TypeHint
fn parse_native_type_hint(
    type_node: tree_sitter::Node,
    parsed: &parser::ParsedSource,
) -> Option<TypeHint> {
    match type_node.kind() {
        "return_type" => {
            // return_type may contain optional_type, primitive_type, named_type, union_type, etc.
            if let Some(optional_node) = child_by_kind(type_node, "optional_type") {
                // Nullable type (? prefix)
                return parse_native_type_hint(optional_node, parsed)
                    .map(|t| TypeHint::Nullable(Box::new(t)));
            }

            if let Some(union_node) = child_by_kind(type_node, "union_type") {
                return parse_native_type_hint(union_node, parsed);
            }

            if let Some(primitive) = child_by_kind(type_node, "primitive_type") {
                return node_text(primitive, parsed).and_then(|text| text_to_type_hint(&text));
            }

            if let Some(named) = child_by_kind(type_node, "named_type") {
                return node_text(named, parsed).and_then(|text| text_to_type_hint(&text));
            }

            None
        }
        "union_type" => {
            // Parse union type: collect all types separated by |
            let mut types = Vec::new();
            for i in 0..type_node.named_child_count() {
                if let Some(child) = type_node.named_child(i) {
                    if let Some(hint) = parse_native_type_hint(child, parsed) {
                        types.push(hint);
                    }
                }
            }
            if types.is_empty() {
                None
            } else {
                Some(TypeHint::Union(types))
            }
        }
        "optional_type" => {
            // Nullable with ? prefix - get the inner type
            for i in 0..type_node.named_child_count() {
                if let Some(child) = type_node.named_child(i) {
                    if let Some(hint) = parse_native_type_hint(child, parsed) {
                        return Some(TypeHint::Nullable(Box::new(hint)));
                    }
                }
            }
            None
        }
        "primitive_type" => {
            node_text(type_node, parsed).and_then(|text| text_to_type_hint(&text))
        }
        "named_type" => {
            node_text(type_node, parsed).and_then(|text| text_to_type_hint(&text))
        }
        _ => None,
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

fn type_hint_to_string(hint: &TypeHint) -> String {
    match hint {
        TypeHint::Int => "int".to_string(),
        TypeHint::String => "string".to_string(),
        TypeHint::Bool => "bool".to_string(),
        TypeHint::Float => "float".to_string(),
        TypeHint::Object(name) => name.clone(),
        TypeHint::Nullable(inner) => format!("?{}", type_hint_to_string(inner)),
        TypeHint::Union(types) => types
            .iter()
            .map(type_hint_to_string)
            .collect::<Vec<_>>()
            .join("|"),
        TypeHint::Unknown => "unknown".to_string(),
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
        TypeExpression::Nullable(inner) => {
            // Wrap the inner type in Nullable
            type_expression_to_hint(inner).map(|t| TypeHint::Nullable(Box::new(t)))
        }
        TypeExpression::Union(types) => {
            // Convert each type in the union
            let hints: Vec<TypeHint> = types.iter().filter_map(type_expression_to_hint).collect();
            if hints.is_empty() {
                None
            } else {
                Some(TypeHint::Union(hints))
            }
        }
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
        assert!(
            diagnostics[0]
                .message
                .contains("@return type 'string' conflicts with native return type hint 'int'")
        );
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
