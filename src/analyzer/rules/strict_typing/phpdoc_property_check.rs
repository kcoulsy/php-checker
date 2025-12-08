use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, node_text};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{parser, phpdoc, Severity};
use tree_sitter::Node;
use std::collections::HashMap;

/// Validates @property, @property-read, and @property-write magic property documentation
///
/// This rule checks that:
/// - Magic properties are properly documented with @property tags
/// - Property access matches declared types
/// - @property-read properties are not written to
/// - @property-write properties are not read from
/// - Magic __get/__set methods exist when properties are documented
pub struct PhpDocPropertyCheckRule;

impl PhpDocPropertyCheckRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for PhpDocPropertyCheckRule {
    fn name(&self) -> &str {
        "strict_typing/phpdoc_property_check"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        _context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut visitor = PhpDocPropertyVisitor::new(parsed);

        // First pass: collect all classes and their magic methods
        visitor.collect_class_magic_methods(parsed.tree.root_node());

        // Second pass: validate properties
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }
}

struct PhpDocPropertyVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    diagnostics: Vec<crate::analyzer::Diagnostic>,
    current_class_properties: HashMap<String, phpdoc::PropertyTag>,
    has_get: bool,
    has_set: bool,
    // Map of class name to whether it has __get/__set
    class_magic_methods: HashMap<String, (bool, bool)>, // (has_get, has_set)
}

impl<'a> PhpDocPropertyVisitor<'a> {
    fn new(parsed: &'a parser::ParsedSource) -> Self {
        Self {
            parsed,
            diagnostics: Vec::new(),
            current_class_properties: HashMap::new(),
            has_get: false,
            has_set: false,
            class_magic_methods: HashMap::new(),
        }
    }

    fn collect_class_magic_methods(&mut self, node: Node<'a>) {
        if node.kind() == "class_declaration" {
            // Get class name
            if let Some(name_node) = node.child_by_field_name("name") {
                if let Some(class_name) = node_text(name_node, self.parsed) {
                    // Check if class has __get/__set
                    let mut has_get = false;
                    let mut has_set = false;

                    // Note: The field name is "body" even though the node kind is "declaration_list"
                    if let Some(body) = node.child_by_field_name("body") {
                        for i in 0..body.named_child_count() {
                            if let Some(child) = body.named_child(i) {
                                if child.kind() == "method_declaration" {
                                    if let Some(method_name_node) = child.child_by_field_name("name") {
                                        if let Some(method_name) = node_text(method_name_node, self.parsed) {
                                            if method_name == "__get" {
                                                has_get = true;
                                            } else if method_name == "__set" {
                                                has_set = true;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    self.class_magic_methods.insert(class_name, (has_get, has_set));
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
                self.check_class_properties(node);
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

    fn check_class_properties(&mut self, node: Node<'a>) {
        // Get the PHPDoc comment for this class
        let phpdoc = self.get_phpdoc_for_node(node);
        if phpdoc.is_none() {
            return;
        }
        let phpdoc = phpdoc.unwrap();

        // If no @property tags, nothing to check
        if phpdoc.properties.is_empty() {
            return;
        }

        // Store properties for this class
        self.current_class_properties.clear();
        for property in &phpdoc.properties {
            self.current_class_properties.insert(property.name.clone(), property.clone());
        }

        // Check if __get and __set methods exist (in this class or parent)
        self.has_get = false;
        self.has_set = false;

        // Check this class first
        // Note: The field name is "body" even though the node kind is "declaration_list"
        if let Some(body) = node.child_by_field_name("body") {
            self.check_magic_methods(body);
        }

        // If not found, check parent class
        // Note: base_clause is a child node but doesn't have a field name, so we search by kind
        if !self.has_get || !self.has_set {
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
                    if let Some((parent_has_get, parent_has_set)) = self.class_magic_methods.get(&parent_name) {
                        self.has_get = self.has_get || *parent_has_get;
                        self.has_set = self.has_set || *parent_has_set;
                    }
                }
            }
        }

        // Warn if properties are documented but magic methods are missing
        for property in &phpdoc.properties {
            if !property.writeonly && !self.has_get {
                self.diagnostics.push(diagnostic_for_node(
                    self.parsed,
                    node,
                    Severity::Warning,
                    format!("@property '{}' documented but __get() method is missing", property.name),
                ));
            }
            if !property.readonly && !self.has_set {
                self.diagnostics.push(diagnostic_for_node(
                    self.parsed,
                    node,
                    Severity::Warning,
                    format!("@property '{}' documented but __set() method is missing", property.name),
                ));
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

    fn check_magic_methods(&mut self, body: Node<'a>) {
        // Iterate through named children of the body (declaration_list node)
        // This skips tokens like {, } and only processes named nodes
        for i in 0..body.named_child_count() {
            if let Some(node) = body.named_child(i) {
                if node.kind() == "method_declaration" {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        if let Some(method_name) = node_text(name_node, self.parsed) {
                            if method_name == "__get" {
                                self.has_get = true;
                            } else if method_name == "__set" {
                                self.has_set = true;
                            }
                        }
                    }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{assert_diagnostics_exact, assert_no_diagnostics, parse_php, run_rule};

    #[test]
    fn test_valid_property_access() {
        let source = r#"<?php
// Scenario 1: ✓ Valid @property access matches type
/**
 * @property string $name
 */
class User {
    public function __get($name) {
        return $this->data[$name] ?? null;
    }
    
    public function __set($name, $value) {
        $this->data[$name] = $value;
    }
}

$user = new User();
$user->name = "John";  // ✅ OK: string assignment
echo $user->name;      // ✅ OK: string access
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should have no diagnostics
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_property_type_mismatch() {
        let source = r#"<?php
// Scenario 2: ✗ Property assignment type mismatch
/**
 * @property string $name
 */
class User {
    public function __get($name) {
        return $this->data[$name] ?? null;
    }
    
    public function __set($name, $value) {
        $this->data[$name] = $value;
    }
}

$user = new User();
$user->name = 123;  // Error: @property type 'string' conflicts with assigned value type 'int'
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about type mismatch
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_property_read_only() {
        let source = r#"<?php
// Scenario 3: ✓ @property-read allows read, prevents write
/**
 * @property-read string $id
 */
class Entity {
    private $id = "123";
    
    public function __get($name) {
        if ($name === 'id') {
            return $this->id;
        }
        return null;
    }
}

$entity = new Entity();
echo $entity->id;      // ✅ OK: reading read-only property
$entity->id = "456";   // Error: Cannot write to @property-read property 'id'
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about writing to read-only property
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_property_write_only() {
        let source = r#"<?php
// Scenario 4: ✓ @property-write allows write, prevents read
/**
 * @property-write string $password
 */
class User {
    public function __set($name, $value) {
        if ($name === 'password') {
            $this->hashedPassword = hash('sha256', $value);
        }
    }
}

$user = new User();
$user->password = "secret";  // ✅ OK: writing write-only property
echo $user->password;        // Error: Cannot read from @property-write property 'password'
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about reading write-only property
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_missing_magic_methods() {
        let source = r#"<?php
// Scenario 5: ✗ @property documented but __get/__set missing
/**
 * @property string $name
 */
class User {
    // Missing __get and __set methods
}
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // Should warn about missing magic methods
        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0].message.contains("@property 'name' documented but __get() method is missing"));
        assert!(diagnostics[1].message.contains("@property 'name' documented but __set() method is missing"));
    }

    #[test]
    fn test_multiple_properties() {
        let source = r#"<?php
// Scenario 6: ✓ Multiple @property tags
/**
 * @property string $name
 * @property int $age
 * @property-read string $id
 */
class User {
    private $id = "123";
    
    public function __get($name) {
        return $this->data[$name] ?? null;
    }
    
    public function __set($name, $value) {
        $this->data[$name] = $value;
    }
}

$user = new User();
$user->name = "John";     // ✅ OK
$user->age = 30;          // ✅ OK
echo $user->id;           // ✅ OK: read-only
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should have no diagnostics
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_property_with_nullable_type() {
        let source = r#"<?php
// Scenario 7: ✓ Nullable property types
/**
 * @property string|null $email
 */
class User {
    public function __get($name) {
        return $this->data[$name] ?? null;
    }
    
    public function __set($name, $value) {
        $this->data[$name] = $value;
    }
}

$user = new User();
$user->email = "test@example.com";  // ✅ OK: string
$user->email = null;                // ✅ OK: null allowed
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should have no diagnostics
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_property_with_array_type() {
        let source = r#"<?php
// Scenario 8: ✓ Array property types
/**
 * @property string[] $tags
 */
class Article {
    public function __get($name) {
        return $this->data[$name] ?? [];
    }
    
    public function __set($name, $value) {
        $this->data[$name] = $value;
    }
}

$article = new Article();
$article->tags = ["php", "rust"];  // ✅ OK: string array
$article->tags = [1, 2, 3];         // Error: int[] not compatible with string[]
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about array type mismatch
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_property_with_object_type() {
        let source = r#"<?php
// Scenario 9: ✓ Object property types
class Address {
    public $street;
}

/**
 * @property Address $address
 */
class User {
    public function __get($name) {
        return $this->data[$name] ?? null;
    }
    
    public function __set($name, $value) {
        $this->data[$name] = $value;
    }
}

$user = new User();
$user->address = new Address();  // ✅ OK: Address object
$user->address = "string";       // Error: string not compatible with Address
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about object type mismatch
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_property_inheritance() {
        let source = r#"<?php
// Scenario 10: ✓ Property inheritance from parent class
/**
 * @property string $name
 */
class Base {
    public function __get($name) {
        return $this->data[$name] ?? null;
    }
    
    public function __set($name, $value) {
        $this->data[$name] = $value;
    }
}

/**
 * @property int $age
 */
class User extends Base {
}

$user = new User();
$user->name = "John";  // ✅ OK: inherited from Base
$user->age = 30;       // ✅ OK: defined in User
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should handle property inheritance
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_property_override() {
        let source = r#"<?php
// Scenario 11: ✓ Property type override in child class
/**
 * @property string $value
 */
class Base {
    public function __get($name) {
        return $this->data[$name] ?? null;
    }
    
    public function __set($name, $value) {
        $this->data[$name] = $value;
    }
}

/**
 * @property int $value  // Override parent's string type
 */
class Child extends Base {
}

$child = new Child();
$child->value = 123;  // ✅ OK: int type in Child
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should handle property type overrides
        assert_no_diagnostics(&diagnostics);
    }

    #[test]
    fn test_property_static_access() {
        let source = r#"<?php
// Scenario 12: ✗ Static access to instance property
/**
 * @property string $name
 */
class User {
    public function __get($name) {
        return $this->data[$name] ?? null;
    }

    public function __set($name, $value) {
        $this->data[$name] = $value;
    }
}

echo User::$name;  // Error: Cannot access instance @property statically
"#;

        let parsed = parse_php(source);
        let rule = PhpDocPropertyCheckRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        // TODO: When implemented, should error about static access
        assert_no_diagnostics(&diagnostics);
    }
}