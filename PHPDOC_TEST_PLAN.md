# PHPDoc Test Plan

This document outlines comprehensive test cases for PHPDoc static analysis support.

## Test Structure

Each test file should cover multiple scenarios for a specific PHPDoc feature:
- **Valid usage** - Correct PHPDoc that matches actual code
- **Type mismatches** - PHPDoc types that don't match actual values
- **Missing PHPDocs** - Required PHPDocs that are absent
- **Reassignment** - Variable type changes after PHPDoc declaration
- **Return type verification** - Checking returns match PHPDoc
- **Parameter verification** - Checking arguments match PHPDoc

## Phase 1: Core PHPDoc Tags

### Test: `strict_typing/phpdoc_param.php`
Test `@param` type checking for function/method parameters.

**Scenarios:**
1. ✓ Correct `@param` matching native type hint
2. ✗ `@param` contradicts native type hint (int vs string)
3. ✓ `@param` adds detail to native array (array vs int[])
4. ✗ Function call with wrong type (based on `@param`)
5. ✓ Multiple params with different types
6. ✗ Variadic params with wrong type (`@param string ...$items` but pass int)
7. ✓ Union types in `@param` (`@param int|string $value`)
8. ✗ Passing type not in union
9. ✓ Nullable types (`@param ?string $value`)
10. ✗ Passing null to non-nullable param

### Test: `strict_typing/phpdoc_return.php`
Test `@return` type checking for function/method returns.

**Scenarios:**
1. ✓ Correct `@return` matching native return type
2. ✗ `@return` contradicts native type (int vs string)
3. ✗ Function returns value not matching `@return` type
4. ✗ Multiple return paths with inconsistent types
5. ✓ `@return void` with no return statement
6. ✗ `@return void` but returns value
7. ✓ `@return static` from method
8. ✗ `@return static` but returns parent class
9. ✓ `@return $this` from method
10. ✗ `@return $this` but returns new instance

### Test: `strict_typing/phpdoc_var.php`
Test `@var` for variable and property type declarations.

**Scenarios:**
1. ✓ Property with correct `@var` type
2. ✗ Property assigned wrong type vs `@var`
3. ✓ Inline `@var` type casting in assignment
4. ✗ Inline `@var` claims wrong type
5. ✗ Variable reassigned to incompatible type after `@var`
6. ✓ `@var` with generic array type (`@var User[]`)
7. ✗ Array element wrong type vs `@var User[]`
8. ✓ `@var` with associative array (`@var array<string, int>`)
9. ✗ Wrong key/value type in associative array

### Test: `strict_typing/phpdoc_throws.php`
Test `@throws` exception documentation validation.

**Scenarios:**
1. ✓ Function with `@throws` actually throws that exception
2. ✗ Function with `@throws` never throws (dead documentation)
3. ✗ Function throws exception not in `@throws`
4. ✓ Multiple `@throws` tags for different exceptions
5. ✗ Try-catch not handling documented `@throws` exception
6. ✓ `@throws` inherited from interface/parent class

## Phase 2: Property & Magic PHPDocs

### Test: `strict_typing/phpdoc_property.php`
Test `@property`, `@property-read`, `@property-write` magic properties.

**Scenarios:**
1. ✓ Class with `@property` accessed correctly
2. ✗ `@property` accessed with wrong type
3. ✓ `@property-read` only read from
4. ✗ `@property-read` assigned to (should error)
5. ✓ `@property-write` only written to
6. ✗ `@property-write` read from (should error)
7. ✓ Magic `__get` with `@property` type
8. ✗ `@property` type doesn't match `__get` return
9. ✓ Override parent property type with `@property`
10. ✗ Child `@property` type incompatible with parent

### Test: `strict_typing/phpdoc_method.php`
Test `@method` magic method declarations.

**Scenarios:**
1. ✓ Class with `@method` called correctly
2. ✗ `@method` called with wrong argument types
3. ✗ `@method` return type used incorrectly
4. ✓ `@method static` called statically
5. ✗ `@method static` called on instance
6. ✓ `@method` with optional parameters
7. ✗ Missing required argument to `@method`
8. ✓ Magic `__call` with `@method` signature

## Phase 3: Array & Iterable Types

### Test: `strict_typing/phpdoc_array_shapes.php`
Test array shape syntax (advanced arrays).

**Scenarios:**
1. ✓ Simple array shape (`@param array{id: int, name: string}`)
2. ✗ Missing required key in array shape
3. ✗ Wrong type for array shape key
4. ✓ Optional keys in array shape (`array{id: int, name?: string}`)
5. ✓ Nested array shapes
6. ✗ Nested array shape type mismatch
7. ✓ Array shape with mixed types
8. ✓ Array shape in return type

### Test: `strict_typing/phpdoc_iterables.php`
Test iterable type syntax variations.

**Scenarios:**
1. ✓ Generic array `Type[]` syntax
2. ✓ Array template `array<Type>` syntax
3. ✓ Keyed array `array<int, User>` syntax
4. ✗ Wrong key type in keyed array
5. ✗ Wrong value type in keyed array
6. ✓ `iterable<Type>` vs `array<Type>`
7. ✗ Non-iterable passed to `iterable` param
8. ✓ Generator return with `iterable<Type>`

## Phase 4: Generics

### Test: `strict_typing/phpdoc_template_basic.php`
Test basic `@template` generic types.

**Scenarios:**
1. ✓ Simple generic class `@template T`
2. ✓ Generic method using class template
3. ✗ Return type doesn't match template constraint
4. ✓ Multiple template parameters `@template T, U`
5. ✗ Wrong type passed for template parameter
6. ✓ Template with default type `@template T = string`
7. ✓ Template type inference from constructor

### Test: `strict_typing/phpdoc_template_extends.php`
Test `@extends`, `@implements`, `@use` for generics.

**Scenarios:**
1. ✓ Child class `@extends Parent<int>` specifies type
2. ✗ Child class `@extends` with incompatible type
3. ✓ Interface `@implements Foo<string>`
4. ✗ Interface implementation missing generic type
5. ✓ Trait `@use Helper<User>`
6. ✗ Trait generic type mismatch with usage
7. ✓ Chained generics `@extends Parent<Collection<T>>`

### Test: `strict_typing/phpdoc_template_variance.php`
Test covariance and contravariance in generics.

**Scenarios:**
1. ✓ `@template-covariant T` allows subtype returns
2. ✗ `@template-covariant` with contravariant usage
3. ✓ `@template-contravariant T` allows supertype params
4. ✗ `@template-contravariant` with covariant usage
5. ✓ Mixed variance in same class

## Phase 5: Type Assertions & Narrowing

### Test: `strict_typing/phpdoc_assert.php`
Test `@phpstan-assert` type narrowing.

**Scenarios:**
1. ✓ Function with `@phpstan-assert string $value` narrows type
2. ✓ After assert call, type is narrowed correctly
3. ✗ Using narrowed type before assert call
4. ✓ `@phpstan-assert-if-true` conditionally narrows
5. ✓ `@phpstan-assert-if-false` conditionally narrows
6. ✗ Type not narrowed when condition is opposite
7. ✓ Multiple assertions in same function
8. ✓ Negative assertions `@phpstan-assert !null`

### Test: `strict_typing/phpdoc_param_out.php`
Test `@param-out` for reference parameters.

**Scenarios:**
1. ✓ Function with `@param-out int $result` changes type
2. ✓ After call, referenced variable has new type
3. ✗ Using variable with old type after `@param-out` call
4. ✓ Multiple `@param-out` parameters
5. ✗ `@param-out` on non-reference parameter
6. ✓ `@param-out` changes mixed to specific type

### Test: `strict_typing/phpdoc_self_out.php`
Test `@phpstan-self-out` and `@phpstan-this-out`.

**Scenarios:**
1. ✓ Method with `@phpstan-self-out self<T|U>` changes type
2. ✓ After method call, object type is updated
3. ✗ Using old type after `@phpstan-self-out` method
4. ✓ `@phpstan-this-out` vs `@phpstan-self-out` difference
5. ✓ Mutable collection adding type to generic
6. ✗ Wrong type after mutation method

## Phase 6: Callables

### Test: `strict_typing/phpdoc_callable_signatures.php`
Test callable signature syntax in PHPDocs.

**Scenarios:**
1. ✓ Simple callable `@param callable(int): string`
2. ✗ Passing callable with wrong signature
3. ✓ Multiple callable parameters
4. ✗ Missing parameter in callable signature
5. ✓ Callable with optional parameters
6. ✓ Variadic callable parameters

### Test: `strict_typing/phpdoc_callable_invocation.php`
Test callable invocation timing tags.

**Scenarios:**
1. ✓ `@param-immediately-invoked-callable` for functions
2. ✓ `@param-later-invoked-callable` for methods
3. ✗ Variable escapes scope before later-invoked-callable runs
4. ✓ `@param-closure-this` changes $this inside closure
5. ✗ Using wrong $this type in closure

## Phase 7: Mixins & Inheritance

### Test: `strict_typing/phpdoc_mixin.php`
Test `@mixin` for delegation patterns.

**Scenarios:**
1. ✓ Class with `@mixin Delegate` can call delegate methods
2. ✗ Calling non-existent method on mixin
3. ✗ Wrong parameter types when calling mixin method
4. ✓ Generic mixin `@mixin T` with template
5. ✓ Multiple `@mixin` tags on same class
6. ✗ Mixin method conflicts with existing method

### Test: `strict_typing/phpdoc_require_extends.php`
Test `@phpstan-require-extends` for interfaces/traits.

**Scenarios:**
1. ✓ Interface with `@phpstan-require-extends Base` and implementing class extends Base
2. ✗ Class implements interface but doesn't extend required base
3. ✓ Trait with `@phpstan-require-extends Base`
4. ✗ Class uses trait but doesn't extend required base

### Test: `strict_typing/phpdoc_require_implements.php`
Test `@phpstan-require-implements` for traits.

**Scenarios:**
1. ✓ Trait with `@phpstan-require-implements Iface` and class implements it
2. ✗ Class uses trait but doesn't implement required interface
3. ✓ Multiple required interfaces
4. ✗ Missing one of multiple required interfaces

## Phase 8: Metadata & Modifiers

### Test: `strict_typing/phpdoc_deprecated.php`
Test `@deprecated` tag validation.

**Scenarios:**
1. ✓ Deprecated class marked correctly
2. ✗ Using deprecated class (should warn)
3. ✗ Using deprecated method (should warn)
4. ✓ `@deprecated` with description message
5. ✓ Child class inherits `@deprecated` from parent
6. ✓ Child class breaks inheritance with `@not-deprecated`
7. ✗ Using deprecated inherited method

### Test: `strict_typing/phpdoc_internal.php`
Test `@internal` tag validation (PHPStan 2.1.13+).

**Scenarios:**
1. ✓ `@internal` class used within same top namespace
2. ✗ `@internal` class used outside top namespace
3. ✗ `@internal` method called externally
4. ✗ `@internal` function called from different namespace
5. ✓ `@internal` used within same library

### Test: `strict_typing/phpdoc_readonly.php`
Test `@readonly` and `@immutable` tags.

**Scenarios:**
1. ✓ `@readonly` property not modified in methods
2. ✗ `@readonly` property assigned outside constructor
3. ✓ `@immutable` class - no properties modified
4. ✗ `@immutable` class but property assigned in method
5. ✓ Native readonly vs `@readonly` PHPDoc

### Test: `strict_typing/phpdoc_pure_impure.php`
Test `@phpstan-pure` and `@phpstan-impure` function tags.

**Scenarios:**
1. ✓ `@phpstan-pure` function returns same value on subsequent calls
2. ✗ `@phpstan-pure` function has side effects (file I/O, etc.)
3. ✓ `@phpstan-impure` function may return different values
4. ✓ Type narrowing with pure functions cached
5. ✗ Type narrowing with impure functions not cached

### Test: `strict_typing/phpdoc_sealed.php`
Test `@phpstan-sealed` tag (PHPStan 2.1.18+).

**Scenarios:**
1. ✓ `@phpstan-sealed Foo|Bar` allows Foo and Bar to extend
2. ✗ Class not in sealed list tries to extend
3. ✗ Interface not in sealed list tries to implement
4. ✓ Sealed with single allowed class

## Phase 9: Edge Cases & Combinations

### Test: `strict_typing/phpdoc_prefixed_tags.php`
Test `@phpstan-` prefixed tags.

**Scenarios:**
1. ✓ `@phpstan-param` overrides regular `@param`
2. ✓ `@phpstan-return` with advanced syntax
3. ✓ Regular `@param` ignored when `@phpstan-param` exists
4. ✓ IDE-friendly tags coexist with `@phpstan-*` tags

### Test: `strict_typing/phpdoc_php_internal_types.php`
Test classes named like PHP internal types.

**Scenarios:**
1. ✓ Custom `Resource` class with fully qualified name
2. ✗ Ambiguous `Resource` reference (should error)
3. ✓ Fully qualified `\My\Resource` in PHPDoc
4. ✓ Custom `Mixed`, `Number`, `Double` classes

### Test: `strict_typing/phpdoc_native_combination.php`
Test PHPDocs augmenting native typehints.

**Scenarios:**
1. ✓ Native `array` with PHPDoc `@param User[]`
2. ✗ PHPDoc contradicts native hint (should error)
3. ✓ Native `self` with PHPDoc `@return static`
4. ✓ Native hint is less specific, PHPDoc narrows it
5. ✗ Native hint is more specific, PHPDoc widens it (should warn)

### Test: `strict_typing/phpdoc_union_intersection.php`
Test union and intersection types in PHPDocs.

**Scenarios:**
1. ✓ Union type `@param int|string $value`
2. ✗ Value not in union type passed
3. ✓ Intersection type `@param Countable&Traversable $value`
4. ✗ Object missing one interface in intersection
5. ✓ Complex unions with null `@param int|string|null`
6. ✓ DNF types (Disjunctive Normal Form)

### Test: `strict_typing/phpdoc_reassignment.php`
Test type tracking through reassignments.

**Scenarios:**
1. ✓ Variable with `@var string` keeps type until reassigned
2. ✗ Reassignment to incompatible type (should error/warn)
3. ✓ Type narrowed by conditional, then reassigned
4. ✗ Using narrowed type after widening reassignment
5. ✓ Multiple reassignments in different scopes
6. ✓ Reassignment inside loop maintains type

## Test File Naming Convention

```
tests/invalid/strict_typing/phpdoc_<feature>.php
tests/invalid/strict_typing/phpdoc_<feature>.expect
```

For valid cases that should pass:
```
tests/valid/strict_typing/phpdoc_<feature>_valid.php
```

## Expected Output Format

Each `.expect` file should list diagnostics in format:
```
error: <message> at <line>:<column>
warning: <message> at <line>:<column>
```

## Implementation Priority

1. **Phase 1** (Core Tags) - Foundation for all PHPDoc checking
2. **Phase 3** (Arrays) - High value for catching array-related bugs
3. **Phase 2** (Properties) - Important for OOP codebases
4. **Phase 5** (Assertions) - Powerful for flow-sensitive analysis
5. **Phase 4** (Generics) - Complex but essential for frameworks
6. **Phase 8** (Metadata) - Code quality and documentation
7. **Phase 6** (Callables) - Specialized use cases
8. **Phase 7** (Mixins) - Advanced patterns
9. **Phase 9** (Edge Cases) - Polish and completeness
