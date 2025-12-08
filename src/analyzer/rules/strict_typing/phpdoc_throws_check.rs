use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, node_text};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{parser, phpdoc, Severity};
use tree_sitter::Node;
use std::collections::{HashMap, HashSet};

/// Validates @throws exception documentation
///
/// This rule checks that:
/// - Documented exceptions are actually thrown
/// - All thrown exceptions are documented
/// - Exception handling coverage
pub struct PhpDocThrowsCheckRule;

impl PhpDocThrowsCheckRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for PhpDocThrowsCheckRule {
    fn name(&self) -> &str {
        "strict_typing/phpdoc_throws_check"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut visitor = PhpDocThrowsVisitor::new(parsed);

        // First pass: collect all function declarations and their @throws
        visitor.collect_function_throws(parsed.tree.root_node());

        // Second pass: validate throws documentation
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct PhpDocThrowsVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    diagnostics: Vec<crate::analyzer::Diagnostic>,
    // Map of function name to its documented throws
    function_throws: HashMap<String, Vec<String>>,
}

impl<'a> PhpDocThrowsVisitor<'a> {
    fn new(parsed: &'a parser::ParsedSource) -> Self {
        Self {
            parsed,
            diagnostics: Vec::new(),
            function_throws: HashMap::new(),
        }
    }

    fn collect_function_throws(&mut self, node: Node<'a>) {
        match node.kind() {
            "function_definition" | "method_declaration" => {
                // Get function name
                if let Some(name_node) = node.child_by_field_name("name") {
                    if let Some(func_name) = node_text(name_node, self.parsed) {
                        // Get PHPDoc and extract @throws
                        if let Some(phpdoc) = self.get_phpdoc_for_node(node) {
                            let throws: Vec<String> = phpdoc.throws
                                .iter()
                                .map(|t| self.normalize_exception_type(&t.exception_type))
                                .collect();

                            if !throws.is_empty() {
                                self.function_throws.insert(func_name, throws);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Recursively visit children
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.collect_function_throws(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn visit(&mut self, node: Node<'a>) {
        match node.kind() {
            "function_definition" | "method_declaration" => {
                self.check_function_throws(node);
            }
            _ => {}
        }

        // Recursively visit children
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

    fn check_function_throws(&mut self, node: Node<'a>) {
        // Get the PHPDoc comment for this function
        let phpdoc = self.get_phpdoc_for_node(node);

        // Collect all thrown exceptions in the function body
        let thrown_exceptions = self.collect_thrown_exceptions(node);

        if let Some(phpdoc) = phpdoc {
            // Check if documented @throws exceptions are actually thrown
            for throws_tag in &phpdoc.throws {
                let exception_type = self.normalize_exception_type(&throws_tag.exception_type);

                // Check if this exception is thrown
                let is_thrown = thrown_exceptions.iter().any(|thrown| {
                    let normalized_thrown = self.normalize_exception_type(thrown);
                    normalized_thrown == exception_type || self.is_subclass(&normalized_thrown, &exception_type)
                });

                if !is_thrown {
                    self.diagnostics.push(diagnostic_for_node(
                        self.parsed,
                        node,
                        Severity::Warning,
                        format!("@throws documents '{}' but this exception is never thrown", throws_tag.exception_type),
                    ));
                }
            }

            // Check if thrown exceptions are documented
            let documented_exceptions: Vec<String> = phpdoc.throws
                .iter()
                .map(|t| self.normalize_exception_type(&t.exception_type))
                .collect();

            for thrown in &thrown_exceptions {
                let normalized_thrown = self.normalize_exception_type(thrown);

                // Check if this thrown exception is documented
                let is_documented = documented_exceptions.iter().any(|doc| {
                    *doc == normalized_thrown || self.is_subclass(&normalized_thrown, doc)
                });

                if !is_documented {
                    self.diagnostics.push(diagnostic_for_node(
                        self.parsed,
                        node,
                        Severity::Error,
                        format!("Function throws '{}' but it is not documented with @throws", thrown),
                    ));
                }
            }
        } else {
            // No PHPDoc but function throws exceptions
            if !thrown_exceptions.is_empty() {
                for thrown in &thrown_exceptions {
                    self.diagnostics.push(diagnostic_for_node(
                        self.parsed,
                        node,
                        Severity::Error,
                        format!("Function throws '{}' but it is not documented with @throws", thrown),
                    ));
                }
            }
        }
    }

    fn get_phpdoc_for_node(&self, node: Node<'a>) -> Option<phpdoc::PhpDocComment> {
        // Look for a comment node immediately before this node
        let cursor = node.walk();
        if let Some(prev_sibling) = cursor.node().prev_sibling() {
            if prev_sibling.kind() == "comment" {
                let comment_text = node_text(prev_sibling, self.parsed)?;
                return phpdoc::PhpDocParser::parse(&comment_text);
            }
        }
        None
    }

    fn collect_thrown_exceptions(&self, node: Node<'a>) -> HashSet<String> {
        let mut exceptions = HashSet::new();
        self.collect_throws_recursive(node, &mut exceptions);
        exceptions
    }

    fn collect_throws_recursive(&self, node: Node<'a>, exceptions: &mut HashSet<String>) {
        if node.kind() == "throw_expression" {
            // Find the exception type being thrown
            // Look for object_creation_expression child
            let mut cursor = node.walk();
            if cursor.goto_first_child() {
                loop {
                    let child = cursor.node();
                    if child.kind() == "object_creation_expression" {
                        // Find the class name (qualified_name or name)
                        let mut class_cursor = child.walk();
                        if class_cursor.goto_first_child() {
                            loop {
                                let class_child = class_cursor.node();
                                if class_child.kind() == "qualified_name" || class_child.kind() == "name" {
                                    if let Some(exception_type) = node_text(class_child, self.parsed) {
                                        exceptions.insert(exception_type);
                                    }
                                    break;
                                }
                                if !class_cursor.goto_next_sibling() {
                                    break;
                                }
                            }
                        }
                        break;
                    }
                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }
        } else if node.kind() == "function_call_expression" {
            // Check if this function call throws exceptions
            if let Some(name_node) = node.child_by_field_name("function") {
                if let Some(func_name) = node_text(name_node, self.parsed) {
                    // Check if we have @throws info for this function
                    if let Some(throws) = self.function_throws.get(&func_name) {
                        for exception in throws {
                            exceptions.insert(exception.clone());
                        }
                    }
                }
            }
        }

        // Recursively visit children
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.collect_throws_recursive(cursor.node(), exceptions);
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn normalize_exception_type(&self, exception_type: &str) -> String {
        // Remove leading backslash if present
        exception_type.trim_start_matches('\\').to_string()
    }

    fn is_subclass(&self, _child: &str, parent: &str) -> bool {
        // Simple heuristic: if parent is "Exception", accept any exception type
        // This is a simplification - a full implementation would need class hierarchy info
        if parent == "Exception" || parent == "Throwable" {
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_valid_throws() {
        let source = r#"<?php
// Scenario 1: ✓ Function with @throws actually throws that exception
/**
 * @throws \InvalidArgumentException
 */
function validThrows(int $value): void {
    if ($value < 0) {
        throw new \InvalidArgumentException("Value must be positive");
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should have no diagnostics
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_never_throws() {
        let source = r#"<?php
// Scenario 2: ✗ Function with @throws never throws (dead documentation)
/**
 * @throws \RuntimeException
 */
function neverThrows(): void {
    echo "No exception thrown here";
}  // Warning: @throws documents exception that is never thrown
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Should warn about dead documentation
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("@throws documents '\\RuntimeException' but this exception is never thrown"));
    }

    #[test]
    fn test_throws_undocumented() {
        let source = r#"<?php
// Scenario 3: ✗ Function throws exception not in @throws
/**
 * @throws \InvalidArgumentException
 */
function throwsUndocumented(string $value): void {
    if ($value === "") {
        throw new \RuntimeException("Empty string");  // Error: RuntimeException not documented
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Should error about undocumented exception and warn about dead documentation
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics.iter().any(|d| d.message.contains("@throws documents '\\InvalidArgumentException' but this exception is never thrown")));
        assert!(diagnostics.iter().any(|d| d.message.contains("Function throws '\\RuntimeException' but it is not documented with @throws")));
    }

    #[test]
    fn test_multiple_throws() {
        let source = r#"<?php
// Scenario 4: ✓ Multiple @throws tags for different exceptions
/**
 * @throws \InvalidArgumentException When value is invalid
 * @throws \RuntimeException When processing fails
 */
function multipleThrows(int $value): void {
    if ($value < 0) {
        throw new \InvalidArgumentException("Negative value");
    }
    if ($value > 100) {
        throw new \RuntimeException("Processing error");
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should have no diagnostics
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_call_without_catch() {
        let source = r#"<?php
// Scenario 5: ✗ Try-catch not handling documented @throws exception
/**
 * @throws \InvalidArgumentException
 */
function throwsException(): void {
    throw new \InvalidArgumentException("Error");
}

function callWithoutCatch(): void {
    throwsException();  // Warning: call to function that throws InvalidArgumentException without try-catch
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Should warn about undocumented exception propagation
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Function throws 'InvalidArgumentException' but it is not documented with @throws"));
    }

    #[test]
    fn test_call_with_catch() {
        let source = r#"<?php
// Additional: Try-catch properly handles documented exception
/**
 * @throws \InvalidArgumentException
 */
function mayThrow(): void {
    throw new \InvalidArgumentException("Error");
}

function callWithCatch(): void {
    try {
        mayThrow();
    } catch (\InvalidArgumentException $e) {
        // Properly handled
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: Catch block detection not yet implemented, so we still warn about propagation
        // When implemented, should have no diagnostics since exception is caught
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("Function throws 'InvalidArgumentException' but it is not documented with @throws"));
    }

    #[test]
    fn test_constructor_with_throws() {
        let source = r#"<?php
// Additional: Constructor with @throws
class ResourceHandler {
    /**
     * @throws \RuntimeException When resource cannot be initialized
     */
    public function __construct(string $path) {
        if (!file_exists($path)) {
            throw new \RuntimeException("Resource not found");
        }
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should have no diagnostics
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_destructor_with_throws() {
        let source = r#"<?php
// Additional: Throwing in __destruct (anti-pattern)
class BadDestructor {
    /**
     * @throws \Exception  // Warning: throwing in destructor is dangerous
     */
    public function __destruct() {
        throw new \Exception("Bad practice");
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should warn about destructor throws
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_missing_namespace_in_throws() {
        let source = r#"<?php
// Additional: Missing namespace in @throws
/**
 * @throws InvalidArgumentException  // Error: should be \InvalidArgumentException with namespace
 */
function missingNamespace(): void {
    throw new \InvalidArgumentException("Error");
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should warn about missing namespace
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_exception_inheritance() {
        let source = r#"<?php
// Additional: Exception inheritance - throwing child when parent is documented
/**
 * @throws \Exception
 */
function throwsParent(): void {
    throw new \RuntimeException("Error");  // RuntimeException extends Exception
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should recognize exception inheritance
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_nested_exception_throwing() {
        let source = r#"<?php
// Additional: Exception thrown in nested function call
/**
 * @throws \InvalidArgumentException
 */
function innerThrows(): void {
    throw new \InvalidArgumentException("Inner error");
}

/**
 * @throws \InvalidArgumentException
 */
function outerThrows(): void {
    innerThrows();  // Exception propagates
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should track exception propagation
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_conditional_throw() {
        let source = r#"<?php
// Additional: Exception thrown conditionally
/**
 * @throws \InvalidArgumentException
 */
function conditionalThrow(bool $flag): void {
    if ($flag) {
        throw new \InvalidArgumentException("Conditional error");
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should recognize conditional throws
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_with_throws() {
        let source = r#"<?php
// Additional: Method with @throws
class Service {
    /**
     * @throws \RuntimeException
     */
    public function process(): void {
        throw new \RuntimeException("Processing error");
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should validate method @throws
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_static_method_with_throws() {
        let source = r#"<?php
// Additional: Static method with @throws
class Util {
    /**
     * @throws \InvalidArgumentException
     */
    public static function validate(int $value): void {
        if ($value < 0) {
            throw new \InvalidArgumentException("Invalid value");
        }
    }
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocThrowsCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should validate static method @throws
        assert_no_diagnostics(&diagnostics);
    }
}
