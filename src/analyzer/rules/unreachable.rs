use super::DiagnosticRule;
use super::helpers::diagnostic_for_node;
use crate::analyzer::{Severity, parser};
use tree_sitter::Node;

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

    fn run(&self, parsed: &parser::ParsedSource) -> Vec<crate::analyzer::Diagnostic> {
        let mut visitor = UnreachableVisitor::new(parsed);
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct UnreachableVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    diagnostics: Vec<crate::analyzer::Diagnostic>,
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
                        self.diagnostics.push(diagnostic_for_node(
                            self.parsed,
                            child,
                            Severity::Warning,
                            "unreachable code after return",
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
