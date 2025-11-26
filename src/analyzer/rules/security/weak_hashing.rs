use super::DiagnosticRule;
use super::helpers::{child_by_kind, diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

const WEAK_HASH_FUNCTIONS: &[&str] = &["md5", "sha1"];
const PASSWORD_INDICATORS: &[&str] = &["password", "passwd", "pwd", "userpassword", "hashedpassword"];

pub struct WeakHashingRule;

impl WeakHashingRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for WeakHashingRule {
    fn name(&self) -> &str {
        "security/weak_hashing"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "function_call_expression" {
                return;
            }

            let name_node = match child_by_kind(node, "name") {
                Some(name_node) => name_node,
                None => return,
            };

            let function_name = match node_text(name_node, parsed) {
                Some(name) => name,
                None => return,
            };

            // Check if this is a weak hash function
            if !WEAK_HASH_FUNCTIONS.contains(&function_name.as_str()) {
                return;
            }

            // Check if this is used in a password-related context
            if is_password_context(node, parsed) {
                diagnostics.push(diagnostic_for_node(
                    parsed,
                    name_node,
                    Severity::Warning,
                    format!("weak hashing function '{}' used for password hashing, consider using password_hash() or similar secure alternatives", function_name),
                ));
            }
        });

        diagnostics
    }
}

fn is_password_context(function_call: tree_sitter::Node, parsed: &parser::ParsedSource) -> bool {
    // Check if the function call is assigned to a password-related variable
    if let Some(parent) = function_call.parent() {
        match parent.kind() {
            "assignment_expression" => {
                if let Some(left) = parent.child_by_field_name("left") {
                    if let Some(var_name) = extract_variable_name(left, parsed) {
                        let lowered = var_name.to_lowercase();
                        if PASSWORD_INDICATORS.iter().any(|indicator| lowered.contains(indicator)) {
                            return true;
                        }
                    }
                }
            }
            "variable_declaration" => {
                if let Some(var_name) = extract_variable_name_from_declaration(parent, parsed) {
                    let lowered = var_name.to_lowercase();
                    if PASSWORD_INDICATORS.iter().any(|indicator| lowered.contains(indicator)) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }

    // Check function arguments for password-related content
    if let Some(arguments) = child_by_kind(function_call, "arguments") {
        for idx in 0..arguments.named_child_count() {
            if let Some(arg) = arguments.named_child(idx) {
                if is_password_argument(arg, parsed) {
                    return true;
                }
            }
        }
    }

    false
}

fn extract_variable_name(node: tree_sitter::Node, parsed: &parser::ParsedSource) -> Option<String> {
    match node.kind() {
        "variable_name" => node_text(node, parsed),
        "member_access_expression" => {
            // For $user->password, etc.
            if let Some(member) = node.child_by_field_name("name") {
                node_text(member, parsed)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_variable_name_from_declaration(node: tree_sitter::Node, parsed: &parser::ParsedSource) -> Option<String> {
    // For declarations like $password = md5(...)
    for idx in 0..node.named_child_count() {
        if let Some(child) = node.named_child(idx) {
            if child.kind() == "simple_parameter" || child.kind() == "property_element" {
                if let Some(var_name) = child.child_by_field_name("name") {
                    return node_text(var_name, parsed);
                }
            }
        }
    }
    None
}

fn is_password_argument(node: tree_sitter::Node, parsed: &parser::ParsedSource) -> bool {
    // Check if argument contains password-related strings
    walk_node(node, &mut |child| {
        if child.kind() == "string" {
            if let Some(text) = node_text(child, parsed) {
                let lowered = text.to_lowercase();
                if PASSWORD_INDICATORS.iter().any(|indicator| lowered.contains(indicator)) {
                    return;
                }
            }
        }
        // Could also check variable names in arguments
        if child.kind() == "variable_name" {
            if let Some(var_name) = node_text(child, parsed) {
                let lowered = var_name.to_lowercase();
                if PASSWORD_INDICATORS.iter().any(|indicator| lowered.contains(indicator)) {
                    return;
                }
            }
        }
    });
    false
}
