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

Tests are now written directly in the same file as your rule implementation, using Rust's built-in test framework. This keeps tests close to the code they're testing and makes it easier to maintain.

### Test Structure

Add a `#[cfg(test)]` module at the end of your rule file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::*;

    #[test]
    fn test_my_rule_triggered() {
        // Test code here
    }

    #[test]
    fn test_my_rule_not_triggered() {
        // Test code here
    }
}
```

### Test Helper Functions

The `test_utils` module provides several helper functions to make testing easier:

#### Parsing PHP Code

- **`parse_php(source: &str)`** - Parse PHP source code into a `ParsedSource` for testing
  ```rust
  let source = r#"<?php
  function test() {
      return 42;
  }
  "#;
  let parsed = parse_php(source);
  ```

- **`parse_php_with_path(source: &str, path: &str)`** - Parse with a custom file path
  - Useful for rules that check the filename (e.g., `strict_types` rule)
  ```rust
  let parsed = parse_php_with_path(source, "strict_missing.php");
  ```

#### Running Rules

- **`run_rule<R>(rule: &R, parsed: &ParsedSource)`** - Run a rule on parsed PHP code
  ```rust
  let rule = MyNewRule::new();
  let diagnostics = run_rule(&rule, &parsed);
  ```

- **`run_rule_with_context<R>(rule: &R, source: &str)`** - Run a rule with context that includes the parsed file
  - Useful for rules that need to resolve symbols defined in the same file
  ```rust
  let diagnostics = run_rule_with_context(&rule, source);
  ```

#### Asserting Diagnostics

- **`assert_no_diagnostics(diagnostics: &[Diagnostic])`** - Assert that no diagnostics were produced (happy path)
  ```rust
  let diagnostics = run_rule(&rule, &parsed);
  assert_no_diagnostics(&diagnostics);
  ```

- **`assert_diagnostics(diagnostics: &[Diagnostic], expected_messages: &[&str])`** - Assert diagnostics match expected messages (case-insensitive substring match)
  ```rust
  assert_diagnostics(&diagnostics, &["inconsistent return type"]);
  ```

- **`assert_diagnostics_exact(diagnostics: &[Diagnostic], expected_lines: &[&str])`** - Assert exact match in the format used in `.expect` files
  ```rust
  assert_diagnostics_exact(&diagnostics, &[
      "error: missing required argument 2 for takesTwo"
  ]);
  ```

#### Testing Auto-Fixes

- **`run_fix<R>(rule: &R, parsed: &ParsedSource)`** - Run a rule's fix function
- **`run_fix_with_context<R>(rule: &R, source: &str)`** - Run fix with context
- **`assert_fix<R>(rule: &R, parsed: &ParsedSource, input: &str, expected: &str)`** - Assert fix produces expected output
- **`assert_fix_with_path<R>(rule: &R, input: &str, expected: &str, path: &str)`** - Assert fix with custom path
- **`assert_fix_with_context<R>(rule: &R, input: &str, expected: &str)`** - Assert fix with context

### What to Test

#### Happy Paths (No Diagnostics)

Test cases where your rule should **not** trigger:

- Valid code that might be similar to problematic code
- Edge cases that are actually acceptable
- Code that should be ignored by your rule

```rust
#[test]
fn test_valid_code() {
    let source = r#"<?php
    function test(): int {
        return 42;
    }
    "#;

    let parsed = parse_php(source);
    let rule = MyNewRule::new();
    let diagnostics = run_rule(&rule, &parsed);

    assert_no_diagnostics(&diagnostics);
}
```

#### Unhappy Paths (Diagnostics Expected)

Test cases where your rule **should** trigger:

- The main problematic pattern your rule detects
- Variations of the problem (different syntax, contexts)
- Multiple issues in the same file
- Edge cases that should trigger the rule

```rust
#[test]
fn test_rule_triggered() {
    let source = r#"<?php
    /**
     * @return string
     */
    function test(): int {
        return 42;
    }
    "#;

    let parsed = parse_php(source);
    let rule = PhpDocReturnCheckRule::new();
    let diagnostics = run_rule(&rule, &parsed);

    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].message.contains("@return type 'string' conflicts"));
}
```

#### Testing with Context

If your rule needs to resolve symbols (functions, classes, etc.) defined in the same file, use `run_rule_with_context`:

```rust
#[test]
fn test_with_context() {
    let source = r#"<?php
    function takesTwo(int $a, int $b): void {}
    takesTwo(1);  // Missing second argument
    "#;

    let rule = MissingArgumentRule::new();
    let diagnostics = run_rule_with_context(&rule, source);

    assert_diagnostics_exact(&diagnostics, &[
        "error: missing required argument 2 for takesTwo"
    ]);
}
```

### Complete Test Example

Here's a complete example from `phpdoc_return_check.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{parse_php, run_rule, assert_no_diagnostics};

    #[test]
    fn test_return_type_conflict() {
        // Unhappy path: @return conflicts with native type
        let source = r#"<?php
        /**
         * @return string
         */
        function test(): int {
            return 42;
        }
        "#;

        let parsed = parse_php(source);
        let context = ProjectContext::new();
        let rule = PhpDocReturnCheckRule::new();
        let diagnostics = rule.run(&parsed, &context);

        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains(
            "@return type 'string' conflicts with native return type hint 'int'"
        ));
    }

    #[test]
    fn test_return_type_matches() {
        // Happy path: @return matches native type
        let source = r#"<?php
        /**
         * @return int
         */
        function test(): int {
            return 42;
        }
        "#;

        let parsed = parse_php(source);
        let context = ProjectContext::new();
        let rule = PhpDocReturnCheckRule::new();
        let diagnostics = rule.run(&parsed, &context);

        assert_no_diagnostics(&diagnostics);
    }
}
```

### Run Tests

Test your rule implementation:

```bash
# Run all tests
cargo test

# Run tests for a specific rule file
cargo test --test lib phpdoc_return_check

# Run a specific test
cargo test test_return_type_conflict
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

Test auto-fixes using the test helpers:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::*;

    #[test]
    fn test_auto_fix() {
        let input = r#"<?php
        function test(): void {}
        "#;

        let expected = r#"<?php

        declare(strict_types=1);

        function test(): void {}
        "#;

        let rule = StrictTypesRule::new();
        let parsed = parse_php_with_path(input, "strict_missing.php");
        assert_fix(&rule, &parsed, input, expected);
    }
}
```

Alternatively, you can test auto-fixes by creating `.expect.fixed` files in the `tests/invalid/` directory and running:

```bash
cargo run --bin php-checker -- analyse tests/invalid --fix --dry-run
```

## Examples

Look at existing rules for implementation examples:

- **`strict_typing/phpdoc_return_check.rs`** - PHPDoc validation with comprehensive in-file tests (happy and unhappy paths)
- **`strict_typing/phpdoc_param_check.rs`** - Parameter type checking with tests
- **`strict_typing/missing_argument.rs`** - Context-aware rule with `run_rule_with_context` tests
- **`control_flow/fallthrough.rs`** - Simple visitor pattern
- **`cleanup/unused_variable.rs`** - Complex analysis with auto-fix
- **`sanity/undefined_variable.rs`** - Basic AST traversal

Each of these rules includes a `#[cfg(test)]` module at the bottom showing how to test the rule using the `test_utils` helpers.

## Testing Tips

1. **Start Simple** - Create minimal test cases first, then add complexity
2. **Test Both Paths** - Always test both happy paths (no diagnostics) and unhappy paths (diagnostics expected)
3. **Use Helpers** - Leverage `test_utils` helpers instead of manually parsing and asserting
4. **Edge Cases** - Test with various PHP syntax variations and edge cases
5. **False Positives** - Ensure your rule doesn't trigger on valid code (happy path tests)
6. **Multiple Issues** - Test scenarios with multiple issues in the same file
7. **Context-Aware Rules** - Use `run_rule_with_context` for rules that need symbol resolution
8. **Ignore Comments** - Rules automatically respect `php-checker-ignore` comments
9. **Performance** - Keep visitor logic efficient for large codebases
10. **Descriptive Names** - Use clear test function names like `test_rule_triggered` and `test_valid_code`

## Need Help?

- Check existing rules in the same category for patterns
- Use the dump_tree tool to understand AST structure
- Look at helper functions for common operations
- Test incrementally as you build your rule
