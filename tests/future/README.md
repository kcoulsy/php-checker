# Future Test Cases

This directory contains test cases for features that are **planned but not yet implemented** in php-checker.

## Purpose

These tests serve as:
1. **Specification** - Define expected behavior for future features
2. **Test-Driven Development** - Write tests first, then implement to pass them
3. **Documentation** - Show what features are coming and how they should work
4. **Validation** - Ensure implementation matches expectations when features are built

## How to Use These Tests

### When Implementing a Feature

1. **Move the test** from `tests/future/` to `tests/invalid/` or `tests/valid/`
2. **Run the test** to see it fail (expected)
3. **Implement the feature** to make the test pass
4. **Verify** all scenarios in the test file work correctly

Example:
```bash
# Move PHPDoc param test to invalid suite
mv tests/future/strict_typing/phpdoc_param.php tests/invalid/strict_typing/
mv tests/future/strict_typing/phpdoc_param.expect tests/invalid/strict_typing/

# Run tests (will fail initially)
cargo test invalid_fixtures_match_expectations

# Implement the feature...

# Run tests again (should pass)
cargo test invalid_fixtures_match_expectations
```

### Test Organization

Tests are organized by category matching the main test structure:
- `strict_typing/` - Type checking and PHPDoc features
- `control_flow/` - Control flow analysis features
- `security/` - Security analysis features
- etc.

## Current Future Tests

### PHPDoc Support (Phase 1 - Core Tags)

**Status:** Not implemented
**Priority:** High
**Effort:** Large

Tests for PHPDoc static analysis based on PHPStan/Psalm standards:

#### `strict_typing/phpdoc_param.php` (+ .expect)
- `@param` type checking for function/method parameters
- 20+ scenarios covering basic types, unions, nullables, arrays, etc.
- **Blocks:** Type mismatch detection, function call validation

#### `strict_typing/phpdoc_return.php` (+ .expect)
- `@return` type checking for function/method returns
- 25+ scenarios covering return types, `static`, `$this`, multiple paths
- **Blocks:** Return type validation, missing return detection

#### `strict_typing/phpdoc_var.php` (+ .expect)
- `@var` type declarations for variables and properties
- 30+ scenarios covering properties, inline casting, reassignments, arrays
- **Blocks:** Property type checking, variable type tracking

#### `strict_typing/phpdoc_throws.php` (+ .expect)
- `@throws` exception documentation validation
- 20+ scenarios covering exception throwing, handling, inheritance
- **Blocks:** Exception safety analysis, dead documentation detection

### Implementation Guidance

See [PHPDOC_IMPLEMENTATION_GUIDE.md](../../PHPDOC_IMPLEMENTATION_GUIDE.md) for:
- Step-by-step implementation roadmap
- Architecture decisions
- Type system design
- Integration with existing rules

See [PHPDOC_TEST_PLAN.md](../../PHPDOC_TEST_PLAN.md) for:
- Complete test plan (9 phases, 30+ features)
- Additional test scenarios to create
- Future phases beyond Phase 1

## Adding New Future Tests

When adding new future tests:

1. **Create comprehensive scenarios** - Cover valid cases, edge cases, and error cases
2. **Write .expect files** - Define exact expected diagnostic output
3. **Document the feature** - Add to this README with status and priority
4. **Link to implementation plan** - Reference design docs or specifications
5. **Organize by category** - Place in appropriate subdirectory

## Moving Tests to Active Suite

Before moving a future test to the active suite:

- [ ] Feature is fully implemented
- [ ] All test scenarios in the file should pass
- [ ] Edge cases are handled
- [ ] Documentation is updated
- [ ] The test follows existing test patterns

## Notes

- Future tests are **NOT** run by `cargo test` automatically
- They exist purely for planning and future development
- Don't create future tests for trivial features - just implement them
- Use future tests for complex features that need careful planning
