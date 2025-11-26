# Test Fixtures for PHP Checker

This directory holds the PHP snippets that the checker will eventually evaluate.

## Structure
- `valid/` — examples that should pass the analyzer.
- `invalid/` — cases that should trigger diagnostics (type errors, undefined names, etc.).

Each file is intentionally minimal to make it easy to shadow or extend with new cases.

