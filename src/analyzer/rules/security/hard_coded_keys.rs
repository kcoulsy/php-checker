use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, node_text, walk_node};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

const KEY_INDICATORS: &[&str] = &[
    "key",
    "secret",
    "token",
    "api_key",
    "apikey",
    "encryption",
    "cipher",
];
const ENCRYPTION_FUNCTIONS: &[&str] = &[
    "openssl_encrypt",
    "openssl_decrypt",
    "crypt",
    "hash_hmac",
    "password_hash",
];

pub struct HardCodedKeysRule;

impl HardCodedKeysRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for HardCodedKeysRule {
    fn name(&self) -> &str {
        "security/hard_coded_keys"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();

        walk_node(parsed.tree.root_node(), &mut |node| {
            if node.kind() != "string" && node.kind() != "encapsed_string" {
                return;
            }

            if let Some(text) = node_text(node, parsed) {
                // Skip obviously non-keys (too short, contains spaces, etc.)
                if text.len() < 8 || text.contains(' ') || text.contains('\n') {
                    return;
                }

                // Look for patterns that suggest encryption keys
                if is_potential_key(&text) {
                    diagnostics.push(diagnostic_for_node(
                        parsed,
                        node,
                        Severity::Error,
                        "potential hard-coded encryption key detected, consider using environment variables or secure key management",
                    ));
                }
            }
        });

        diagnostics
    }
}

fn is_potential_key(text: &str) -> bool {
    // Check for common key patterns:
    // - Hexadecimal strings (common for keys)
    // - Base64-like strings
    // - Long alphanumeric strings
    // - Strings containing key-related keywords

    let text_lower = text.to_lowercase();

    // Check for key indicator words
    if KEY_INDICATORS
        .iter()
        .any(|indicator| text_lower.contains(indicator))
    {
        return true;
    }

    // Check for hexadecimal patterns (common in keys)
    if text.len() >= 16 && text.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }

    // Check for base64-like patterns
    if text.len() >= 16
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
    {
        // Additional check for base64 padding
        if text.ends_with('=') || (text.len() % 4 == 0) {
            return true;
        }
    }

    // Check for long random-looking strings
    if text.len() >= 20 && text.chars().all(|c| c.is_ascii_alphanumeric()) {
        // Count different character types to detect randomness
        let has_lower = text.chars().any(|c| c.is_ascii_lowercase());
        let has_upper = text.chars().any(|c| c.is_ascii_uppercase());
        let has_digit = text.chars().any(|c| c.is_ascii_digit());

        // If it has mixed case and digits, likely a key
        if has_lower && has_upper && has_digit {
            return true;
        }
    }

    false
}

fn is_used_as_key(string_node: tree_sitter::Node, parsed: &parser::ParsedSource) -> bool {
    // Walk up the AST to find if this string is used as a key parameter
    let mut current = string_node;

    while let Some(parent) = current.parent() {
        match parent.kind() {
            "argument" => {
                // Check if this argument is in a key position for encryption functions
                if let Some(grandparent) = parent.parent() {
                    if grandparent.kind() == "arguments" {
                        if let Some(function_call) = grandparent.parent() {
                            if function_call.kind() == "function_call_expression" {
                                if is_encryption_function_call(&function_call, parsed) {
                                    // Check argument position (keys are typically 2nd or 3rd argument)
                                    let arg_index = get_argument_index(&parent, &grandparent);
                                    if arg_index == 1 || arg_index == 2 {
                                        // 0-indexed
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            "assignment_expression" => {
                // Check if assigned to a key-related variable
                if let Some(left) = parent.child_by_field_name("left") {
                    if let Some(var_name) = extract_variable_name(left, parsed) {
                        let lowered = var_name.to_lowercase();
                        if KEY_INDICATORS
                            .iter()
                            .any(|indicator| lowered.contains(indicator))
                        {
                            return true;
                        }
                    }
                }
            }
            "variable_declaration" => {
                // Check if declared as a key-related variable
                if let Some(var_name) = extract_variable_name_from_declaration(parent, parsed) {
                    let lowered = var_name.to_lowercase();
                    if KEY_INDICATORS
                        .iter()
                        .any(|indicator| lowered.contains(indicator))
                    {
                        return true;
                    }
                }
            }
            _ => {}
        }

        current = parent;

        // Don't walk too far up
        if current.kind() == "function_definition" || current.kind() == "method_declaration" {
            break;
        }
    }

    false
}

fn is_encryption_function_call(
    function_call: &tree_sitter::Node,
    parsed: &parser::ParsedSource,
) -> bool {
    use super::helpers::child_by_kind;

    if let Some(name_node) = child_by_kind(*function_call, "name") {
        if let Some(function_name) = node_text(name_node, parsed) {
            return ENCRYPTION_FUNCTIONS.contains(&function_name.as_str());
        }
    }
    false
}

fn get_argument_index(argument: &tree_sitter::Node, arguments: &tree_sitter::Node) -> usize {
    for idx in 0..arguments.named_child_count() {
        if let Some(child) = arguments.named_child(idx) {
            if child == *argument {
                return idx;
            }
        }
    }
    0
}

fn extract_variable_name(node: tree_sitter::Node, parsed: &parser::ParsedSource) -> Option<String> {
    match node.kind() {
        "variable_name" => node_text(node, parsed),
        "member_access_expression" => {
            if let Some(member) = node.child_by_field_name("name") {
                node_text(member, parsed)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_variable_name_from_declaration(
    node: tree_sitter::Node,
    parsed: &parser::ParsedSource,
) -> Option<String> {
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
