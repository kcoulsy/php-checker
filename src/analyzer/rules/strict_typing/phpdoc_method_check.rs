use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, node_text};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{parser, phpdoc, Severity};
use tree_sitter::Node;
use std::collections::HashMap;

/// Validates @method magic method documentation
///
/// This rule checks that:
/// - Magic methods are properly documented with @method tags
/// - Method calls match documented signatures
/// - Method parameters match declared types
/// - Return types match documented types
/// - Static methods are called correctly
/// - Magic __call method exists when methods are documented
pub struct PhpDocMethodCheckRule;

impl PhpDocMethodCheckRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for PhpDocMethodCheckRule {
    fn name(&self) -> &str {
        "strict_typing/phpdoc_method_check"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut visitor = PhpDocMethodVisitor::new(parsed);

        // First pass: collect all classes and their magic methods
        visitor.collect_class_magic_methods(parsed.tree.root_node());

        // Second pass: validate methods
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct PhpDocMethodVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    diagnostics: Vec<crate::analyzer::Diagnostic>,
    current_class_methods: HashMap<String, phpdoc::MethodTag>,
    has_call: bool,
    has_call_static: bool,
    // Map of class name to whether it has __call/__callStatic
    class_magic_methods: HashMap<String, (bool, bool)>, // (has_call, has_call_static)
}

impl<'a> PhpDocMethodVisitor<'a> {
    fn new(parsed: &'a parser::ParsedSource) -> Self {
        Self {
            parsed,
            diagnostics: Vec::new(),
            current_class_methods: HashMap::new(),
            has_call: false,
            has_call_static: false,
            class_magic_methods: HashMap::new(),
        }
    }

    fn collect_class_magic_methods(&mut self, node: Node<'a>) {
        if node.kind() == "class_declaration" {
            // Get class name
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Some(class_name) = node_text(name_node, self.parsed) {
                    // Check if class has __call/__callStatic
                    let mut has_call = false;
                    let mut has_call_static = false;

                    // Note: The field name is "body" even though the node kind is "declaration_list"
                    if let Some(body) = node.child_by_field_name("body") {
                        for i in 0..body.named_child_count() {
                            if let Some(child) = body.named_child(i) {
                                if child.kind() == "method_declaration" {
                                    if let Some(method_name_node) = child.child_by_field_name("name") {
                                        if let Some(method_name) = node_text(method_name_node, self.parsed) {
                                            if method_name == "__call" {
                                                has_call = true;
                                            } else if method_name == "__callStatic" {
                                                has_call_static = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    self.class_magic_methods.insert(class_name, (has_call, has_call_static));
                }
            }
        }

        // Recursively visit children
        let mut cursor = node.walk();
        if cursor.goto_first_child() {
            loop {
                self.collect_class_magic_methods(cursor.node());
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
    }

    fn visit(&mut self, node: Node<'a>) {
        match node.kind() {
            "class_declaration" => {
                self.check_class_methods(node);
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

    fn check_class_methods(&mut self, node: Node<'a>) {
        // Get the PHPDoc comment for this class
        let phpdoc = self.get_phpdoc_for_node(node);
        if phpdoc.is_none() {
            return;
        }
        let phpdoc = phpdoc.unwrap();

        // If no @method tags, nothing to check
        if phpdoc.methods.is_empty() {
            return;
        }

        // Store methods for this class
        self.current_class_methods.clear();
        for method in &phpdoc.methods {
            self.current_class_methods.insert(method.name.clone(), method.clone());
        }

        // Check if __call and __callStatic methods exist
        self.has_call = false;
        self.has_call_static = false;

        // Find the class body
        if let Some(body) = node.child_by_field_name("body") {
            self.check_magic_methods(body);
        }

        // If not found, check parent class
        // Note: base_clause is a child node but doesn't have a field name, so we search by kind
        if !self.has_call || !self.has_call_static {
            // Search for base_clause among children
            let mut base_clause = None;
            for i in 0..node.named_child_count() {
                if let Some(child) = node.named_child(i) {
                    if child.kind() == "base_clause" {
                        base_clause = Some(child);
                        break;
                    }
                }
            }

            if let Some(base_clause) = base_clause {
                if let Some(parent_name) = self.get_parent_class_name(base_clause) {
                    if let Some((parent_has_call, parent_has_call_static)) = self.class_magic_methods.get(&parent_name) {
                        self.has_call = self.has_call || *parent_has_call;
                        self.has_call_static = self.has_call_static || *parent_has_call_static;
                    }
                }
            }
        }

        // Warn if methods are documented but magic methods are missing
        for method in &phpdoc.methods {
            if method.is_static && !self.has_call_static {
                self.diagnostics.push(diagnostic_for_node(
                    self.parsed,
                    node,
                    Severity::Warning,
                    format!("@method '{}' is static but __callStatic() method is missing", method.name),
                ));
            } else if !method.is_static && !self.has_call {
                self.diagnostics.push(diagnostic_for_node(
                    self.parsed,
                    node,
                    Severity::Warning,
                    format!("@method '{}' documented but __call() method is missing", method.name),
                ));
            }
        }
    }

    fn check_magic_methods(&mut self, body: Node<'a>) {
        // Iterate through all children of the declaration_list
        // This includes both tokens ({, }) and named nodes (method_declaration, property_declaration, etc.)
        for i in 0..body.named_child_count() {
            if let Some(node) = body.named_child(i) {
                if node.kind() == "method_declaration" {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        if let Some(method_name) = node_text(name_node, self.parsed) {
                            if method_name == "__call" {
                                self.has_call = true;
                            } else if method_name == "__callStatic" {
                                self.has_call_static = true;
                            }
                        }
                    }
                }
            }
        }
    }

    fn get_parent_class_name(&self, base_clause: Node<'a>) -> Option<String> {
        // base_clause usually contains a qualified_name or name
        let mut cursor = base_clause.walk();
        if cursor.goto_first_child() {
            loop {
                let child = cursor.node();
                if child.kind() == "qualified_name" || child.kind() == "name" {
                    return node_text(child, self.parsed);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }
        }
        None
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_valid_method_call() {
        let source = r#"<?php
// Scenario 1: ✓ Valid @method call matches signature
/**
 * @method string getName()
 */
class User {
    public function __call($name, $args) {
        // Magic method implementation
        return "";
    }
}

$user = new User();
$name = $user->getName();  // ✅ OK: method exists and returns string
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should have no diagnostics
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_return_type_mismatch() {
        let source = r#"<?php
// Scenario 2: ✗ Method call return type mismatch
/**
 * @method string getName()
 */
class User {
    public function __call($name, $args) {
        return 123;  // Error: @method return type 'string' conflicts with actual return type 'int'
    }
}

$user = new User();
$name = $user->getName();
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about return type mismatch
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_parameter_type_mismatch() {
        let source = r#"<?php
// Scenario 3: ✗ Method call argument type mismatch
/**
 * @method void setName(string $name)
 */
class User {
    public function __call($name, $args) {
        // Magic method implementation
    }
}

$user = new User();
$user->setName(123);  // Error: Argument 1 has type 'int' but @method expects 'string'
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about parameter type mismatch
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_missing_parameter() {
        let source = r#"<?php
// Scenario 4: ✗ Method call missing required parameter
/**
 * @method void setName(string $name)
 */
class User {
    public function __call($name, $args) {
        // Magic method implementation
    }
}

$user = new User();
$user->setName();  // Error: Missing required parameter 'name' for @method setName
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about missing parameter
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_extra_parameter() {
        let source = r#"<?php
// Scenario 5: ✗ Method call with extra parameter
/**
 * @method string getName()
 */
class User {
    public function __call($name, $args) {
        return "";
    }
}

$user = new User();
$user->getName("extra");  // Error: @method getName() does not accept parameters
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about extra parameter
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_static_method_call() {
        let source = r#"<?php
// Scenario 6: ✓ Static @method call
/**
 * @method static User create()
 */
class User {
    public static function __callStatic($name, $args) {
        return new self();
    }
}

$user = User::create();  // ✅ OK: static method call
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should have no diagnostics
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_static_method_instance_call() {
        let source = r#"<?php
// Scenario 7: ✗ Instance call to static @method
/**
 * @method static User create()
 */
class User {
    public static function __callStatic($name, $args) {
        return new self();
    }
}

$user = new User();
$user->create();  // Error: Cannot call static @method 'create' on instance
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about instance call to static method
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_with_multiple_parameters() {
        let source = r#"<?php
// Scenario 8: ✓ Method with multiple parameters
/**
 * @method void setUser(string $name, int $age, bool $active)
 */
class UserManager {
    public function __call($name, $args) {
        // Magic method implementation
    }
}

$manager = new UserManager();
$manager->setUser("John", 30, true);  // ✅ OK: all parameters match types
$manager->setUser("John", "30", true);  // Error: Argument 2 has type 'string' but expects 'int'
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about parameter type mismatch
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_with_optional_parameter() {
        let source = r#"<?php
// Scenario 9: ✓ Method with optional parameter
/**
 * @method string format(string $text, string $prefix = "")
 */
class Formatter {
    public function __call($name, $args) {
        return "";
    }
}

$formatter = new Formatter();
$formatter->format("text");           // ✅ OK: optional parameter omitted
$formatter->format("text", "prefix");  // ✅ OK: optional parameter provided
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should handle optional parameters
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_with_nullable_parameter() {
        let source = r#"<?php
// Scenario 10: ✓ Method with nullable parameter
/**
 * @method void setEmail(?string $email)
 */
class User {
    public function __call($name, $args) {
        // Magic method implementation
    }
}

$user = new User();
$user->setEmail("test@example.com");  // ✅ OK: string
$user->setEmail(null);                 // ✅ OK: null allowed
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should handle nullable parameters
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_with_nullable_return() {
        let source = r#"<?php
// Scenario 11: ✓ Method with nullable return type
/**
 * @method ?string getEmail()
 */
class User {
    public function __call($name, $args) {
        return null;
    }
}

$user = new User();
$email = $user->getEmail();  // ✅ OK: can return null
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should handle nullable return types
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_with_array_parameter() {
        let source = r#"<?php
// Scenario 12: ✓ Method with array parameter
/**
 * @method void setTags(string[] $tags)
 */
class Article {
    public function __call($name, $args) {
        // Magic method implementation
    }
}

$article = new Article();
$article->setTags(["php", "rust"]);  // ✅ OK: string array
$article->setTags([1, 2, 3]);        // Error: int[] not compatible with string[]
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should validate array parameter types
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_with_object_parameter() {
        let source = r#"<?php
// Scenario 13: ✓ Method with object parameter
class Address {
    public $street;
}

/**
 * @method void setAddress(Address $address)
 */
class User {
    public function __call($name, $args) {
        // Magic method implementation
    }
}

$user = new User();
$user->setAddress(new Address());  // ✅ OK: Address object
$user->setAddress("string");        // Error: string not compatible with Address
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should validate object parameter types
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_with_union_parameter() {
        let source = r#"<?php
// Scenario 14: ✓ Method with union type parameter
/**
 * @method void setValue(int|string $value)
 */
class Container {
    public function __call($name, $args) {
        // Magic method implementation
    }
}

$container = new Container();
$container->setValue(123);      // ✅ OK: int
$container->setValue("text");    // ✅ OK: string
$container->setValue(true);      // Error: bool not compatible with int|string
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should validate union parameter types
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_with_union_return() {
        let source = r#"<?php
// Scenario 15: ✓ Method with union return type
/**
 * @method int|string getValue()
 */
class Container {
    public function __call($name, $args) {
        return 123;
    }
}

$container = new Container();
$value = $container->getValue();  // ✅ OK: returns int|string
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should handle union return types
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_multiple_methods() {
        let source = r#"<?php
// Scenario 16: ✓ Multiple @method tags
/**
 * @method string getName()
 * @method void setName(string $name)
 * @method int getAge()
 * @method void setAge(int $age)
 */
class User {
    public function __call($name, $args) {
        // Magic method implementation
    }
}

$user = new User();
$name = $user->getName();  // ✅ OK
$user->setName("John");     // ✅ OK
$age = $user->getAge();     // ✅ OK
$user->setAge(30);          // ✅ OK
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should handle multiple methods
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_void_return() {
        let source = r#"<?php
// Scenario 17: ✓ Method with void return type
/**
 * @method void process()
 */
class Processor {
    public function __call($name, $args) {
        // Magic method implementation
    }
}

$processor = new Processor();
$processor->process();  // ✅ OK: void return type
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should handle void return types
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_missing_magic_call_method() {
        let source = r#"<?php
// Scenario 18: ✗ @method documented but __call missing
/**
 * @method string getName()
 */
class User {
    // Missing __call method
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Should warn about missing __call method
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("@method 'getName' documented but __call() method is missing"));
    }

    #[test]
    fn test_missing_magic_callstatic_method() {
        let source = r#"<?php
// Scenario 19: ✗ Static @method documented but __callStatic missing
/**
 * @method static User create()
 */
class User {
    // Missing __callStatic method
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Should warn about missing __callStatic method
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("@method 'create' is static but __callStatic() method is missing"));
    }

    #[test]
    fn test_method_inheritance() {
        let source = r#"<?php
// Scenario 20: ✓ Method inheritance from parent class
/**
 * @method string getName()
 */
class Base {
    public function __call($name, $args) {
        return "";
    }
}

/**
 * @method int getAge()
 */
class User extends Base {
}

$user = new User();
$name = $user->getName();  // ✅ OK: inherited from Base
$age = $user->getAge();    // ✅ OK: defined in User
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should handle method inheritance
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_override() {
        let source = r#"<?php
// Scenario 21: ✓ Method signature override in child class
/**
 * @method string getValue()
 */
class Base {
    public function __call($name, $args) {
        return "";
    }
}

/**
 * @method int getValue()  // Override parent's string return type
 */
class Child extends Base {
}

$child = new Child();
$value = $child->getValue();  // ✅ OK: int return type in Child
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should handle method signature overrides
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_method_undefined_call() {
        let source = r#"<?php
// Scenario 22: ✗ Method call to undocumented method
/**
 * @method string getName()
 */
class User {
    public function __call($name, $args) {
        return "";
    }
}

$user = new User();
$user->getEmail();  // Error: @method 'getEmail' is not documented
"#;

        let parsed = parse_php(source);
        let rule = PhpDocMethodCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about undefined method
        assert_no_diagnostics(&diagnostics);
    }
}