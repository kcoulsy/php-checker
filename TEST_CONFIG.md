# Test Configuration

This document explains how to configure test files to control which rules are executed during analysis.

## Overview

Test configuration allows you to add special comments at the top of PHP files to control which analyzer rules run. This is particularly useful for:

- Testing specific rules in isolation
- Creating focused test scenarios
- Skipping rules that generate noise in test files
- Debugging individual rule behavior

## Syntax

Test configuration directives are PHP comments with a special prefix: `// php-checker-test:`

Two directives are supported:

### only-rules

Run **only** the specified rules, ignoring all others.

```php
<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

// Only the phpdoc_var_check rule will run on this file
```

Multiple rules can be specified with comma separation:

```php
<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check,strict_typing/phpdoc_param_check

// Only phpdoc_var_check and phpdoc_param_check will run
```

### skip-rules

Skip the specified rules, run all others normally.

```php
<?php
// php-checker-test: skip-rules=sanity/undefined_variable

// All rules except undefined_variable will run
```

Multiple rules:

```php
<?php
// php-checker-test: skip-rules=sanity/undefined_variable,cleanup/unused_variable

// All rules except undefined_variable and unused_variable will run
```

## Rule Names

Rule names follow the format: `category/rule_name`

Available categories:
- `sanity/` - Basic sanity checks (undefined variables, duplicate declarations)
- `strict_typing/` - Type checking and PHPDoc validation
- `control_flow/` - Control flow analysis (unreachable code, fallthrough)
- `cleanup/` - Unused code detection
- `security/` - Security issue detection
- `api/` - API usage validation
- `psr4/` - PSR-4 namespace compliance

Example rule names:
- `strict_typing/phpdoc_var_check`
- `strict_typing/phpdoc_param_check`
- `sanity/undefined_variable`
- `cleanup/unused_variable`
- `security/hard_coded_credentials`

## Directive Placement

Test configuration directives must appear in the **first 20 lines** of the file. They are typically placed at the very top, before the opening `<?php` tag or immediately after it.

```php
<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check
// php-checker-test: skip-rules=sanity/undefined_variable

// Your PHP code here
```

## Combining Directives

- **only-rules takes precedence**: If `only-rules` is specified, `skip-rules` has no effect
- **Multiple skip-rules accumulate**: You can have multiple `skip-rules` directives and they'll all be honored

```php
<?php
// php-checker-test: skip-rules=sanity/undefined_variable
// php-checker-test: skip-rules=cleanup/unused_variable

// Both rules will be skipped
```

## Examples

### Example 1: Test Only PHPDoc @var Validation

```php
<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check

class Example {
    /** @var string */
    private $name = 123;  // Error: type mismatch

    private $test = $undefinedVar;  // No error - undefined_variable rule not running
}
```

**Output:**
```
error [strict_typing/phpdoc_var_check]: @var type 'string' conflicts with assigned value type 'int'
```

### Example 2: Skip Undefined Variable Checks

```php
<?php
// php-checker-test: skip-rules=sanity/undefined_variable

class Example {
    /** @var string */
    private $name = 123;  // Error: type mismatch

    private $test = $undefinedVar;  // No error - undefined_variable skipped
}
```

**Output:**
```
error [strict_typing/phpdoc_var_check]: @var type 'string' conflicts with assigned value type 'int'
```

### Example 3: Test Multiple PHPDoc Rules

```php
<?php
// php-checker-test: only-rules=strict_typing/phpdoc_var_check,strict_typing/phpdoc_param_check

class Example {
    /** @var string */
    private $name = 123;  // Error: @var mismatch

    /**
     * @param string $value
     */
    public function test(int $value) {}  // Error: @param conflict
}
```

**Output:**
```
error [strict_typing/phpdoc_var_check]: @var type 'string' conflicts with assigned value type 'int'
error [strict_typing/phpdoc_param_check]: @param type 'string' conflicts with native type hint 'int'
```

## When Test Config is Active

Test configuration is only active when the file contains at least one test directive. Files without any `// php-checker-test:` comments run all enabled rules normally.

## Implementation Details

- Directives are parsed from the source code before analysis begins
- Parsing checks only the first 20 lines for performance
- Rule filtering happens in the `collect_diagnostics` method
- The feature is implemented in `src/analyzer/test_config.rs`

## See Also

- [PHPDOC_PROGRESS.md](PHPDOC_PROGRESS.md) - PHPDoc implementation status
- [TEST_ORGANIZATION.md](TEST_ORGANIZATION.md) - Test file organization guide
- [ADD_RULE.md](ADD_RULE.md) - How to add new analyzer rules
