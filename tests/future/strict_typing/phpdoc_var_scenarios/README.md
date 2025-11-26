# PHPDoc @var Scenarios

This directory contains focused test scenarios for `@var` PHPDoc tag validation.

## Structure

Each scenario is a separate test file with:
- **Filename:** `NN_description.php` (e.g., `01_correct_property.php`)
- **Comment:** Top of file describes the scenario and expected outcome
- **Expected file:** `NN_description.expect` with expected diagnostic output

## Benefits of This Approach

### 1. **Focused Testing**
- Each file tests ONE specific scenario
- Easy to understand what's being tested
- Easy to debug when something fails

### 2. **Clear Organization**
- Numbered files run in logical order
- Descriptive names make purpose obvious
- Related scenarios grouped together

### 3. **Better Error Messages**
When a test fails, you see:
```
FAILED: tests/future/strict_typing/phpdoc_var_scenarios/04_wrong_inline_var.php

Expected diagnostics:
   1. error: type mismatch: assigning int to string variable at 6:14

Actual diagnostics:
   (none)
```
**Clear:** The filename tells you exactly which scenario failed!

### 4. **Easy to Add New Scenarios**
Just create two files:
```bash
07_new_scenario.php
07_new_scenario.expect
```

### 5. **Selective Testing**
Can test individual scenarios:
```bash
cargo run --bin php-checker -- analyse tests/future/.../04_wrong_inline_var.php
```

## Scenario Naming Convention

```
NN_descriptive_name.php
```

Where:
- `NN` = Two-digit number (01, 02, 03...)
- `descriptive_name` = Clear description of what's being tested
- Use underscores for spaces
- Keep names concise but clear

## Adding a New Scenario

1. **Pick the next number** (e.g., next available is 07)
2. **Create PHP file** with scenario code
3. **Add comment** at top explaining scenario
4. **Create .expect file** with expected diagnostics (or empty if none)
5. **Update this README** with scenario description

## Current Scenarios

| # | File | Description | Expected |
|---|------|-------------|----------|
| 01 | correct_property.php | Property with correct @var type | ✓ No errors |
| 02 | wrong_property_type.php | Property assigned wrong type vs @var | ✗ Type mismatch error |
| 03 | inline_var_cast.php | Inline @var type casting | ✓ Valid narrowing |
| 04 | wrong_inline_var.php | Inline @var claims wrong type | ✗ Type mismatch error |
| 05 | reassignment_violation.php | Variable reassigned to incompatible type | ✗ Reassignment error |
| 06 | generic_array.php | @var with generic array type | ✓ Correct User[] usage |

## Planned Additional Scenarios

- [ ] 07 - Wrong array element type vs @var
- [ ] 08 - Associative array with key/value types
- [ ] 09 - Wrong key/value type in associative array
- [ ] 10 - Union type variables
- [ ] 11 - Nullable type variables
- [ ] 12 - Property type changes in methods
- [ ] 13 - Static properties with @var
- [ ] 14 - Nested array types
- [ ] 15 - Type narrowing with @var

## Comparison: Monolithic vs Scenario-Based

### Monolithic Test (Old Approach)
```
tests/invalid/strict_typing/phpdoc_var.php          (200+ lines)
tests/invalid/strict_typing/phpdoc_var.expect       (20+ errors)
```
**Issues:**
- Hard to understand which scenario failed
- All scenarios mixed together
- Difficult to add new scenarios
- Errors blend together

### Scenario-Based Tests (New Approach)
```
tests/future/strict_typing/phpdoc_var_scenarios/
├── README.md
├── 01_correct_property.php
├── 01_correct_property.expect
├── 02_wrong_property_type.php
├── 02_wrong_property_type.expect
├── 03_inline_var_cast.php
├── 03_inline_var_cast.expect
└── ...
```
**Benefits:**
- Crystal clear which scenario failed
- Each scenario self-contained
- Easy to add new scenarios
- Better organized and documented

## Usage

### Run All Scenarios (When Moved to Active Tests)
```bash
cargo test invalid_fixtures_match_expectations
```

### Run Single Scenario
```bash
cargo run --bin php-checker -- analyse tests/future/.../04_wrong_inline_var.php
```

### Move to Active Suite
When ready to implement @var support:
```bash
# Move entire directory
mv tests/future/strict_typing/phpdoc_var_scenarios/ tests/invalid/strict_typing/

# Or move individual scenarios
mv tests/future/.../01_correct_property.* tests/invalid/strict_typing/phpdoc_var_scenarios/
```

## Notes

- Empty `.expect` files = no errors expected (valid code)
- Keep scenarios focused - one thing per file
- Add comments explaining what's being tested
- Update README when adding scenarios
