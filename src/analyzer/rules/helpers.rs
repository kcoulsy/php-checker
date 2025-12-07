use crate::analyzer::parser;
use crate::analyzer::{Diagnostic, Severity, Span};
use std::collections::HashMap;
use tree_sitter::Node;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeHint {
    Int,
    String,
    Bool,
    Float,
    Object(String),          // Stores the class/interface name
    Nullable(Box<TypeHint>), // Wraps another type to make it nullable
    Union(Vec<TypeHint>),    // Union of multiple types (int|string)
    Array(Box<TypeHint>),    // Array of a specific type (int[], User[])
    GenericArray {           // Associative array with key/value types (array<string, int>)
        key: Box<TypeHint>,
        value: Box<TypeHint>,
    },
    ShapedArray(Vec<(String, TypeHint)>), // Shaped array with named fields (array{name: string, age: int})
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralKind {
    Integer,
    String,
}

pub struct FunctionSignature {
    pub params: Vec<TypeHint>,
}

pub fn diagnostic_for_node(
    parsed: &parser::ParsedSource,
    node: Node,
    severity: Severity,
    message: impl Into<String>,
) -> Diagnostic {
    let span = Span {
        start: node.start_position(),
        end: node.end_position(),
    };

    let snippet_before = span
        .start
        .row
        .checked_sub(1)
        .and_then(|row| line_at(parsed.source.as_str(), row));

    let snippet_line = line_at(parsed.source.as_str(), span.start.row);
    let snippet_after = line_at(parsed.source.as_str(), span.start.row + 1);
    let caret_col = Some(span.start.column);
    let caret_len = if span.start.row == span.end.row {
        span.end.column.saturating_sub(span.start.column).max(1)
    } else {
        1
    };

    Diagnostic::with_span(
        parsed.path.clone(),
        severity,
        message,
        span,
        snippet_before,
        snippet_line,
        snippet_after,
        caret_col,
        caret_len,
    )
}

pub fn diagnostic_for_span(
    parsed: &parser::ParsedSource,
    span: Span,
    severity: Severity,
    message: impl Into<String>,
) -> Diagnostic {
    let snippet_before = span
        .start
        .row
        .checked_sub(1)
        .and_then(|row| line_at(parsed.source.as_str(), row));

    let snippet_line = line_at(parsed.source.as_str(), span.start.row);
    let snippet_after = line_at(parsed.source.as_str(), span.start.row + 1);
    let caret_col = Some(span.start.column);
    let caret_len = if span.start.row == span.end.row {
        span.end.column.saturating_sub(span.start.column).max(1)
    } else {
        1
    };

    Diagnostic::with_span(
        parsed.path.clone(),
        severity,
        message,
        span,
        snippet_before,
        snippet_line,
        snippet_after,
        caret_col,
        caret_len,
    )
}

pub fn line_at(source: &str, row: usize) -> Option<String> {
    source.lines().nth(row).map(ToOwned::to_owned)
}

pub fn walk_node<'a, F>(node: Node<'a>, callback: &mut F)
where
    F: FnMut(Node<'a>),
{
    callback(node);
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_node(cursor.node(), callback);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

pub fn child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    for idx in 0..node.named_child_count() {
        if let Some(child) = node.named_child(idx) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }
    None
}

pub fn find_descendant_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    let mut stack = vec![node];
    while let Some(current) = stack.pop() {
        if current.kind() == kind {
            return Some(current);
        }
        let mut cursor = current.walk();
        if cursor.goto_first_child() {
            loop {
                stack.push(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
    None
}

pub fn node_text(node: Node, parsed: &parser::ParsedSource) -> Option<String> {
    node.utf8_text(parsed.source.as_bytes())
        .ok()
        .map(str::trim)
        .map(ToOwned::to_owned)
}

pub fn variable_name_text(node: Node, parsed: &parser::ParsedSource) -> Option<String> {
    node_text(node, parsed).map(|text| text.trim_start_matches('$').to_string())
}

pub fn has_conditional_ancestor(node: Node, boundary: Node) -> bool {
    let boundary_id = boundary.id();
    let mut current = node;
    while let Some(parent) = current.parent() {
        if parent.id() == boundary_id {
            break;
        }

        if matches!(
            parent.kind(),
            "if_statement"
                | "elseif_clause"
                | "else_clause"
                | "match_expression"
                | "switch_statement"
        ) {
            return true;
        }

        current = parent;
    }

    false
}

pub fn is_definition(node: Node) -> bool {
    if let Some(parent) = node.parent() {
        match parent.kind() {
            "assignment_expression" => parent.named_child(0).map_or(false, |left| left == node),
            "simple_parameter" | "variadic_parameter" | "property_promotion_parameter" => true,
            _ => false,
        }
    } else {
        false
    }
}

pub fn collect_function_signatures(
    parsed: &parser::ParsedSource,
) -> HashMap<String, FunctionSignature> {
    let mut signatures = HashMap::new();

    walk_node(parsed.tree.root_node(), &mut |node| {
        if node.kind() != "function_definition" {
            return;
        }

        let name_node = child_by_kind(node, "name");
        let Some(name_node) = name_node else {
            return;
        };

        let Some(name) = node_text(name_node, parsed) else {
            return;
        };

        let formal = child_by_kind(node, "formal_parameters");
        let params = if let Some(formal_params) = formal {
            (0..formal_params.named_child_count())
                .filter_map(|idx| formal_params.named_child(idx))
                .filter(|child: &Node| {
                    matches!(child.kind(), "simple_parameter" | "variadic_parameter")
                })
                .map(|param| type_hint_from_parameter(param, parsed))
                .collect()
        } else {
            Vec::new()
        };

        signatures.insert(name, FunctionSignature { params });
    });

    signatures
}

pub fn type_hint_from_parameter(param: Node, parsed: &parser::ParsedSource) -> TypeHint {
    // Check for optional_type (nullable with ?)
    if let Some(optional_type) = find_descendant_by_kind(param, "optional_type") {
        // Extract the inner type from the optional_type node
        for i in 0..optional_type.named_child_count() {
            if let Some(child) = optional_type.named_child(i) {
                // Recursively get the inner type hint
                let inner = type_hint_from_node(child, parsed);
                if inner != TypeHint::Unknown {
                    return TypeHint::Nullable(Box::new(inner));
                }
            }
        }
    }

    // Check for primitive types
    if let Some(primitive) = find_descendant_by_kind(param, "primitive_type") {
        if let Some(text) = node_text(primitive, parsed) {
            return match text.as_str() {
                "int" => TypeHint::Int,
                "string" => TypeHint::String,
                "bool" | "boolean" => TypeHint::Bool,
                "float" | "double" => TypeHint::Float,
                _ => TypeHint::Unknown,
            };
        }
    }

    // Check for named types (classes/interfaces)
    if let Some(named_type) = find_descendant_by_kind(param, "named_type") {
        if let Some(text) = node_text(named_type, parsed) {
            return TypeHint::Object(text);
        }
    }

    TypeHint::Unknown
}

// Helper function to get type hint from a node (primitive or named)
fn type_hint_from_node(node: Node, parsed: &parser::ParsedSource) -> TypeHint {
    match node.kind() {
        "primitive_type" => {
            if let Some(text) = node_text(node, parsed) {
                return match text.as_str() {
                    "int" => TypeHint::Int,
                    "string" => TypeHint::String,
                    "bool" | "boolean" => TypeHint::Bool,
                    "float" | "double" => TypeHint::Float,
                    _ => TypeHint::Unknown,
                };
            }
            TypeHint::Unknown
        }
        "named_type" => {
            if let Some(text) = node_text(node, parsed) {
                TypeHint::Object(text)
            } else {
                TypeHint::Unknown
            }
        }
        _ => TypeHint::Unknown,
    }
}

pub fn argument_literal_kind<'a>(arg: Node<'a>) -> Option<(LiteralKind, Node<'a>)> {
    for idx in 0..arg.named_child_count() {
        if let Some(child) = arg.named_child(idx) {
            if let Some(kind) = literal_kind(child) {
                return Some((kind, child));
            }
        }
    }

    let mut cursor = arg.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.is_named() {
                if let Some(kind) = literal_kind(child) {
                    return Some((kind, child));
                }
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    None
}

pub fn literal_type(node: Node) -> Option<TypeHint> {
    let result = match node.kind() {
        "string" | "encapsed_string" => Some(TypeHint::String),
        "integer" => Some(TypeHint::Int),
        "boolean" => Some(TypeHint::Bool),
        "float" => Some(TypeHint::Float),
        _ => None,
    };
    if result.is_none() {
    }
    result
}

/// Infer the type of a node, including variables with known assignments
/// Returns Some(TypeHint::Unknown) if the node is a variable but type cannot be determined
/// Returns None if the node is not a value expression
pub fn infer_type(node: Node, parsed: &parser::ParsedSource) -> Option<TypeHint> {
    // First try to get literal type
    if let Some(lit_type) = literal_type(node) {
        return Some(lit_type);
    }

    // Check for object creation expression (new User())
    if node.kind() == "object_creation_expression" {
        // Get the class name from the object creation
        if let Some(name_node) = child_by_kind(node, "name") {
            if let Some(class_name) = node_text(name_node, parsed) {
                return Some(TypeHint::Object(class_name));
            }
        }
        // Also check for qualified_name (namespaced classes)
        if let Some(name_node) = child_by_kind(node, "qualified_name") {
            if let Some(class_name) = node_text(name_node, parsed) {
                return Some(TypeHint::Object(class_name));
            }
        }
        return Some(TypeHint::Unknown);
    }

    // If it's a variable, try to infer from context
    if node.kind() == "variable_name" {
        // For now, we'll collect variable assignments in the same scope
        // and try to infer the type
        if let Some(var_name) = variable_name_text(node, parsed) {
            // Look backwards in the tree to find assignments to this variable
            if let Some(inferred) = infer_variable_type(&var_name, node, parsed) {
                return Some(inferred);
            }
        }
        // If we can't infer, return Unknown to signal we should warn
        return Some(TypeHint::Unknown);
    }

    None
}

/// Try to infer a variable's type by looking at @var declarations or assignments
fn infer_variable_type(
    var_name: &str,
    _context_node: Node,
    parsed: &parser::ParsedSource,
) -> Option<TypeHint> {
    use crate::analyzer::phpdoc::{extract_phpdoc_for_node, TypeExpression};

    let root = parsed.tree.root_node();
    let mut found_type = None;

    // First priority: Look for @var declarations
    walk_node(root, &mut |node| {
        if found_type.is_some() {
            return; // Already found
        }

        // Check for inline @var on expression_statement
        if node.kind() == "expression_statement" {
            if let Some(phpdoc) = extract_phpdoc_for_node(node, parsed) {
                if let Some(var_tag) = phpdoc.var_tag {
                    // Check if the @var is for our variable
                    if let Some(declared_name) = &var_tag.name {
                        if declared_name == var_name {
                            // Found a @var declaration for this variable
                            found_type = type_expression_to_hint(&var_tag.type_expr);
                        }
                    }
                }
            }
        }
    });

    // If we found a @var declaration, use it
    if found_type.is_some() {
        return found_type;
    }

    // Second priority: Infer from literal assignment
    walk_node(root, &mut |node| {
        if found_type.is_some() {
            return; // Already found
        }

        if node.kind() == "assignment_expression" {
            // Check if this assigns to our variable
            if let Some(left) = node.child_by_field_name("left") {
                if left.kind() == "variable_name" {
                    if let Some(name) = variable_name_text(left, parsed) {
                        if name == var_name {
                            // Found an assignment to our variable
                            if let Some(right) = node.child_by_field_name("right") {
                                if let Some(typ) = literal_type(right) {
                                    found_type = Some(typ);
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    found_type
}

/// Helper to convert TypeExpression to TypeHint (reused from phpdoc rules)
fn type_expression_to_hint(expr: &crate::analyzer::phpdoc::TypeExpression) -> Option<TypeHint> {
    use crate::analyzer::phpdoc::TypeExpression;

    match expr {
        TypeExpression::Simple(s) => match s.as_str() {
            "int" | "integer" => Some(TypeHint::Int),
            "string" => Some(TypeHint::String),
            "bool" | "boolean" => Some(TypeHint::Bool),
            "float" | "double" => Some(TypeHint::Float),
            _ => Some(TypeHint::Object(s.clone())),
        },
        TypeExpression::Nullable(inner) => {
            type_expression_to_hint(inner).map(|t| TypeHint::Nullable(Box::new(t)))
        }
        TypeExpression::Union(types) => {
            let hints: Vec<TypeHint> = types
                .iter()
                .filter_map(|t| type_expression_to_hint(t))
                .collect();
            if hints.is_empty() {
                None
            } else {
                Some(TypeHint::Union(hints))
            }
        }
        TypeExpression::Array(inner) => {
            type_expression_to_hint(inner).map(|t| TypeHint::Array(Box::new(t)))
        }
        TypeExpression::Generic { base, params } => {
            if base == "array" && params.len() == 2 {
                let key_hint = type_expression_to_hint(&params[0])?;
                let value_hint = type_expression_to_hint(&params[1])?;
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

fn literal_kind(node: Node) -> Option<LiteralKind> {
    match node.kind() {
        "string" | "encapsed_string" => Some(LiteralKind::String),
        "integer" => Some(LiteralKind::Integer),
        _ => None,
    }
}

pub fn newline_for_source(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Extract array elements from an array_creation_expression node
/// Returns a vector of (element_node, element_type) pairs
pub fn extract_array_elements<'a>(
    array_node: Node<'a>,
    parsed: &parser::ParsedSource,
) -> Vec<(Node<'a>, Option<TypeHint>)> {
    let mut elements = Vec::new();
    let mut cursor = array_node.walk();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "array_element_initializer" {
                // For simple arrays like [1, 2, 3], the value is a direct child
                // For associative arrays like ["key" => value], we need the value after =>
                let value_node = if let Some(pair_node) = child_by_kind(child, "pair") {
                    // Associative array - get the value (second element of pair)
                    pair_node.named_child(1)
                } else {
                    // Simple array - the element itself is the value
                    child.named_child(0)
                };

                if let Some(val_node) = value_node {
                    let elem_type = infer_type(val_node, parsed);
                    elements.push((val_node, elem_type));
                }
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    elements
}

/// Extract key-value pairs from an array_creation_expression node for generic array validation
/// Returns a vector of (key_node, key_type, value_node, value_type) tuples
pub fn extract_array_key_value_pairs<'a>(
    array_node: Node<'a>,
    parsed: &parser::ParsedSource,
) -> Vec<(Option<Node<'a>>, Option<TypeHint>, Node<'a>, Option<TypeHint>)> {
    let mut pairs = Vec::new();
    let mut cursor = array_node.walk();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "array_element_initializer" {
                // Check number of children to determine if it's a key-value pair or simple element
                if child.named_child_count() == 2 {
                    // Associative array ["key" => value]
                    // tree-sitter PHP represents this with 2 children directly (no "pair" wrapper)
                    let key_node = child.named_child(0);
                    let value_node = child.named_child(1);

                    if let (Some(k_node), Some(v_node)) = (key_node, value_node) {
                        let key_type = infer_type(k_node, parsed);
                        let value_type = infer_type(v_node, parsed);
                        pairs.push((Some(k_node), key_type, v_node, value_type));
                    }
                } else if child.named_child_count() == 1 {
                    // Simple array [value] - implicit integer keys
                    if let Some(val_node) = child.named_child(0) {
                        let value_type = infer_type(val_node, parsed);
                        // Implicit integer key
                        pairs.push((None, Some(TypeHint::Int), val_node, value_type));
                    }
                }
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    pairs
}

/// Check if actual_type is compatible with (a subset of) expected_type
/// Examples:
/// - int is compatible with int|string (subset)
/// - int is compatible with int (exact match)
/// - ?string is compatible with string|null (equivalent)
/// - string is compatible with ?string (subset)
pub fn is_type_compatible(actual: &TypeHint, expected: &TypeHint) -> bool {
    // Exact match
    if actual == expected {
        return true;
    }

    match expected {
        // If expected is a union, actual must be compatible with at least one member
        TypeHint::Union(expected_types) => {
            // Check if actual matches any of the union members
            for expected_member in expected_types {
                if is_type_compatible(actual, expected_member) {
                    return true;
                }
            }

            // If actual is also a union, all its members must be in expected union
            if let TypeHint::Union(actual_types) = actual {
                return actual_types.iter().all(|actual_member| {
                    expected_types.iter().any(|expected_member| {
                        is_type_compatible(actual_member, expected_member)
                    })
                });
            }

            false
        }

        // If expected is nullable, actual can be the inner type or null
        TypeHint::Nullable(expected_inner) => {
            // Check if actual matches the inner type
            if is_type_compatible(actual, expected_inner) {
                return true;
            }

            // Check if actual is also nullable with compatible inner type
            if let TypeHint::Nullable(actual_inner) = actual {
                return is_type_compatible(actual_inner, expected_inner);
            }

            // Nullable is equivalent to Union with null, so handle that case
            // But we don't have a Null type, so we can't check for it here
            false
        }

        // If expected is an array, actual must be an array with compatible element type
        TypeHint::Array(expected_elem) => {
            if let TypeHint::Array(actual_elem) = actual {
                return is_type_compatible(actual_elem, expected_elem);
            }
            false
        }

        // If expected is a generic array, actual must have compatible key/value types
        TypeHint::GenericArray {
            key: expected_key,
            value: expected_value,
        } => {
            if let TypeHint::GenericArray {
                key: actual_key,
                value: actual_value,
            } = actual
            {
                return is_type_compatible(actual_key, expected_key)
                    && is_type_compatible(actual_value, expected_value);
            }
            false
        }

        // If actual is a union but expected is not, check if all actual types match expected
        _ => {
            if let TypeHint::Union(actual_types) = actual {
                // All members of actual union must match the expected type
                // This is generally false unless expected is Unknown or very generic
                return actual_types.iter().all(|t| is_type_compatible(t, expected));
            }

            // If actual is nullable, unwrap and check inner type
            if let TypeHint::Nullable(actual_inner) = actual {
                // Nullable type is only compatible with non-nullable if they match exactly
                // which we already checked above, so this is false
                return false;
            }

            false
        }
    }
}

 
