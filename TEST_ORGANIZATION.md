# Test Organization Guide

This guide explains the improved test organization with scenario-based testing.

## Overview

Tests in php-checker use two complementary approaches:

1. **Single-file tests** - For simple, focused rules
2. **Scenario-based tests** - For complex features with many edge cases

## Directory Structure

```
tests/
├── invalid/                    # Tests that should produce errors
│   ├── api/
│   ├── cleanup/
│   ├── control_flow/
│   ├── sanity/
│   ├── security/
│   └── strict_typing/
│
├── valid/                      # Tests that should produce NO errors
│   └── ...
│
├── fix/                        # Tests for auto-fix functionality
│   └── ...
│
└── future/                     # Tests for unimplemented features
    ├── README.md
    └── strict_typing/
        ├── phpdoc_var_scenarios/    # ← Scenario-based tests
        │   ├── README.md
        │   ├── 01_correct_property.php
        │   ├── 01_correct_property.expect
        │   ├── 02_wrong_property_type.php
        │   ├── 02_wrong_property_type.expect
        │   └── ...
        ├── phpdoc_param.php             # ← Monolithic test (old style)
        └── phpdoc_param.expect
```

## Test Approaches

### Approach 1: Single-File Tests

**When to use:**
- Simple rules with few scenarios
- Features with limited edge cases
- Quick validation of specific behavior

**Example:**
```
tests/invalid/cleanup/unused_variable.php
tests/invalid/cleanup/unused_variable.expect
```

**Structure:**
```php
<?php
// tests/invalid/cleanup/unused_variable.php

// Test case 1
$unused1 = 1;

// Test case 2
function foo() {
    $unused2 = 2;
}
```

```
# tests/invalid/cleanup/unused_variable.expect
error: unused variable $unused1
error: unused variable $unused2
```

**Pros:**
- Simple and straightforward
- Quick to create
- Good for basic rules

**Cons:**
- Hard to isolate which scenario failed
- All scenarios mixed together
- Difficult to add new scenarios
- Error messages less clear

### Approach 2: Scenario-Based Tests

**When to use:**
- Complex features with many scenarios
- Features with subtle edge cases
- PHPDoc validation
- Type system features
- Features that need clear documentation

**Example:**
```
tests/future/strict_typing/phpdoc_var_scenarios/
├── README.md
├── 01_correct_property.php
├── 01_correct_property.expect
├── 02_wrong_property_type.php
├── 02_wrong_property_type.expect
└── ...
```

**Structure:**
```php
<?php
// tests/future/.../02_wrong_property_type.php
// Scenario: Property assigned wrong type vs @var
// Expected: Error on line 8

class WrongPropertyType {
    /** @var string */
    private $name = 123;  // Error: int assigned to string
}
```

```
# tests/future/.../02_wrong_property_type.expect
error: type mismatch: assigning int to string property at 8:21
```

**Pros:**
- ✓ Crystal clear which scenario failed
- ✓ Each scenario self-contained and documented
- ✓ Easy to add new scenarios
- ✓ Better test failure messages
- ✓ Can test scenarios individually
- ✓ Doubles as documentation

**Cons:**
- More files to manage
- Requires more upfront planning

## When to Use Each Approach

### Use Single-File Tests For:
- ✓ Simple rules (e.g., unused variables, duplicate declarations)
- ✓ Features with < 5 test cases
- ✓ Quick validation
- ✓ Existing simple tests

### Use Scenario-Based Tests For:
- ✓ PHPDoc validation (many tags, many edge cases)
- ✓ Type system features (unions, generics, narrowing)
- ✓ Complex control flow analysis
- ✓ Features with > 10 test cases
- ✓ Features that need good documentation

## Creating Scenario-Based Tests

### 1. Create a scenarios directory
```bash
mkdir -p tests/future/strict_typing/phpdoc_param_scenarios
```

### 2. Create README.md
Document the scenarios, naming convention, and purpose.

### 3. Create numbered scenario files
```
01_basic_type_check.php
01_basic_type_check.expect
02_union_types.php
02_union_types.expect
03_nullable_types.php
03_nullable_types.expect
```

### 4. Add scenario header comments
```php
<?php
// Scenario: Basic @param type checking
// Expected: No errors for correct types
```

### 5. Keep scenarios focused
Each file should test ONE specific thing.

## Test Output with Scenarios

### Failed Monolithic Test:
```
FAILED: tests/invalid/strict_typing/phpdoc_var.php

Expected diagnostics:
   1. error: type mismatch at 16:21
   2. error: type mismatch at 30:14
   ... (20 more errors)

Actual diagnostics:
   1. error: unused variable $input
```
**Problem:** Which of the 20+ scenarios failed? Hard to tell!

### Failed Scenario Test:
```
FAILED: tests/.../phpdoc_var_scenarios/02_wrong_property_type.php

Expected diagnostics:
   1. error: type mismatch: assigning int to string property at 8:21

Actual diagnostics:
   (none)
```
**Clear:** Scenario 02 (wrong property type) failed. Easy to debug!

## Best Practices

### Naming Scenarios

**Good names:**
```
01_correct_usage.php
02_type_mismatch_int_to_string.php
03_nullable_parameter.php
04_union_type_violation.php
```

**Bad names:**
```
test1.php
foo.php
check_stuff.php
```

### Organizing Scenarios

**Logical order:**
1. Valid cases first (01-05)
2. Simple error cases (06-10)
3. Complex error cases (11-15)
4. Edge cases (16-20)

**Group related scenarios:**
```
01-05: Basic types
06-10: Array types
11-15: Union types
16-20: Generic types
```

### Writing Scenario Comments

```php
<?php
// Scenario: [Brief description]
// Expected: [What should happen]
// Related: [Links to other scenarios if applicable]
```

### Documenting in README

Keep the scenarios README.md up to date with:
- List of all scenarios
- Description of each
- Which are implemented
- Which are planned

## Migration Guide

### Converting Monolithic Test to Scenarios

**Before:**
```
tests/invalid/strict_typing/phpdoc_var.php          (200 lines)
tests/invalid/strict_typing/phpdoc_var.expect       (20 errors)
```

**After:**
```
tests/invalid/strict_typing/phpdoc_var_scenarios/
├── README.md
├── 01_scenario.php
├── 01_scenario.expect
├── 02_scenario.php
├── 02_scenario.expect
└── ... (15 more scenarios)
```

**Steps:**
1. Create scenarios directory
2. Extract each test case into separate file
3. Number files logically
4. Add descriptive names
5. Add scenario comments
6. Create README documenting all scenarios
7. Delete original monolithic file

## Example: PHPDoc @var Scenarios

See [tests/future/strict_typing/phpdoc_var_scenarios/](tests/future/strict_typing/phpdoc_var_scenarios/) for a complete example of scenario-based testing.

**Includes:**
- 6 initial scenarios
- Clear README documentation
- Focused test files
- Descriptive naming
- Scenario comments

## Running Scenario Tests

### Run all scenarios in a directory:
```bash
cargo test invalid_fixtures_match_expectations
```

### Run single scenario:
```bash
cargo run --bin php-checker -- analyse tests/.../02_wrong_property_type.php
```

### Check specific scenario expectations:
```bash
cat tests/.../02_wrong_property_type.expect
```

## Benefits Summary

| Aspect | Single-File | Scenario-Based |
|--------|------------|----------------|
| **Clarity** | Moderate | Excellent |
| **Debuggability** | Difficult | Easy |
| **Documentation** | Minimal | Built-in |
| **Maintainability** | Hard | Easy |
| **Setup Effort** | Low | Medium |
| **Best For** | Simple rules | Complex features |

## Recommendation

**New features with > 10 test cases:** Use scenario-based testing

**Simple rules with < 5 test cases:** Use single-file testing

**Complex type system features:** Definitely use scenario-based testing

## Real-World Example

### PHPDoc @param Testing

**Old monolithic approach:**
- 1 file with 20+ test cases
- Hard to identify failures
- Difficult to maintain

**New scenario approach:**
- 20 separate scenario files
- Each clearly documented
- Failures immediately obvious
- Easy to add new scenarios
- Self-documenting

**Result:** Much easier to implement and maintain!

## Future Direction

Consider creating scenario directories for:
- [ ] Type narrowing scenarios
- [ ] Generics scenarios
- [ ] Exception handling scenarios
- [ ] Control flow scenarios
- [ ] Security rule scenarios

Each with clear documentation and focused test cases.
