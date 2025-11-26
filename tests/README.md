# Test Fixtures for PHP Checker

This directory holds the PHP snippets that the checker will eventually evaluate.

### Coverage notes
- `invalid/redundant_condition.php` — duplicate guard expressions.
- `invalid/unused_variable.php` — assignments that never get used.
- `invalid/duplicate_declaration.php` — conflicting function names.
- `invalid/missing_argument.php` — calls that omit required parameters.
- `invalid/impossible_comparison.php` — comparisons that are always false due to known types.
- `invalid/array_key_not_defined.php` — array keys read before being written.

## Structure
- `valid/` — examples that should pass the analyzer.
- `invalid/` — cases that should trigger diagnostics (type errors, undefined names, etc.).

Each file is intentionally minimal to make it easy to shadow or extend with new cases.

- Fixtures can include a `.expect.fixed` sibling that holds the content we expect after running `php-checker --fix --dry-run`. The `fix_suite.rs` test compares that file against the fix engine output to guard automatic edits.

