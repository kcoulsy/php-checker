use crate::analyzer::parser;
use crate::analyzer::{Diagnostic, Severity, Span};
use std::collections::HashMap;
use tree_sitter::Node;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeHint {
    Int,
    String,
    Bool,
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
            "simple_parameter" | "variadic_parameter" => true,
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
    if let Some(primitive) = find_descendant_by_kind(param, "primitive_type") {
        if let Some(text) = node_text(primitive, parsed) {
            return match text.as_str() {
                "int" => TypeHint::Int,
                "string" => TypeHint::String,
                "bool" | "boolean" => TypeHint::Bool,
                _ => TypeHint::Unknown,
            };
        }
    }

    TypeHint::Unknown
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
    match node.kind() {
        "string" | "encapsed_string" => Some(TypeHint::String),
        "integer" => Some(TypeHint::Int),
        "boolean" => Some(TypeHint::Bool),
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
