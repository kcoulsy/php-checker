use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, is_definition, variable_name_text};
use crate::analyzer::{Severity, parser};
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

pub struct UnusedVariableRule;

impl UnusedVariableRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for UnusedVariableRule {
    fn name(&self) -> &str {
        "unused-variable"
    }

    fn run(&self, parsed: &parser::ParsedSource) -> Vec<crate::analyzer::Diagnostic> {
        let mut visitor = UnusedVariableVisitor::new(parsed);
        visitor.visit(parsed.tree.root_node());
        visitor.emit()
    }
}

struct UnusedVariableVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    defined: HashMap<String, Node<'a>>,
    used: HashSet<String>,
}

impl<'a> UnusedVariableVisitor<'a> {
    fn new(parsed: &'a parser::ParsedSource) -> Self {
        Self {
            parsed,
            defined: HashMap::new(),
            used: HashSet::new(),
        }
    }

    fn visit(&mut self, node: Node<'a>) {
        if node.kind() == "variable_name" {
            if let Some(name) = variable_name_text(node, self.parsed) {
                let is_definition = is_definition(node);
                if is_definition {
                    if !is_parameter_definition(node) {
                        self.defined.entry(name).or_insert(node);
                    }
                } else {
                    self.used.insert(name);
                }
            }
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

    fn emit(self) -> Vec<crate::analyzer::Diagnostic> {
        self.defined
            .into_iter()
            .filter_map(|(name, node)| {
                if self.used.contains(&name) {
                    None
                } else {
                    Some(diagnostic_for_node(
                        self.parsed,
                        node,
                        Severity::Error,
                        format!("unused variable ${name}"),
                    ))
                }
            })
            .collect()
    }
}

fn is_parameter_definition(node: Node) -> bool {
    node.parent()
        .map(|parent| matches!(parent.kind(), "simple_parameter" | "variadic_parameter"))
        .unwrap_or(false)
}
