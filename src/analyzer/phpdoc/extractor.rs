use super::parser::{PhpDocParser, PhpDocComment};
use crate::analyzer::parser::ParsedSource;
use tree_sitter::Node;

/// Extract PHPDoc comment that precedes a node
pub fn extract_phpdoc_for_node<'a>(node: Node<'a>, parsed: &'a ParsedSource) -> Option<PhpDocComment> {
    // Look for a comment node immediately before this node
    let parent = node.parent()?;
    let node_index = (0..parent.named_child_count())
        .find(|&i| parent.named_child(i).map(|n| n.id()) == Some(node.id()))?;

    // Check the previous sibling
    if node_index > 0 {
        if let Some(prev) = parent.named_child(node_index - 1) {
            if prev.kind() == "comment" {
                let comment_text = prev.utf8_text(parsed.source.as_bytes()).ok()?;
                return PhpDocParser::parse(comment_text);
            }
        }
    }

    // Also check all children of parent for a comment node before our node
    for i in 0..parent.child_count() {
        if let Some(child) = parent.child(i) {
            // If we found our target node, stop
            if child.id() == node.id() {
                break;
            }
            // Check if this is a comment
            if child.kind() == "comment" {
                let comment_text = child.utf8_text(parsed.source.as_bytes()).ok()?;
                if let Some(parsed_doc) = PhpDocParser::parse(comment_text) {
                    // Return the last comment found before our node
                    return Some(parsed_doc);
                }
            }
        }
    }

    None
}

/// Find the comment node immediately preceding a given node
pub fn find_preceding_comment<'a>(node: Node<'a>) -> Option<Node<'a>> {
    // Check parent's children for a comment before this node
    let parent = node.parent()?;

    for i in 0..parent.child_count() {
        if let Some(child) = parent.child(i) {
            // If we found our target node, stop
            if child.id() == node.id() {
                break;
            }
            // Save the last comment we find
            if child.kind() == "comment" {
                return Some(child);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::parser::TreeSitterPhpParser;
    use crate::analyzer::parser::PhpParser;
    use std::sync::Arc;

    #[test]
    fn test_extract_phpdoc_from_function() {
        let php_code = r#"<?php
/**
 * @param int $value
 * @return string
 */
function test($value) {
    return "test";
}
"#;

        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser.set_language(tree_sitter_php::language()).unwrap();
        let tree = ts_parser.parse(php_code, None).unwrap();

        let parsed = crate::analyzer::parser::ParsedSource {
            path: std::path::PathBuf::from("test.php"),
            source: Arc::new(php_code.to_string()),
            tree,
        };

        // Find the function_definition node
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();

        let mut function_node = None;
        for i in 0..root.named_child_count() {
            if let Some(child) = root.named_child(i) {
                if child.kind() == "function_definition" {
                    function_node = Some(child);
                    break;
                }
            }
        }

        let function_node = function_node.expect("Should find function_definition");
        let phpdoc = extract_phpdoc_for_node(function_node, &parsed);

        assert!(phpdoc.is_some());
        let doc = phpdoc.unwrap();
        assert_eq!(doc.params.len(), 1);
        assert!(doc.return_tag.is_some());
    }
}
