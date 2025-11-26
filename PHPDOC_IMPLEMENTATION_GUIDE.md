# PHPDoc Implementation Guide

This guide provides a roadmap for implementing comprehensive PHPDoc static analysis in php-checker.

## What I've Created

### 1. Test Plan Document
**File:** `PHPDOC_TEST_PLAN.md`

A comprehensive test plan covering all PHPDoc features from the PHPStan documentation, organized into 9 phases with 30+ test scenarios.

### 2. Phase 1 Test Files (Core PHPDoc Tags)

I've created 4 complete test files with expected output for the most important PHPDoc features:

#### `tests/invalid/strict_typing/phpdoc_param.php` (+ .expect)
Tests `@param` type checking including:
- Basic type matching
- Type contradictions with native hints
- Union types
- Nullable types
- Array generic types (`int[]`, `array<string, int>`)
- Variadic parameters
- Object/interface types
- 20+ test scenarios

#### `tests/invalid/strict_typing/phpdoc_return.php` (+ .expect)
Tests `@return` type checking including:
- Return type matching
- Type contradictions
- Multiple return paths
- `void` functions
- `@return static` vs `@return $this`
- Array return types
- Union and nullable returns
- Missing return statements
- 25+ test scenarios

#### `tests/invalid/strict_typing/phpdoc_var.php` (+ .expect)
Tests `@var` type declarations including:
- Property type declarations
- Inline variable type casting
- Type reassignments
- Generic arrays (`User[]`, `array<string, int>`)
- Union and nullable types
- Static properties
- Constants
- Array destructuring
- Reference variables
- 30+ test scenarios

#### `tests/invalid/strict_typing/phpdoc_throws.php` (+ .expect)
Tests `@throws` exception documentation including:
- Exception throwing validation
- Dead documentation (never throws)
- Undocumented exceptions
- Multiple throws
- Try-catch handling
- Exception inheritance
- Constructor/destructor exceptions
- Exception hierarchy
- 20+ test scenarios

## Current State of Your Analyzer

**PHPDoc Support:** ❌ None currently implemented

**What Exists:**
- Tree-sitter-php captures PHPDoc comments as `comment` nodes ✓
- Basic type system (`TypeHint` enum: Int, String, Bool, Unknown)
- Function signature collection
- Type mismatch detection for literals
- Helper functions for AST traversal

**What's Missing:**
- PHPDoc comment parser
- PHPDoc type system (arrays, unions, generics, etc.)
- PHPDoc-aware type checking
- Integration with existing type rules

## Implementation Roadmap

### Step 1: Build PHPDoc Parser Module
**Estimated Effort:** Medium-Large

Create `src/analyzer/phpdoc/mod.rs` with:

```rust
pub struct PhpDocComment {
    pub params: Vec<ParamTag>,
    pub return_type: Option<ReturnTag>,
    pub var_type: Option<VarTag>,
    pub throws: Vec<ThrowsTag>,
    pub properties: Vec<PropertyTag>,
    pub methods: Vec<MethodTag>,
    // ... other tags
}

pub struct ParamTag {
    pub name: String,
    pub type_expr: TypeExpression,
}

pub enum TypeExpression {
    Simple(String),                     // int, string, User
    Array(Box<TypeExpression>),         // int[], User[]
    Generic(String, Vec<TypeExpression>), // array<string, int>
    Union(Vec<TypeExpression>),         // int|string|null
    Nullable(Box<TypeExpression>),      // ?string
    // ... more variants
}
```

**Parser Requirements:**
- Extract PHPDoc from `comment` nodes
- Parse tag lines (`@param`, `@return`, etc.)
- Parse type expressions with proper grammar
- Handle multi-line PHPDocs
- Support `@phpstan-*` prefixed variants

### Step 2: Extend Type System
**Estimated Effort:** Medium

Expand `helpers.rs` type system:

```rust
pub enum TypeHint {
    Int,
    String,
    Bool,
    Float,
    Array(Box<TypeHint>),                    // New
    AssocArray(Box<TypeHint>, Box<TypeHint>), // New: array<K, V>
    Union(Vec<TypeHint>),                     // New
    Nullable(Box<TypeHint>),                  // New
    Object(String),                           // New: class/interface names
    Callable(Vec<TypeHint>, Box<TypeHint>),   // New: callable signature
    Mixed,                                    // New
    Void,                                     // New
    Never,                                    // New
    Unknown,
}
```

### Step 3: Create PHPDoc Extraction Rule
**Estimated Effort:** Small-Medium

Create `src/analyzer/rules/strict_typing/phpdoc_extractor.rs`:

- Walk AST and collect PHPDoc comments
- Associate comments with following nodes (functions, classes, properties)
- Build context of all PHPDoc information
- Store in `ProjectContext` for other rules to use

### Step 4: Implement Core PHPDoc Rules
**Estimated Effort:** Large

Create new rules in `src/analyzer/rules/strict_typing/`:

#### `phpdoc_param_check.rs`
- Verify `@param` types match function signatures
- Check function calls against `@param` types
- Validate variadic parameters
- **Test with:** `tests/invalid/strict_typing/phpdoc_param.php`

#### `phpdoc_return_check.rs`
- Verify `@return` types match return statements
- Check all return paths
- Validate `static` vs `$this` returns
- **Test with:** `tests/invalid/strict_typing/phpdoc_return.php`

#### `phpdoc_var_check.rs`
- Verify property initialization matches `@var`
- Track variable type through reassignments
- Validate array element types
- **Test with:** `tests/invalid/strict_typing/phpdoc_var.php`

#### `phpdoc_throws_check.rs`
- Verify documented exceptions are actually thrown
- Warn about undocumented exceptions
- Check exception handling coverage
- **Test with:** `tests/invalid/strict_typing/phpdoc_throws.php`

### Step 5: Register Rules
**Estimated Effort:** Small

Update `src/analyzer/rules/mod.rs` and `src/analyzer.rs`:

```rust
pub use strict_typing::{
    // ... existing rules
    PhpDocParamCheckRule,
    PhpDocReturnCheckRule,
    PhpDocVarCheckRule,
    PhpDocThrowsCheckRule,
};
```

### Step 6: Test and Iterate
**Estimated Effort:** Medium

```bash
# Run tests
cargo test

# Test specific file
cargo run --bin php-checker -- analyse tests/invalid/strict_typing/phpdoc_param.php

# Update expectations if needed
cargo test -- --ignored
```

## Future Phases

Once Phase 1 (Core Tags) is working, implement:

### Phase 2: Property & Magic PHPDocs
- `@property`, `@property-read`, `@property-write`
- `@method` magic methods
- **Test files needed** (not created yet)

### Phase 3: Array & Iterable Types
- Array shapes: `array{id: int, name: string}`
- Complex iterables
- **Test files needed** (not created yet)

### Phase 4: Generics
- `@template` basic generics
- `@extends`, `@implements`, `@use`
- Variance (`@template-covariant`, etc.)
- **Test files needed** (not created yet)

### Phase 5: Type Assertions
- `@phpstan-assert` and variants
- `@param-out` for references
- `@phpstan-self-out` / `@phpstan-this-out`
- **Test files needed** (not created yet)

### Phase 6-9: Advanced Features
See `PHPDOC_TEST_PLAN.md` for complete breakdown.

## Key Implementation Challenges

### 1. Type Expression Parsing
PHPDoc types are complex:
```php
@param array<string, array<int, User|null>> $data
@param callable(int, string): bool $callback
@param array{id: int, name?: string} $shape
```

**Solution:** Write a proper parser or use a parsing library (nom, pest).

### 2. Type Compatibility Checking
Need to implement subtyping rules:
- `int` ⊆ `int|string` (int is subset of union)
- `?string` ≡ `string|null` (nullable equals union with null)
- `Child` ⊆ `Parent` (subclass is compatible)
- `int[]` ⊆ `array` (generic array is subset of array)

### 3. Context Propagation
PHPDocs affect type information across:
- Function calls (parameter types)
- Return statements (return type checking)
- Variable assignments (type tracking)
- Property access (magic properties)

**Solution:** Enhance `ProjectContext` with PHPDoc information, create type inference system.

### 4. Comment Association
Tree-sitter provides comments as separate nodes. Need to:
- Find comment before a function/class/property
- Handle multiple comments
- Skip regular comments, find PHPDoc comments
- Deal with whitespace and formatting

**Solution:** Look for `comment` nodes immediately preceding declarations.

## Testing Strategy

### For Each Rule:
1. Run test file: `cargo run --bin php-checker -- analyse tests/invalid/strict_typing/phpdoc_<feature>.php`
2. Compare output with `.expect` file
3. Iterate until all scenarios pass

### Test Coverage Goals:
- ✓ Valid PHPDocs that should pass
- ✗ Type mismatches detected
- ✗ Contradictions between PHPDoc and native hints
- ✗ Missing PHPDocs where required
- ✗ Reassignments violating PHPDoc types

## Example Implementation Snippet

Here's a skeleton for the PHPDoc param checker:

```rust
// src/analyzer/rules/strict_typing/phpdoc_param_check.rs

use super::DiagnosticRule;
use super::helpers::diagnostic_for_node;
use crate::analyzer::phpdoc::{PhpDocParser, TypeExpression};
use crate::analyzer::project::ProjectContext;
use crate::analyzer::{Severity, parser};

pub struct PhpDocParamCheckRule;

impl DiagnosticRule for PhpDocParamCheckRule {
    fn name(&self) -> &str {
        "strict_typing/phpdoc_param_check"
    }

    fn run(
        &self,
        parsed: &parser::ParsedSource,
        context: &ProjectContext,
    ) -> Vec<crate::analyzer::Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut cursor = parsed.tree.root_node().walk();

        // Find all function definitions
        walk_functions(parsed.tree.root_node(), &mut |func_node| {
            // Look for PHPDoc comment before function
            if let Some(comment) = find_preceding_comment(func_node, parsed) {
                if let Some(phpdoc) = PhpDocParser::parse(&comment) {
                    // Check @param tags against function signature
                    for param_tag in phpdoc.params {
                        // Validate parameter type...
                        if let Some(diag) = check_param_type(
                            param_tag,
                            func_node,
                            parsed
                        ) {
                            diagnostics.push(diag);
                        }
                    }

                    // Check function calls to this function
                    // Validate argument types against @param...
                }
            }
        });

        diagnostics
    }
}
```

## Quick Start Commands

```bash
# 1. See current test structure
ls tests/invalid/strict_typing/phpdoc_*

# 2. Examine a test file
cat tests/invalid/strict_typing/phpdoc_param.php

# 3. View expected output
cat tests/invalid/strict_typing/phpdoc_param.expect

# 4. Run current analyzer (will not detect PHPDoc issues yet)
cargo run --bin php-checker -- analyse tests/invalid/strict_typing/phpdoc_param.php

# 5. Start implementing PHPDoc parser
# Create: src/analyzer/phpdoc/mod.rs
# Then implement parser logic
```

## Success Criteria

Your analyzer successfully handles PHPDocs when:

1. ✅ All 4 Phase 1 test files pass their `.expect` assertions
2. ✅ Type mismatches in PHPDocs are caught (contradictions, wrong types)
3. ✅ Function calls are validated against `@param` types
4. ✅ Return statements are validated against `@return` types
5. ✅ Property/variable assignments are validated against `@var` types
6. ✅ Exception handling is validated against `@throws` tags
7. ✅ Native type hints and PHPDocs work together correctly
8. ✅ Union types, nullable types, and array generics are supported

## Resources

- **PHPStan Documentation:** https://phpstan.org/writing-php-code/phpdocs-basics
- **Tree-sitter PHP Grammar:** Check comment node structure
- **Your Test Files:** `tests/invalid/strict_typing/phpdoc_*.php`
- **Test Plan:** `PHPDOC_TEST_PLAN.md` (9 phases, 30+ scenarios)

## Next Steps

1. **Start with PHPDoc Parser:** Build the foundation to parse PHPDoc comments
2. **Extend Type System:** Add support for complex types (arrays, unions, etc.)
3. **Implement `@param` Rule:** Easiest to start with, most valuable
4. **Test Incrementally:** Run tests after each feature addition
5. **Iterate Through Phase 1:** Complete all 4 core tag rules
6. **Move to Phase 2+:** Build on the foundation with advanced features

Good luck! The test files I created cover realistic scenarios you'll encounter in production PHP code. They should guide your implementation and validate correctness.
