use super::DiagnosticRule;
use super::helpers::{
    LiteralKind, TypeHint, argument_literal_kind, child_by_kind, collect_function_signatures,
    diagnostic_for_node, node_text, walk_node,
};
use crate::analyzer::{Severity, parser};

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

    fn run(&self, parsed: &parser::ParsedSource) -> Vec<crate::analyzer::Diagnostic> {
        let signatures = collect_function_signatures(parsed);
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_call_expression" {
                return;
            }

            let name_node = match child_by_kind(node, "name") {
                Some(name_node) => name_node,
                None => return,
            };

            let name = match node_text(name_node, parsed) {
                Some(name) => name,
                None => return,
            };

            let signature = match signatures.get(&name) {
                Some(signature) => signature,
                None => return,
            };

            let arguments = match child_by_kind(node, "arguments") {
                Some(arguments) => arguments,
                None => return,
            };

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
                        let start = literal_node.start_position();
                        let row = start.row + 1;
                        let column = start.column + 1;
                        diagnostics.push(diagnostic_for_node(
                            parsed,
                            literal_node,
                            Severity::Error,
                            format!(
                                "type mismatch: argument {} of {name} expects int but got string literal at {row}:{column}",
                                arg_index + 1
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
