use super::{Diagnostic, Severity, parser};
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

pub trait DiagnosticRule {
    fn name(&self) -> &str;
    fn run(&self, parsed: &parser::ParsedSource) -> Vec<Diagnostic>;
}

/// Tracks variable usage to report undefined names.
pub struct UndefinedVariableRule;

impl UndefinedVariableRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for UndefinedVariableRule {
    fn name(&self) -> &str {
        "undefined-variable"
    }

    fn run(&self, parsed: &parser::ParsedSource) -> Vec<Diagnostic> {
        let mut visitor = ScopeVisitor::new(parsed);
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct ScopeVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    scopes: Vec<HashSet<String>>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> ScopeVisitor<'a> {
    fn new(parsed: &'a parser::ParsedSource) -> Self {
        Self {
            parsed,
            scopes: vec![HashSet::new()],
            diagnostics: Vec::new(),
        }
    }

    fn visit(&mut self, node: Node) {
        if node.kind() == "function_definition" {
            self.enter_scope();
            self.visit_children(node);
            self.exit_scope();
            return;
        }

        if node.kind() == "variable_name" {
            if let Some(name) = self.variable_name_text(node) {
                if self.is_definition(node) {
                    self.define_variable(name);
                } else if !self.is_defined(&name) {
                    self.report_undefined(node, name);
                }
            }
        }

        self.visit_children(node);
    }

    fn visit_children(&mut self, node: Node) {
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.visit(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashSet::new());
    }

    fn exit_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_variable(&mut self, name: String) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name);
        }
    }

    fn is_defined(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| scope.contains(name))
    }

    fn variable_name_text(&self, node: Node) -> Option<String> {
        let source = self.parsed.source.as_str();
        node.utf8_text(source.as_bytes())
            .ok()
            .map(str::trim)
            .map(|text| text.trim_start_matches('$'))
            .filter(|name| !name.is_empty())
            .map(ToOwned::to_owned)
    }

    fn is_definition(&self, node: Node) -> bool {
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

    fn report_undefined(&mut self, node: Node, name: String) {
        let pos = node.start_position();
        self.diagnostics.push(Diagnostic::new(
            self.parsed.path.clone(),
            Severity::Error,
            format!(
                "undefined variable ${name} at {}:{}",
                pos.row + 1,
                pos.column + 1
            ),
        ));
    }
}

/// Detects functions that return on some paths but not others.
pub struct MissingReturnRule;

impl MissingReturnRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for MissingReturnRule {
    fn name(&self) -> &str {
        "missing-return"
    }

    fn run(&self, parsed: &parser::ParsedSource) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_definition" {
                return;
            }

            let body = child_by_kind(node, "compound_statement");
            let Some(body) = body else {
                return;
            };

            let mut return_nodes = Vec::new();
            walk_node(body, &mut |candidate| {
                if candidate.kind() == "return_statement" {
                    return_nodes.push(candidate);
                }
            });

            if return_nodes.is_empty() {
                return;
            }

            let has_unconditional = return_nodes
                .iter()
                .any(|r| !has_conditional_ancestor(*r, body));

            if has_unconditional {
                return;
            }

            let name_node = node.child_by_field_name("name").unwrap_or(node);
            let name = node_text(name_node, parsed).unwrap_or_else(|| "anonymous".into());
            let pos = name_node.start_position();

            diagnostics.push(Diagnostic::new(
                parsed.path.clone(),
                Severity::Error,
                format!(
                    "function {name} is missing a return on some paths at {}:{}",
                    pos.row + 1,
                    pos.column + 1
                ),
            ));
        });

        diagnostics
    }
}

/// Ensures literal arguments are compatible with declared parameter types.
pub struct TypeMismatchRule;

impl TypeMismatchRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for TypeMismatchRule {
    fn name(&self) -> &str {
        "type-mismatch"
    }

    fn run(&self, parsed: &parser::ParsedSource) -> Vec<Diagnostic> {
        let signatures = collect_function_signatures(parsed);
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_call_expression" {
                return;
            }

            let name_node = child_by_kind(node, "name");
            let Some(name_node) = name_node else {
                return;
            };

            let Some(name) = node_text(name_node, parsed) else {
                return;
            };

            let signature = signatures.get(&name);
            if signature.is_none() {
                return;
            }
            let signature = signature.unwrap();

            let arguments = child_by_kind(node, "arguments");
            if arguments.is_none() {
                return;
            }
            let arguments = arguments.unwrap();

            let mut arg_index = 0;
            for idx in 0..arguments.named_child_count() {
                let Some(argument_node) = arguments.named_child(idx) else {
                    continue;
                };

                if argument_node.kind() != "argument" {
                    continue;
                }

                if arg_index >= signature.params.len() {
                    break;
                }

                if let Some((literal, literal_node)) = argument_literal_kind(argument_node) {
                    let expected = signature.params[arg_index];
                    if expected == TypeHint::Int && literal == LiteralKind::String {
                        let pos = literal_node.start_position();
                        diagnostics.push(Diagnostic::new(
                            parsed.path.clone(),
                            Severity::Error,
                            format!(
                                "type mismatch: argument {} of {name} expects int but got string literal at {}:{}",
                                arg_index + 1,
                                pos.row + 1,
                                pos.column + 1
                            ),
                        ));
                    }
                }

                arg_index += 1;
            }
        });

        diagnostics
    }
}

/// Reports statements that follow an unconditional return.
pub struct UnreachableCodeRule;

impl UnreachableCodeRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for UnreachableCodeRule {
    fn name(&self) -> &str {
        "unreachable-code"
    }

    fn run(&self, parsed: &parser::ParsedSource) -> Vec<Diagnostic> {
        let mut visitor = UnreachableVisitor::new(parsed);
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct UnreachableVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> UnreachableVisitor<'a> {
    fn new(parsed: &'a parser::ParsedSource) -> Self {
        Self {
            parsed,
            diagnostics: Vec::new(),
        }
    }

    fn visit(&mut self, node: Node) {
        if node.kind() == "compound_statement" {
            self.inspect_compound(node);
        }

        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.visit(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn inspect_compound(&mut self, compound: Node) {
        let mut reachable = true;
        let mut cursor = compound.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.is_named() {
                    if !reachable {
                        let pos = child.start_position();
                        self.diagnostics.push(Diagnostic::new(
                            self.parsed.path.clone(),
                            Severity::Warning,
                            format!(
                                "unreachable code after return at {}:{}",
                                pos.row + 1,
                                pos.column + 1
                            ),
                        ));
                    }

                    if child.kind() == "return_statement" || child.kind() == "throw_statement" {
                        reachable = false;
                    }
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeHint {
    Int,
    String,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LiteralKind {
    Integer,
    String,
}

struct FunctionSignature {
    params: Vec<TypeHint>,
}

fn collect_function_signatures(
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

fn type_hint_from_parameter(param: Node, parsed: &parser::ParsedSource) -> TypeHint {
    if let Some(primitive) = find_descendant_by_kind(param, "primitive_type") {
        if let Some(text) = node_text(primitive, parsed) {
            return match text.as_str() {
                "int" => TypeHint::Int,
                "string" => TypeHint::String,
                _ => TypeHint::Unknown,
            };
        }
    }

    TypeHint::Unknown
}

fn argument_literal_kind(arg: Node) -> Option<(LiteralKind, Node)> {
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

fn literal_kind(node: Node) -> Option<LiteralKind> {
    match node.kind() {
        "string" | "encapsed_string" => Some(LiteralKind::String),
        "integer" => Some(LiteralKind::Integer),
        _ => None,
    }
}

fn child_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    for idx in 0..node.named_child_count() {
        if let Some(child) = node.named_child(idx) {
            if child.kind() == kind {
                return Some(child);
            }
        }
    }
    None
}

fn find_descendant_by_kind<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
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

fn node_text(node: Node, parsed: &parser::ParsedSource) -> Option<String> {
    node.utf8_text(parsed.source.as_bytes())
        .ok()
        .map(ToOwned::to_owned)
}

fn has_conditional_ancestor(node: Node, boundary: Node) -> bool {
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

fn walk_node<'a, F>(node: Node<'a>, callback: &mut F)
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
