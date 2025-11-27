# PHPDoc Test Coverage

This document tracks the test scenarios created for PHPDoc validation rules.

## Test Structure

All PHPDoc tests are located in `tests/future/strict_typing/` and organized by tag type:
- `phpdoc_param_scenarios/` - Tests for `@param` tag validation
- `phpdoc_return_scenarios/` - Tests for `@return` tag validation
- `phpdoc_var_scenarios/` - Tests for `@var` tag validation

## Coverage Summary

### @param Tag Tests (7 scenarios)

| Test File | Type Feature | Expected Result |
|-----------|-------------|----------------|
| `03_param_object_conflict.php` | Object type conflict | Error |
| `04_param_object_matches.php` | Object type matches | Pass |
| `05_param_nullable_matches.php` | Nullable type matches (`?string`) | Pass |
| `06_param_nullable_conflict.php` | Nullable vs non-nullable conflict | Error |
| `07_param_non_nullable_conflict.php` | Non-nullable vs nullable conflict | Error |
| `08_param_union_matches.php` | Union type matches (`int\|string`) | Pass |
| `09_param_union_conflict.php` | Union type conflict | Error |

**Type Coverage:**
- ✅ Simple types (int, string, bool, float)
- ✅ Object types (User, Admin, etc.)
- ✅ Nullable types (?string)
- ✅ Union types (int|string)
- ❌ Array types (int[], array<K,V>)
- ❌ Generic types (@template)

### @return Tag Tests (8 scenarios)

| Test File | Type Feature | Expected Result |
|-----------|-------------|----------------|
| `01_return_type_conflict.php` | Basic type conflict | Error |
| `02_return_type_matches.php` | Basic type matches | Pass |
| `03_return_object_conflict.php` | Object type conflict | Error |
| `04_return_nullable_matches.php` | Nullable type matches (`?User`) | Pass |
| `05_return_nullable_conflict.php` | Nullable type conflict | Error |
| `06_return_non_nullable_conflict.php` | Non-nullable vs nullable conflict | Error |
| `07_return_union_matches.php` | Union type matches (`int\|string`) | Pass |
| `08_return_union_conflict.php` | Union type conflict | Error |

**Type Coverage:**
- ✅ Simple types (int, string, bool, float)
- ✅ Object types (User, Admin, etc.)
- ✅ Nullable types (?string)
- ✅ Union types (int|string)
- ❌ Array types (int[], array<K,V>)
- ❌ Generic types (@template)
- ❌ Return value validation (checking actual return statements)

### @var Tag Tests (10 scenarios)

| Test File | Type Feature | Expected Result |
|-----------|-------------|----------------|
| `00_test_config_example.php` | Test config demonstration | N/A |
| `00_test_config_skip_example.php` | Test config skip demonstration | N/A |
| `01_correct_property.php` | Basic property type matches | Pass |
| `02_wrong_property_type.php` | Property type conflict | Error |
| `03_inline_var_cast.php` | Inline @var type casting | Future |
| `04_wrong_inline_var.php` | Wrong inline @var type | Future |
| `05_reassignment_violation.php` | Type violation on reassignment | Future |
| `06_generic_array.php` | Generic array types | Future |
| `07_var_union_matches.php` | Union type matches (`int\|string`) | ✅ Pass |
| `08_var_union_conflict.php` | Union type conflict | Error |

**Type Coverage:**
- ✅ Simple types (int, string, bool, float)
- ✅ Object types (User, Admin, etc.)
- ✅ Nullable types (?string)
- ✅ Union types (int|string) - **FULLY WORKING** with compatibility checking
- ❌ Array types (int[], array<K,V>)
- ❌ Generic types (@template)
- ❌ Inline @var in functions
- ❌ Variable reassignment tracking

## Type Feature Matrix

| Type Feature | Parsing | @param | @return | @var | Notes |
|--------------|---------|--------|---------|------|-------|
| Simple (int, string) | ✅ | ✅ | ✅ | ✅ | Fully implemented |
| Object (User) | ✅ | ✅ | ✅ | ✅ | Fully implemented |
| Nullable (?string) | ✅ | ✅ | ✅ | ✅ | Fully implemented |
| Union (int\|string) | ✅ | ✅ | ✅ | ✅ | ✅ **COMPLETE** with compatibility checking |
| Native Union (int\|bool) | ✅ | N/A | ✅ | N/A | PHP 8.0+ union types supported |
| Array (int[]) | ✅ | ❌ | ❌ | ❌ | Parsed but not validated |
| Generic (array<K,V>) | ✅ | ❌ | ❌ | ❌ | Parsed but not validated |
| Template (@template T) | ❌ | ❌ | ❌ | ❌ | Not implemented |

## Test Scenarios by Implementation Status

### ✅ Currently Working (17 scenarios)

**@param:**
- Object type matching and conflicts (2)
- Nullable type matching and conflicts (3)
- Union type matching and conflicts (2)

**@return:**
- Basic type matching and conflicts (2)
- Object type conflicts (1)
- Nullable type matching and conflicts (3)
- Union type matching and conflicts (2)
- Native union type matching and conflicts (2)

**@var:**
- Basic property type matching and conflicts (2)
- Union type matching (1) ✅ **NEW**
- Union type conflicts (1)

### ⏳ Future Implementation (7 scenarios)

**@var:**
- Inline @var type casting (2)
- Variable reassignment tracking (1)
- Generic array validation (1)

## Next Steps

### High Priority

1. ✅ **Union Type Compatibility Checking** - **COMPLETE**
   - ✅ Implement: `int` should be compatible with `int|string`
   - ✅ Update: All three validation rules to support subset checking
   - ✅ Impact: Makes union types fully functional
   - ✅ Added: `is_type_compatible()` helper function in helpers.rs
   - ✅ Added: Native PHP 8.0+ union type support in @return rule

2. **Array Element Validation**
   - Implement: `User[]` validation
   - Implement: `array<string, int>` validation
   - Add: Test scenarios for array types

3. **Return Value Validation**
   - Implement: Check actual return statements match `@return` type
   - Handle: Multiple return paths
   - Support: `void`, `static`, `$this`

### Medium Priority

4. **Inline @var Support**
   - Implement: `/** @var Type $var */` inside functions
   - Support: Type narrowing/casting

5. **Variable Reassignment Tracking**
   - Implement: Track type through reassignments
   - Detect: Type violations after `@var` declaration

### Lower Priority

6. **@throws Validation**
7. **@property Magic Properties**
8. **@method Magic Methods**
9. **Generic Types (@template)**

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test suite
cargo test invalid_suite
cargo test valid_suite

# Analyze a specific test file
cargo run --bin php-checker -- analyse tests/future/strict_typing/phpdoc_param_scenarios/08_param_union_matches.php
```

## Test File Naming Convention

```
tests/future/strict_typing/phpdoc_<tag>_scenarios/
  ├── NN_<tag>_<feature>_<result>.php
  └── NN_<tag>_<feature>_<result>.expect
```

Where:
- `NN` = Sequential number (00-99)
- `<tag>` = param, return, or var
- `<feature>` = Type feature being tested (nullable, union, object, etc.)
- `<result>` = matches, conflict, or descriptive name

## Test Configuration Directives

Tests can use special comments to control which rules run:

```php
// php-checker-test: only-rules=strict_typing/phpdoc_param_check
// php-checker-test: skip-rules=strict_typing/phpdoc_var_check
```

This allows focused testing of specific validation rules.
