use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, is_definition, variable_name_text};
use crate::analyzer::fix;
use crate::analyzer::project::ProjectContext;
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
        "cleanup/unused_variable"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        collect_unused_variables(parsed)
            .into_iter()
            .map(|unused| {
                diagnostic_for_node(
                    parsed,
                    unused.definition.node,
                    Severity::Error,
                    format!("unused variable ${}", unused.name),
                )
            })
            .collect()
    }

    fn fix(&self, parsed: &parser::ParsedSource, _context: &ProjectContext) -> Vec<fix::TextEdit> {
        let source = parsed.source.as_str();
        collect_unused_variables(parsed)
            .into_iter()
            .map(|unused| {
                let (start, end) = fix::covering_line_range(
                    source,
                    unused.definition.statement.start_byte(),
                    unused.definition.statement.end_byte(),
                );
                fix::TextEdit::new(start, end, "")
            })
            .collect()
    }
}

fn collect_unused_variables<'a>(parsed: &'a parser::ParsedSource) -> Vec<UnusedVariable<'a>> {
    let mut visitor = UnusedVariableVisitor::new(parsed);
    visitor.visit(parsed.tree.root_node());
    visitor.collect_unused()
}

struct UnusedVariable<'a> {
    name: String,
    definition: VariableDefinition<'a>,
}

struct VariableDefinition<'a> {
    node: Node<'a>,
    statement: Node<'a>,
}

struct UnusedVariableVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    defined: HashMap<String, VariableDefinition<'a>>,
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
                        self.define_variable(name, node);
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

    fn collect_unused(self) -> Vec<UnusedVariable<'a>> {
        let UnusedVariableVisitor { defined, used, .. } = self;
        defined
            .into_iter()
            .filter(|(name, _)| !used.contains(name) && !name.starts_with('_'))
            .map(|(name, definition)| UnusedVariable { name, definition })
            .collect()
    }

    fn define_variable(&mut self, name: String, node: Node<'a>) {
        let statement = enclosing_expression_statement(node);
        self.defined
            .entry(name)
            .or_insert(VariableDefinition { node, statement });
    }
}

fn enclosing_expression_statement(mut node: Node) -> Node {
    while let Some(parent) = node.parent() {
        if parent.kind() == "expression_statement" {
            return parent;
        }
        node = parent;
    }

    node
}

fn is_parameter_definition(node: Node) -> bool {
    node.parent()
        .map(|parent| {
            matches!(
                parent.kind(),
                "simple_parameter" | "variadic_parameter" | "property_promotion_parameter"
            )
        })
        .unwrap_or(false)
}
