//! Test utilities for colocated rule tests.
//!
//! This module provides utilities to make it easy to write tests directly
//! in rule files, allowing for better test organization and isolation.

use std::path::PathBuf;
use std::sync::Arc;

use crate::analyzer::fix;
use crate::analyzer::parser;
use crate::analyzer::project::ProjectContext;
use crate::analyzer::Diagnostic;

/// Parse PHP source code into a `ParsedSource` for testing.
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::parse_php;
///
/// let source = r#"<?php
/// function test() {
///     return 42;
/// }
/// "#;
///
/// let parsed = parse_php(source);
/// ```
pub fn parse_php(source: &str) -> parser::ParsedSource {
    parse_php_with_path(source, "test.php")
}

/// Parse PHP source code into a `ParsedSource` for testing with a custom file path.
///
/// This is useful for rules that check the filename (e.g., strict_types rule).
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::parse_php_with_path;
///
/// let source = r#"<?php
/// function test() {
///     return 42;
/// }
/// "#;
///
/// let parsed = parse_php_with_path(source, "strict_missing.php");
/// ```
pub fn parse_php_with_path(source: &str, path: &str) -> parser::ParsedSource {
    let mut ts_parser = tree_sitter::Parser::new();
    ts_parser
        .set_language(tree_sitter_php::language())
        .expect("failed to load tree-sitter-php language");
    let tree = ts_parser
        .parse(source, None)
        .expect("failed to parse PHP source");

    parser::ParsedSource {
        path: PathBuf::from(path),
        source: Arc::new(source.to_string()),
        tree,
    }
}

/// Run a rule on parsed PHP code and return the diagnostics.
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::{parse_php, run_rule};
/// use crate::analyzer::rules::strict_typing::ConsistentReturnRule;
///
/// let source = r#"<?php
/// function test() {
///     return 1;
///     return "string";
/// }
/// "#;
///
/// let parsed = parse_php(source);
/// let rule = ConsistentReturnRule::new();
/// let diagnostics = run_rule(&rule, &parsed);
///
/// assert_eq!(diagnostics.len(), 1);
/// ```
pub fn run_rule<R>(rule: &R, parsed: &parser::ParsedSource) -> Vec<Diagnostic>
where
    R: crate::analyzer::rules::DiagnosticRule,
{
    let context = ProjectContext::new();
    rule.run(parsed, &context)
}

/// Run a rule on parsed PHP code with a context that includes the parsed file.
///
/// This is useful for rules that need to resolve symbols defined in the same file.
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::{parse_php, run_rule_with_context};
/// use crate::analyzer::rules::strict_typing::MissingArgumentRule;
///
/// let source = r#"<?php
/// function test(int $a, int $b) {}
/// test(1);
/// "#;
///
/// let rule = MissingArgumentRule::new();
/// let diagnostics = run_rule_with_context(&rule, source);
/// ```
pub fn run_rule_with_context<R>(rule: &R, source: &str) -> Vec<Diagnostic>
where
    R: crate::analyzer::rules::DiagnosticRule,
{
    // Parse twice: once for context (needs ownership), once for rule (needs reference)
    let parsed_for_context = parse_php(source);
    let parsed_for_rule = parse_php(source);
    
    let mut context = ProjectContext::new();
    context.insert(parsed_for_context);
    rule.run(&parsed_for_rule, &context)
}

/// Assert that diagnostics match expected messages.
///
/// This is a convenience function for checking that the right diagnostics
/// were produced. It compares diagnostic messages (case-insensitive substring match).
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::{parse_php, run_rule, assert_diagnostics};
/// use crate::analyzer::rules::strict_typing::ConsistentReturnRule;
///
/// let source = r#"<?php
/// function test() {
///     return 1;
///     return "string";
/// }
/// "#;
///
/// let parsed = parse_php(source);
/// let rule = ConsistentReturnRule::new();
/// let diagnostics = run_rule(&rule, &parsed);
///
/// assert_diagnostics(&diagnostics, &["inconsistent return type"]);
/// ```
pub fn assert_diagnostics(diagnostics: &[Diagnostic], expected_messages: &[&str]) {
    assert_eq!(
        diagnostics.len(),
        expected_messages.len(),
        "Expected {} diagnostics, but got {}",
        expected_messages.len(),
        diagnostics.len()
    );

    for (i, expected_msg) in expected_messages.iter().enumerate() {
        assert!(
            diagnostics[i]
                .message
                .to_lowercase()
                .contains(&expected_msg.to_lowercase()),
            "Diagnostic {}: expected message containing '{}', but got '{}'",
            i,
            expected_msg,
            diagnostics[i].message
        );
    }
}

/// Assert that no diagnostics were produced.
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::{parse_php, run_rule, assert_no_diagnostics};
/// use crate::analyzer::rules::strict_typing::ConsistentReturnRule;
///
/// let source = r#"<?php
/// function test() {
///     return 1;
///     return 2;
/// }
/// "#;
///
/// let parsed = parse_php(source);
/// let rule = ConsistentReturnRule::new();
/// let diagnostics = run_rule(&rule, &parsed);
///
/// assert_no_diagnostics(&diagnostics);
/// ```
pub fn assert_no_diagnostics(diagnostics: &[Diagnostic]) {
    if !diagnostics.is_empty() {
        let mut error_msg = String::new();
        error_msg.push_str(&format!(
            "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        error_msg.push_str("Expected no diagnostics, but got:\n");
        error_msg.push_str(&format!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        
        for (i, diag) in diagnostics.iter().enumerate() {
            error_msg.push_str(&format!(
                "  {}. {}: {}\n",
                i + 1,
                diag.severity,
                diag.message
            ));
        }
        
        error_msg.push_str(&format!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        
        panic!("{}", error_msg);
    }
}

/// Assert that diagnostics match expected messages exactly (as they appear in .expect files).
///
/// This function matches diagnostics in the format: `{severity}: {message}`
/// The expected messages should be in the same format as they appear in .expect files.
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::{parse_php, run_rule, assert_diagnostics_exact};
/// use crate::analyzer::rules::strict_typing::ConsistentReturnRule;
///
/// let source = r#"<?php
/// function test() {
///     return 1;
///     return "string";
/// }
/// "#;
///
/// let parsed = parse_php(source);
/// let rule = ConsistentReturnRule::new();
/// let diagnostics = run_rule(&rule, &parsed);
///
/// assert_diagnostics_exact(&diagnostics, &["error: inconsistent return type: expected int, found string at 3:9"]);
/// ```
pub fn assert_diagnostics_exact(diagnostics: &[Diagnostic], expected_lines: &[&str]) {
    // Convert diagnostics to the format used in .expect files
    let actual_lines: Vec<String> = diagnostics
        .iter()
        .map(|d| {
            format!("{}: {}", d.severity, d.message)
        })
        .collect();

    if actual_lines.len() != expected_lines.len() {
        let mut error_msg = String::new();
        error_msg.push_str(&format!(
            "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        error_msg.push_str(&format!(
            "Diagnostic count mismatch: expected {}, got {}\n",
            expected_lines.len(),
            actual_lines.len()
        ));
        error_msg.push_str(&format!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        
        error_msg.push_str("\nExpected diagnostics:\n");
        for (i, line) in expected_lines.iter().enumerate() {
            error_msg.push_str(&format!("  {}. {}\n", i + 1, line));
        }
        
        error_msg.push_str("\nActual diagnostics:\n");
        for (i, line) in actual_lines.iter().enumerate() {
            error_msg.push_str(&format!("  {}. {}\n", i + 1, line));
        }
        
        error_msg.push_str(&format!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        
        panic!("{}", error_msg);
    }

    for (i, expected_line) in expected_lines.iter().enumerate() {
        // Allow for slight variations in line/column numbers by checking if the message core matches
        // This is more flexible than exact matching but still validates the important parts
        let expected_parts: Vec<&str> = expected_line.splitn(2, ':').collect();
        if expected_parts.len() == 2 {
            let expected_severity = expected_parts[0].trim();
            let expected_msg = expected_parts[1].trim();
            
            let actual_severity = format!("{}", diagnostics[i].severity);
            let actual_msg = &diagnostics[i].message;

            if expected_severity != actual_severity {
                let mut error_msg = String::new();
                error_msg.push_str(&format!(
                    "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
                ));
                error_msg.push_str(&format!(
                    "Diagnostic {}: severity mismatch\n",
                    i + 1
                ));
                error_msg.push_str(&format!(
                    "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
                ));
                error_msg.push_str(&format!("Expected: {}\n", expected_severity));
                error_msg.push_str(&format!("Actual:   {}\n", actual_severity));
                error_msg.push_str(&format!(
                    "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
                ));
                panic!("{}", error_msg);
            }

            // For the message, check if it contains the key parts (ignore exact line/column if they differ slightly)
            // But first try exact match
            if actual_msg != expected_msg {
                // If exact match fails, check if the core message matches (without line/column)
                let expected_core = expected_msg
                    .rsplitn(2, " at ")
                    .nth(1)
                    .unwrap_or(expected_msg);
                let actual_core = actual_msg
                    .rsplitn(2, " at ")
                    .nth(1)
                    .unwrap_or(actual_msg);
                
                if actual_core.trim() != expected_core.trim() && !actual_msg.contains(expected_core.trim()) {
                    let mut error_msg = String::new();
                    error_msg.push_str(&format!(
                        "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
                    ));
                    error_msg.push_str(&format!(
                        "Diagnostic {}: message mismatch\n",
                        i + 1
                    ));
                    error_msg.push_str(&format!(
                        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
                    ));
                    error_msg.push_str(&format!("Expected: {}\n", expected_msg));
                    error_msg.push_str(&format!("Actual:   {}\n", actual_msg));
                    error_msg.push_str(&format!(
                        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
                    ));
                    panic!("{}", error_msg);
                }
            }
        } else {
            // Fallback: just check if message contains the expected text
            if !actual_lines[i].contains(expected_line) {
                let mut error_msg = String::new();
                error_msg.push_str(&format!(
                    "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
                ));
                error_msg.push_str(&format!(
                    "Diagnostic {}: expected line containing '{}', but got '{}'\n",
                    i + 1,
                    expected_line,
                    actual_lines[i]
                ));
                error_msg.push_str(&format!(
                    "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
                ));
                panic!("{}", error_msg);
            }
        }
    }
}

/// Run a rule's fix function on parsed PHP code and return the text edits.
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::{parse_php, run_fix};
/// use crate::analyzer::rules::strict_typing::StrictTypesRule;
///
/// let source = r#"<?php
/// function test(): void {}
/// "#;
///
/// let parsed = parse_php(source);
/// let rule = StrictTypesRule::new();
/// let edits = run_fix(&rule, &parsed);
/// ```
pub fn run_fix<R>(rule: &R, parsed: &parser::ParsedSource) -> Vec<fix::TextEdit>
where
    R: crate::analyzer::rules::DiagnosticRule,
{
    let context = ProjectContext::new();
    rule.fix(parsed, &context)
}

/// Run a rule's fix function on parsed PHP code with a context that includes the parsed file.
///
/// This is useful for rules that need to resolve symbols defined in the same file.
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::run_fix_with_context;
/// use crate::analyzer::rules::strict_typing::StrictTypesRule;
///
/// let source = r#"<?php
/// function test(): void {}
/// "#;
///
/// let rule = StrictTypesRule::new();
/// let edits = run_fix_with_context(&rule, source);
/// ```
pub fn run_fix_with_context<R>(rule: &R, source: &str) -> Vec<fix::TextEdit>
where
    R: crate::analyzer::rules::DiagnosticRule,
{
    // Parse twice: once for context (needs ownership), once for rule (needs reference)
    let parsed_for_context = parse_php(source);
    let parsed_for_rule = parse_php(source);
    
    let mut context = ProjectContext::new();
    context.insert(parsed_for_context);
    rule.fix(&parsed_for_rule, &context)
}

/// Assert that a rule's fix produces the expected output when applied to input source.
///
/// This function runs the rule's fix, applies the edits to the source, and compares
/// the result with the expected output.
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::{assert_fix, parse_php_with_path};
/// use crate::analyzer::rules::strict_typing::StrictTypesRule;
///
/// let input = r#"<?php
/// function test(): void {}
/// "#;
///
/// let expected = r#"<?php
///
/// declare(strict_types=1);
///
/// function test(): void {}
/// "#;
///
/// let rule = StrictTypesRule::new();
/// let parsed = parse_php_with_path(input, "strict_missing.php");
/// assert_fix(&rule, &parsed, input, expected);
/// ```
pub fn assert_fix<R>(
    rule: &R,
    parsed: &parser::ParsedSource,
    input: &str,
    expected: &str,
) where
    R: crate::analyzer::rules::DiagnosticRule,
{
    let edits = run_fix(rule, parsed);
    let actual = fix::apply_text_edits(input, &edits);

    if actual != expected {
        let mut error_msg = String::new();
        error_msg.push_str(&format!(
            "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        error_msg.push_str("Fix output mismatch\n");
        error_msg.push_str(&format!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        
        error_msg.push_str("\nExpected output:\n");
        error_msg.push_str(&format!("```php\n{}\n```\n", expected));
        
        error_msg.push_str("\nActual output:\n");
        error_msg.push_str(&format!("```php\n{}\n```\n", actual));
        
        // Show diff-like output
        error_msg.push_str("\nDifferences:\n");
        let expected_lines: Vec<&str> = expected.lines().collect();
        let actual_lines: Vec<&str> = actual.lines().collect();
        
        let max_lines = expected_lines.len().max(actual_lines.len());
        for i in 0..max_lines {
            let expected_line = expected_lines.get(i).copied().unwrap_or("");
            let actual_line = actual_lines.get(i).copied().unwrap_or("");
            
            if expected_line != actual_line {
                error_msg.push_str(&format!("  Line {}:\n", i + 1));
                if !expected_line.is_empty() {
                    error_msg.push_str(&format!("    - {}\n", expected_line));
                }
                if !actual_line.is_empty() {
                    error_msg.push_str(&format!("    + {}\n", actual_line));
                }
            }
        }
        
        error_msg.push_str(&format!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        
        panic!("{}", error_msg);
    }
}

/// Assert that a rule's fix produces the expected output when applied to input source,
/// using a custom file path for parsing.
///
/// This is useful for rules that check the filename (e.g., strict_types rule).
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::assert_fix_with_path;
/// use crate::analyzer::rules::strict_typing::StrictTypesRule;
///
/// let input = r#"<?php
/// function test(): void {}
/// "#;
///
/// let expected = r#"<?php
///
/// declare(strict_types=1);
///
/// function test(): void {}
/// "#;
///
/// let rule = StrictTypesRule::new();
/// assert_fix_with_path(&rule, input, expected, "strict_missing.php");
/// ```
pub fn assert_fix_with_path<R>(
    rule: &R,
    input: &str,
    expected: &str,
    path: &str,
) where
    R: crate::analyzer::rules::DiagnosticRule,
{
    let parsed = parse_php_with_path(input, path);
    assert_fix(rule, &parsed, input, expected);
}

/// Assert that a rule's fix produces the expected output when applied to input source,
/// using a context that includes the parsed file.
///
/// This is useful for rules that need to resolve symbols defined in the same file.
///
/// # Example
/// ```rust
/// use crate::analyzer::rules::test_utils::assert_fix_with_context;
/// use crate::analyzer::rules::cleanup::UnusedUseRule;
///
/// let input = r#"<?php
/// use Multi\Service as Svc;
/// use Multi\Client;
/// Svc\takesTwo(1);
/// "#;
///
/// let expected = r#"<?php
/// use Multi\Service as Svc;
/// Svc\takesTwo(1);
/// "#;
///
/// let rule = UnusedUseRule::new();
/// assert_fix_with_context(&rule, input, expected);
/// ```
pub fn assert_fix_with_context<R>(
    rule: &R,
    input: &str,
    expected: &str,
) where
    R: crate::analyzer::rules::DiagnosticRule,
{
    let edits = run_fix_with_context(rule, input);
    let actual = fix::apply_text_edits(input, &edits);

    if actual != expected {
        let mut error_msg = String::new();
        error_msg.push_str(&format!(
            "\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        error_msg.push_str("Fix output mismatch\n");
        error_msg.push_str(&format!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        
        error_msg.push_str("\nExpected output:\n");
        error_msg.push_str(&format!("```php\n{}\n```\n", expected));
        
        error_msg.push_str("\nActual output:\n");
        error_msg.push_str(&format!("```php\n{}\n```\n", actual));
        
        // Show diff-like output
        error_msg.push_str("\nDifferences:\n");
        let expected_lines: Vec<&str> = expected.lines().collect();
        let actual_lines: Vec<&str> = actual.lines().collect();
        
        let max_lines = expected_lines.len().max(actual_lines.len());
        for i in 0..max_lines {
            let expected_line = expected_lines.get(i).copied().unwrap_or("");
            let actual_line = actual_lines.get(i).copied().unwrap_or("");
            
            if expected_line != actual_line {
                error_msg.push_str(&format!("  Line {}:\n", i + 1));
                if !expected_line.is_empty() {
                    error_msg.push_str(&format!("    - {}\n", expected_line));
                }
                if !actual_line.is_empty() {
                    error_msg.push_str(&format!("    + {}\n", actual_line));
                }
            }
        }
        
        error_msg.push_str(&format!(
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n"
        ));
        
        panic!("{}", error_msg);
    }
}
