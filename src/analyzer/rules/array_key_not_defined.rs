use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, variable_name_text};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

pub struct ArrayKeyNotDefinedRule;

impl ArrayKeyNotDefinedRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for ArrayKeyNotDefinedRule {
    fn name(&self) -> &str {
        "array-key-not-defined"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut visitor = ArrayKeyVisitor::new(parsed);
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct ArrayKeyVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    definitions: HashMap<String, HashSet<String>>,
    diagnostics: Vec<crate::analyzer::Diagnostic>,
}

impl<'a> ArrayKeyVisitor<'a> {
    fn new(parsed: &'a parser::ParsedSource) -> Self {
        Self {
            parsed,
            definitions: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn visit(&mut self, node: Node<'a>) {
        match node.kind() {
            "assignment_expression" => self.handle_assignment(node),
            "subscript_expression" => self.handle_subscript(node),
            _ => {}
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

    fn handle_assignment(&mut self, node: Node<'a>) {
        let Some(variable_node) = child_by_kind(node, "variable_name") else {
            return;
        };

        let Some(name) = variable_name_text(variable_node, self.parsed) else {
            return;
        };

        if let Some(array_node) = child_by_kind(node, "array_creation_expression") {
            let keys = collect_array_keys(array_node, self.parsed);
            self.definitions.insert(name, keys);
        } else {
            self.definitions.remove(&name);
        }
    }

    fn handle_subscript(&mut self, node: Node<'a>) {
        let mut variable_name = None;
        let mut literal_value = None;
        let mut literal_node = None;

        for idx in 0..node.named_child_count() {
            let child = match node.named_child(idx) {
                Some(child) => child,
                None => continue,
            };

            match child.kind() {
                "variable_name" => {
                    if variable_name.is_none() {
                        variable_name = variable_name_text(child, self.parsed);
                    }
                }
                "string" | "encapsed_string" => {
                    if literal_value.is_none() {
                        if let Some(text) = literal_string_value(child, self.parsed) {
                            literal_value = Some(text);
                            literal_node = Some(child);
                        }
                    }
                }
                _ => {}
            }
        }

        if let (Some(name), Some(value), Some(node)) = (variable_name, literal_value, literal_node)
        {
            if let Some(defined_keys) = self.definitions.get(&name) {
                if !defined_keys.contains(&value) {
                    let start = node.start_position();
                    let row = start.row + 1;
                    let column = start.column + 1;
                    self.diagnostics.push(diagnostic_for_node(
                        self.parsed,
                        node,
                        Severity::Error,
                        format!("undefined array key '{value}' at {row}:{column}"),
                    ));
                }
            }
        }
    }
}

fn collect_array_keys<'a>(node: Node<'a>, parsed: &'a parser::ParsedSource) -> HashSet<String> {
    let mut keys = HashSet::new();
    let mut cursor = node.walk();

    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            if child.kind() == "array_element_initializer" {
                if let Some(key) = extract_element_key(child, parsed) {
                    keys.insert(key);
                }
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    keys
}

fn extract_element_key<'a>(node: Node<'a>, parsed: &'a parser::ParsedSource) -> Option<String> {
    for idx in 0..node.named_child_count() {
        if let Some(child) = node.named_child(idx) {
            match child.kind() {
                "string" | "encapsed_string" => return literal_string_value(child, parsed),
                _ => {}
            }
        }
    }

    None
}

fn literal_string_value(node: Node, parsed: &parser::ParsedSource) -> Option<String> {
    node_text(node, parsed).map(|text| text.trim_matches(|c| c == '\'' || c == '"').to_string())
}
