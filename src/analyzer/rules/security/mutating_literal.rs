use super::DiagnosticRule;
use super::helpers::{
    child_by_kind, diagnostic_for_node, newline_for_source, node_text, walk_node,
};
use crate::analyzer::fix;
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use std::collections::BTreeMap;
use tree_sitter::Node;
const MUTATING_FUNCTIONS: &[&str] = &[
    "array_pop",
    "array_shift",
    "array_push",
    "sort",
    "rsort",
    "asort",
    "ksort",
];

pub struct MutatingLiteralRule;

impl MutatingLiteralRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for MutatingLiteralRule {
    fn name(&self) -> &str {
        "security/mutating_literal"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        collect_mutating_literal_infos(parsed)
            .into_iter()
            .map(|info| {
                diagnostic_for_node(
                    parsed,
                    info.literal,
                    Severity::Warning,
                    format!(
                        "{} modifies its argument in place; avoid passing literals",
                        info.function_name
                    ),
                )
            })
            .collect()
    }

    fn fix(&self, parsed: &parser::ParsedSource, _context: &ProjectContext) -> Vec<fix::TextEdit> {
        let source = parsed.source.as_str();
        let newline = newline_for_source(source);
        let infos = collect_mutating_literal_infos(parsed);
        if infos.is_empty() {
            return Vec::new();
        }

        let mut statements = BTreeMap::new();
        for info in infos {
            statements
                .entry(info.statement.start_byte())
                .or_insert_with(|| StatementFix {
                    statement: info.statement,
                    literals: Vec::new(),
                })
                .literals
                .push(info.literal);
        }

        let mut edits = Vec::new();
        let mut counter = 0;
        for (_, statement_fix) in statements.into_iter() {
            let statement_start = statement_fix.statement.start_byte();
            let statement_end = statement_fix.statement.end_byte();
            let statement_text = &source[statement_start..statement_end];

            let mut literals = statement_fix.literals;
            literals.sort_by_key(|node| node.start_byte());

            let mut assignment_text = String::new();
            let mut new_statement = String::new();
            let mut cursor = statement_start;

            for literal in literals {
                counter += 1;
                let placeholder = format!("$_php_checker_literal_{counter}");
                let literal_text = &source[literal.start_byte()..literal.end_byte()];
                assignment_text.push_str(&format!("{placeholder} = {literal_text};{newline}"));

                let rel_start = (cursor - statement_start) as usize;
                let rel_literal_start = (literal.start_byte() - statement_start) as usize;
                new_statement.push_str(&statement_text[rel_start..rel_literal_start]);
                new_statement.push_str(&placeholder);
                cursor = literal.end_byte();
            }

            new_statement.push_str(&statement_text[(cursor - statement_start) as usize..]);

            if !assignment_text.is_empty() {
                let replacement = format!("{assignment_text}{new_statement}");
                edits.push(fix::TextEdit::new(
                    statement_start,
                    statement_end,
                    replacement,
                ));
            }
        }

        edits
    }
}

struct StatementFix<'a> {
    statement: Node<'a>,
    literals: Vec<Node<'a>>,
}

struct MutatingLiteralInfo<'a> {
    literal: Node<'a>,
    statement: Node<'a>,
    function_name: String,
}

fn collect_mutating_literal_infos<'a>(
    parsed: &'a parser::ParsedSource,
) -> Vec<MutatingLiteralInfo<'a>> {
    let mut infos = Vec::new();

    walk_node(parsed.tree.root_node(), &mut |node| {
        if node.kind() != "function_call_expression" {
            return;
        }

        let name_node = match child_by_kind(node, "name") {
            Some(node) => node,
            None => return,
        };

        let name = match node_text(name_node, parsed) {
            Some(name) => name,
            None => return,
        };

        if !MUTATING_FUNCTIONS.contains(&name.as_str()) {
            return;
        }

        let statement = enclosing_expression_statement(node);

        let arguments = match child_by_kind(node, "arguments") {
            Some(arguments) => arguments,
            None => return,
        };

        for idx in 0..arguments.named_child_count() {
            if let Some(argument) = arguments.named_child(idx) {
                if let Some(array_literal) = child_by_kind(argument, "array_creation_expression") {
                    infos.push(MutatingLiteralInfo {
                        literal: array_literal,
                        statement,
                        function_name: name.clone(),
                    });
                }
            }
        }
    });

    infos
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
