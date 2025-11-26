# Adding a New Rule to PHP-Checker

This guide explains how to add a new static analysis rule to the PHP-Checker project.

## Rule Categories

Rules are organized into the following categories based on their purpose:

- **`api/`** - API usage and deprecated function detection
- **`cleanup/`** - Code cleanup (unused variables, imports, etc.)
- **`control_flow/`** - Control flow analysis (unreachable code, fallthrough, etc.)
- **`psr4/`** - PSR-4 namespace validation
- **`sanity/`** - Basic sanity checks (undefined variables, duplicate declarations)
- **`security/`** - Security-related issues
- **`strict_typing/`** - Type checking and strict typing enforcement

Choose the most appropriate category for your rule, or create a new category if none fit.

## Step 1: Create the Rule File

Create a new Rust file in the appropriate category directory:

```bash
# For a new security rule
touch src/analyzer/rules/security/my_new_rule.rs
```

## Step 2: Implement the Rule Structure

Each rule must implement the `DiagnosticRule` trait. Here's a template:

```rust
use super::DiagnosticRule;
use super::helpers::{diagnostic_for_node, /* other helpers */};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};
use tree_sitter::Node;

pub struct MyNewRule;

impl MyNewRule {
    pub fn new() -> Self {
        Self
    }
}

impl DiagnosticRule for MyNewRule {
    fn name(&self) -> &str {
        "category/my_new_rule"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut visitor = MyNewRuleVisitor::new(parsed);
        visitor.visit(parsed.tree.root_node());
        visitor.diagnostics
    }

    // Optional: implement fix() for auto-fixable rules
    fn fix(&self, parsed: &parser::ParsedSource, _context: &ProjectContext) -> Vec<fix::TextEdit> {
        // Implement auto-fix logic here
        Vec::new()
    }
}

struct MyNewRuleVisitor<'a> {
    parsed: &'a parser::ParsedSource,
    diagnostics: Vec<crate::analyzer::Diagnostic>,
}

impl<'a> MyNewRuleVisitor<'a> {
    fn new(parsed: &'a parser::ParsedSource) -> Self {
        Self {
            parsed,
            diagnostics: Vec::new(),
        }
    }

    fn visit(&mut self, node: Node<'a>) {
        // Implement your rule logic here
        // Use tree_sitter::Node methods to traverse the AST

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
}
```

## Step 3: Register the Rule

### Update the Category Module

Add your rule to the category's `mod.rs` file:

```rust
// src/analyzer/rules/security/mod.rs
pub mod my_new_rule;

pub use my_new_rule::MyNewRule;
```

### Update the Main Rules Module

Add your rule to `src/analyzer/rules/mod.rs`:

```rust
// Add to the appropriate use statement
pub use security::{..., MyNewRule};

// Add to the pub use exports at the top
pub use security::MyNewRule;
```

### Register in the Analyzer

Add your rule to the `Analyzer::new()` method in `src/analyzer.rs`:

```rust
let mut rules: Vec<Box<dyn rules::DiagnosticRule>> = vec![
    // ... existing rules ...
    Box::new(rules::MyNewRule::new()),
];
```

## Step 4: Create Tests

### Test PHP File

Create a PHP test file in `tests/invalid/category/my_new_rule.php`:

```php
<?php

// Code that should trigger your rule
bad_example_function();

// Code that should NOT trigger your rule
good_example_function();
```

### Expected Output File

Create a corresponding `.expect` file with the expected diagnostic output:

```
error: description of the issue at 3:1
warning: another issue description at 7:5
```

The format is: `{severity}: {message} at {line}:{column}`

### Run Tests

Test your rule implementation:

```bash
# Run all tests
cargo test

# Run only the invalid suite tests
cargo test invalid_fixtures_match_expectations
```

## Step 5: Configuration Support

Your rule will automatically support configuration via `php_checker.yaml`:

```yaml
rules:
  category/my_new_rule: false  # Disable the rule
  category: false              # Disable all rules in the category
```

## Helper Functions

Use the helper functions in `src/analyzer/rules/helpers.rs`:

- **`diagnostic_for_node()`** - Create a diagnostic with proper span and snippet information
- **`child_by_kind()`** - Find a child node of a specific type
- **`node_text()`** - Extract text content from a node
- **`walk_node()`** - Recursively walk the AST
- **`find_descendant_by_kind()`** - Find any descendant of a specific type

## AST Exploration

Use the dump_tree binary to explore the AST structure:

```bash
# Build the dump tool
cargo build --bin dump_tree

# Dump AST for a PHP file
cargo run --bin dump_tree -- tests/invalid/category/my_new_rule.php
```

This will show you the tree-sitter node types and structure for your test cases.

## Rule Naming Convention

- Use lowercase with underscores: `my_new_rule`
- Category prefix in name: `category/my_new_rule`
- Match the file path: `src/analyzer/rules/category/my_new_rule.rs`

## Severity Levels

Choose the appropriate severity for your diagnostic:

- **`Error`** - Compilation-blocking issues or serious bugs
- **`Warning`** - Code quality issues that should be addressed
- **`Info`** - Suggestions or informational messages

## Auto-Fix Support (Optional)

To make your rule auto-fixable, implement the `fix()` method:

```rust
use crate::analyzer::fix;

impl DiagnosticRule for MyNewRule {
    fn fix(&self, parsed: &parser::ParsedSource, _context: &ProjectContext) -> Vec<fix::TextEdit> {
        // Return a list of text edits to fix the issues
        vec![fix::TextEdit::new(start_byte, end_byte, replacement_text)]
    }
}
```

Test auto-fixes by creating `.expect.fixed` files and running:

```bash
cargo run --bin php-checker -- analyse tests/invalid --fix --dry-run
```

## Examples

Look at existing rules for implementation examples:

- **`control_flow/fallthrough.rs`** - Simple visitor pattern
- **`cleanup/unused_variable.rs`** - Complex analysis with auto-fix
- **`sanity/undefined_variable.rs`** - Basic AST traversal

## Testing Tips

1. **Start Simple** - Create minimal test cases first
2. **Edge Cases** - Test with various PHP syntax variations
3. **False Positives** - Ensure your rule doesn't trigger on valid code
4. **Ignore Comments** - Rules automatically respect `php-checker-ignore` comments
5. **Performance** - Keep visitor logic efficient for large codebases

## Need Help?

- Check existing rules in the same category for patterns
- Use the dump_tree tool to understand AST structure
- Look at helper functions for common operations
- Test incrementally as you build your rule
