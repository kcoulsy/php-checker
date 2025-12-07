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

Tests should be **colocated** in the rule file itself using a `#[cfg(test)]` module. This keeps tests close to the implementation and makes it easier to maintain.

### Test Module Structure

Add a test module at the bottom of your rule file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{
        assert_diagnostics_exact, 
        assert_no_diagnostics, 
        parse_php, 
        run_rule
    };

    #[test]
    fn test_my_rule_detects_issue() {
        let source = r#"<?php
// Code that should trigger your rule
bad_example_function();
"#;

        let parsed = parse_php(source);
        let rule = MyNewRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics, 
            &["error: description of the issue at 2:1"]
        );
    }

    #[test]
    fn test_my_rule_no_false_positives() {
        let source = r#"<?php
// Code that should NOT trigger your rule
good_example_function();
"#;

        let parsed = parse_php(source);
        let rule = MyNewRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }
}
```

### Run Tests

Test your rule implementation:

```bash
# Run all tests
cargo test

# Run tests for a specific rule
cargo test my_new_rule

# Run tests in a specific module
cargo test --lib analyzer::rules::category::my_new_rule
```

## Test Utilities Reference

The `test_utils` module (`src/analyzer/rules/test_utils.rs`) provides comprehensive utilities for writing rule tests. All functions are designed to work with colocated tests.

### Parsing Functions

#### `parse_php(source: &str) -> ParsedSource`

Parse PHP source code into a `ParsedSource` for testing. Uses a default path of `"test.php"`.

```rust
use crate::analyzer::rules::test_utils::parse_php;

let source = r#"<?php
function test() {
    return 42;
}
"#;

let parsed = parse_php(source);
```

#### `parse_php_with_path(source: &str, path: &str) -> ParsedSource`

Parse PHP source code with a custom file path. Useful for rules that check the filename (e.g., `strict_types` rule).

```rust
use crate::analyzer::rules::test_utils::parse_php_with_path;

let parsed = parse_php_with_path(source, "strict_missing.php");
```

### Running Rules

#### `run_rule<R>(rule: &R, parsed: &ParsedSource) -> Vec<Diagnostic>`

Run a rule on parsed PHP code and return the diagnostics. Creates a minimal `ProjectContext` (no file context).

```rust
use crate::analyzer::rules::test_utils::{parse_php, run_rule};

let parsed = parse_php(source);
let rule = MyNewRule::new();
let diagnostics = run_rule(&rule, &parsed);
```

#### `run_rule_with_context<R>(rule: &R, source: &str) -> Vec<Diagnostic>`

Run a rule with a context that includes the parsed file. Useful for rules that need to resolve symbols defined in the same file (e.g., checking if a function exists).

```rust
use crate::analyzer::rules::test_utils::run_rule_with_context;

let source = r#"<?php
function test(int $a, int $b) {}
test(1);  // Missing argument - needs context to check function signature
"#;

let rule = MissingArgumentRule::new();
let diagnostics = run_rule_with_context(&rule, source);
```

### Assertion Functions

#### `assert_no_diagnostics(diagnostics: &[Diagnostic])`

Assert that no diagnostics were produced. Provides helpful error output if diagnostics are found.

```rust
use crate::analyzer::rules::test_utils::{parse_php, run_rule, assert_no_diagnostics};

let parsed = parse_php(source);
let rule = MyNewRule::new();
let diagnostics = run_rule(&rule, &parsed);

assert_no_diagnostics(&diagnostics);
```

#### `assert_diagnostics(diagnostics: &[Diagnostic], expected_messages: &[&str])`

Assert that diagnostics match expected messages (case-insensitive substring match). Useful for checking that specific issues were detected without worrying about exact formatting.

```rust
use crate::analyzer::rules::test_utils::{parse_php, run_rule, assert_diagnostics};

let diagnostics = run_rule(&rule, &parsed);

assert_diagnostics(&diagnostics, &["inconsistent return type", "missing type hint"]);
```

#### `assert_diagnostics_exact(diagnostics: &[Diagnostic], expected_lines: &[&str])`

Assert that diagnostics match expected messages exactly in the format used in `.expect` files: `{severity}: {message}`. This is the most precise assertion and matches the format used by the integration test suite.

```rust
use crate::analyzer::rules::test_utils::{parse_php, run_rule, assert_diagnostics_exact};

let diagnostics = run_rule(&rule, &parsed);

assert_diagnostics_exact(
    &diagnostics, 
    &["error: inconsistent return type: expected int, found string at 3:9"]
);
```

**Note:** This function is flexible with line/column numbers - it checks that the core message matches even if line/column numbers differ slightly.

#### `assert_has_diagnostics(diagnostics: &[Diagnostic], context: &str)`

Assert that at least one diagnostic was produced, with helpful output if none are found. Useful for tests where you expect some diagnostics but want better error messages when none are produced.

```rust
use crate::analyzer::rules::test_utils::{parse_php, run_rule, assert_has_diagnostics};

let diagnostics = run_rule(&rule, &parsed);

assert_has_diagnostics(&diagnostics, "generic array type conflicts");
```

### Auto-Fix Testing Functions

#### `run_fix<R>(rule: &R, parsed: &ParsedSource) -> Vec<TextEdit>`

Run a rule's fix function on parsed PHP code and return the text edits.

```rust
use crate::analyzer::rules::test_utils::{parse_php, run_fix};

let parsed = parse_php(source);
let rule = MyNewRule::new();
let edits = run_fix(&rule, &parsed);
```

#### `run_fix_with_context<R>(rule: &R, source: &str) -> Vec<TextEdit>`

Run a rule's fix function with a context that includes the parsed file.

```rust
use crate::analyzer::rules::test_utils::run_fix_with_context;

let edits = run_fix_with_context(&rule, source);
```

#### `assert_fix<R>(rule: &R, parsed: &ParsedSource, input: &str, expected: &str)`

Assert that a rule's fix produces the expected output when applied to input source. This function:
1. Runs the rule's fix function
2. Applies the edits to the input
3. Compares the result with the expected output
4. Provides detailed diff output if they don't match

```rust
use crate::analyzer::rules::test_utils::{assert_fix, parse_php_with_path};

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
```

#### `assert_fix_with_path<R>(rule: &R, input: &str, expected: &str, path: &str)`

Convenience function that combines `parse_php_with_path` and `assert_fix`. Useful for rules that check the filename.

```rust
use crate::analyzer::rules::test_utils::assert_fix_with_path;

let rule = StrictTypesRule::new();
assert_fix_with_path(&rule, input, expected, "strict_missing.php");
```

#### `assert_fix_with_context<R>(rule: &R, input: &str, expected: &str)`

Assert that a rule's fix produces the expected output using a context that includes the parsed file.

```rust
use crate::analyzer::rules::test_utils::assert_fix_with_context;

let rule = UnusedUseRule::new();
assert_fix_with_context(&rule, input, expected);
```

### Complete Test Example

Here's a complete example showing how to use the test utilities:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::rules::test_utils::{
        assert_diagnostics_exact,
        assert_no_diagnostics,
        assert_fix,
        parse_php,
        parse_php_with_path,
        run_rule,
    };

    #[test]
    fn test_detects_issue() {
        let source = r#"<?php
function test() {
    return 1;
    return "string";
}
"#;

        let parsed = parse_php(source);
        let rule = ConsistentReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_diagnostics_exact(
            &diagnostics,
            &["error: inconsistent return type: expected int, found string at 4:5"]
        );
    }

    #[test]
    fn test_no_false_positives() {
        let source = r#"<?php
function test() {
    return 1;
    return 2;
}
"#;

        let parsed = parse_php(source);
        let rule = ConsistentReturnRule::new();
        let diagnostics = run_rule(&rule, &parsed);

        assert_no_diagnostics(&diagnostics);
    }

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

## AST Exploration and Debugging

Use the `dump-tree` binary to explore the AST structure and debug your rule implementation. This is especially helpful when working out how to handle specific code patterns or understanding the tree-sitter node structure.

You can use `dump-tree` in two ways:

### Using a File Path

```bash
# Dump AST for a PHP file
cargo run --bin dump-tree test_debug.php

# Or with a full path
cargo run --bin dump-tree tests/invalid/category/my_new_rule.php
```

### Using a Direct String (No File Needed)

This is especially useful for AI agents or quick debugging without creating temporary files:

```bash
# Pass PHP code directly as a string (single line)
cargo run --bin dump-tree -- --string "<?php function test() { return 42; }"

# Or without the flag (if the string doesn't match an existing file path)
cargo run --bin dump-tree "<?php function test() { return 42; }"

# Multi-line example (PowerShell)
cargo run --bin dump-tree -- --string @"
<?php
function test(bool \$flag) {
    if (\$flag) {
        return 42;
    }
    return "string";
}
"@

# Multi-line example (Bash/Unix)
cargo run --bin dump-tree -- --string '<?php
function test(bool $flag) {
    if ($flag) {
        return 42;
    }
    return "string";
}'
```

**Note:** If you use the `--string` flag (or `-s`), the input is always treated as code. Without the flag, `dump-tree` will check if the argument exists as a file path first, and if not, treat it as a string.

This will show you the tree-sitter node types and structure for your test cases, helping you:
- Understand which node types to look for in your visitor
- See the exact structure of the code you're analyzing
- Debug why your rule might not be matching certain patterns
- Identify the correct child nodes to access for specific information

**Tip:** For quick debugging, you can pass PHP code directly as a string without creating a file. This is perfect for exploring small code snippets during rule development.

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

- **`strict_typing/consistent_return.rs`** - Colocated tests with comprehensive test utilities usage
- **`control_flow/fallthrough.rs`** - Simple visitor pattern
- **`cleanup/unused_variable.rs`** - Complex analysis with auto-fix
- **`sanity/undefined_variable.rs`** - Basic AST traversal

## Testing Tips

1. **Start Simple** - Create minimal test cases first using `parse_php()` and `run_rule()`
2. **Edge Cases** - Test with various PHP syntax variations
3. **False Positives** - Use `assert_no_diagnostics()` to ensure your rule doesn't trigger on valid code
4. **Exact Assertions** - Use `assert_diagnostics_exact()` for precise message matching (matches `.expect` file format)
5. **Context-Aware Rules** - Use `run_rule_with_context()` for rules that need to resolve symbols in the same file
6. **Auto-Fix Testing** - Use `assert_fix()` or `assert_fix_with_path()` to test fix functionality
7. **Ignore Comments** - Rules automatically respect `php-checker-ignore` comments
8. **Performance** - Keep visitor logic efficient for large codebases
9. **Colocated Tests** - Keep tests in the same file as the rule for better maintainability

## Need Help?

- Check existing rules in the same category for patterns
- Look at `strict_typing/consistent_return.rs` for a complete example with colocated tests
- Review `test_utils.rs` for all available testing utilities
- Use `cargo run --bin dump-tree test_debug.php` to debug and understand AST structure
- Look at helper functions in `helpers.rs` for common operations
- Test incrementally as you build your rule
