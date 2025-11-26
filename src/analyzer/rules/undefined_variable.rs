use super::DiagnosticRule;
use super::helpers::diagnostic_for_node;
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use std::collections::HashSet;
use tree_sitter::Node;

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

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut visitor = ScopeVisitor::new(parsed);
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct ScopeVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    scopes: Vec<HashSet<String>>,
    diagnostics: Vec<crate::analyzer::Diagnostic>,
}

impl<'a> ScopeVisitor<'a> {
    fn new(parsed: &'a parser::ParsedSource) -> Self {
        Self {
            parsed,
            scopes: vec![std::collections::HashSet::new()],
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
                if name == "this" {
                    return;
                }

                if matches!(
                    name.as_str(),
                    "_GET"
                        | "_POST"
                        | "_REQUEST"
                        | "_COOKIE"
                        | "_FILES"
                        | "_SERVER"
                        | "_ENV"
                        | "argc"
                        | "argv"
                ) {
                    self.define_variable(name);
                    return;
                }

                if let Some(parent) = node.parent() {
                    if parent.kind() == "property_promotion_parameter" {
                        self.define_variable(name);
                        return;
                    }
                }

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
        self.diagnostics.push(diagnostic_for_node(
            self.parsed,
            node,
            Severity::Error,
            format!(
                "undefined variable ${name} at {}:{}",
                node.start_position().row + 1,
                node.start_position().column + 1
            ),
        ));
    }
}
