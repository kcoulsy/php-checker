# Rule Expansion Roadmap

This document lists the next set of static-analysis rules we want to add so the checker can start replacing PHPStan/Psalm (excluding PHPDoc checks for now). Each section describes a rule family and the concrete checks that would give similar coverage to the established PHP tools.

## Strict typing / conversions

- **Scalar compatibility enforcement**
  - Report assignments, return statements, and function arguments where the inferred or declared type differs from the actual value without an explicit cast (e.g., `string` passed into an `int` parameter, or `float` returned from a function declared to return `int`).
  - Track type coercions through expressions such as string concatenation, arithmetic, and comparison so mixed/string values don’t accidentally pass into strictly typed APIs.
  - Combine with namespace/global function info so we can flag calls to standard PHP APIs when the arguments we pass don’t match the signature we have recorded in the project context.

- **Mixed vs. specific scalar tracking**
  - Warn when `$mixed` values are used in contexts that require more specific types unless there is an explicit guard (e.g., calling methods on `$mixed` without an `is_string` check).
  - Surface cases where returning `mixed` from one function flows into a strictly typed consumer without any narrowing guard.

- **Scalar parameter enforcement**
  - Ensure scalar-typed parameters (`int`, `string`, `float`, `bool`) are never called with incompatible values such as arrays or objects, and that `null` is only passed when the signature allows nullable types.
  - Detect when literal values or constants clearly violate the declared signature (e.g., passing `true` into a `string` parameter) so callers know immediately which call-site needs fixing.
  - Track default values to ensure they match the parameter’s declared type, warning when an incompatible default is silently accepted.

- **Return-type solidity**
  - Strengthen the missing-return checks by validating the type of each `return` expression against the declared return type, even through conditional chains, `match` arms, and `yield`.
  - Emit warnings when a function declared to return `void` returns a value, or when a union return type never fulfills one of its constituent types on any code path.

- **Arithmetic/type conversion guards**
  - Flag arithmetic and bitwise expressions that mix scalars with incompatible types (e.g., adding a string to an array) without casting.
  - Watch for dynamic conversions where the operand’s inferred type isn’t compatible with the operation, warning especially inside `declare(strict_types=1)` files where PHP would otherwise throw a `TypeError`.

## Control-flow / type narrowing

- **Guard inference**
  - Observe branch conditions (`is_string`, `instanceof`, `null` checks, etc.) and keep track of the narrowed type for the sibling block to flag incompatible usages later in the same function.
  - Detect cases where the guard proves a variable is always `null` or `false`, and a subsequent branch assumes otherwise.

- **Unreachable / redundant blocks**
  - Extend unreachable detection to whole branches, not just statements after `return`. If all `if`/`elseif` combinations are covered, warn about the redundant `else`.
  - Spot duplicated `case` labels or identical `if` conditions within the same scope and emit a warning.

- **Return-path completeness**
  - Expand the missing-return rule to differentiate between nullable/union return types and `yield`, ensuring that every declared type path terminates with an appropriate value.
  - Warn about `return` statements that never execute due to preceding guards.

- **Switch/match exhaustiveness**
  - Verify that `switch`/`match` statements handle every case implied by the selector (especially enums or literal sets) and warn about missing `default`/catch-all branches.
  - Flag `break`/`return` statements that are impossible because a previous `case` already handles every condition.

## API misuse

- **Method/property existence**
  - Infer variable types and ensure that method or property accesses are valid on those types, resolving across namespaces and `use` statements when methods come from classes defined in other files.
  - Mark usages of non-existent methods (dynamic method calls can be approximated by checking literal names).

- **Array access safety**
  - Expand the existing array-key rule to warn whenever an array variable is accessed with a literal key that was never assigned, including multi-file scopes where the assignment happens elsewhere.
  - For dynamic keys, track preceding checks (e.g., `isset`) and only warn when no guard was seen.

- **Undefined symbol detection**
  - Report function/class/method references that do not resolve in the project context, including cases where PHP’s autoloading would fail.
  - Flag `use` statements that alias symbols that don’t exist in the resolved namespace set.

## Dead code / duplicates

- **Duplicate declarations**
  - Look for duplicate class/interface/trait names across the workspace and warn about redeclarations similar to Psalm/PHPStan.
  - Report duplicate constants or functions even when they only exist in included files.

- **Switch/case anomalies**
  - Warn about `switch` statements where cases fall through without an explicit comment, and about repeated literal cases.
  - Detect `match` arms that mirror previous arms without differences.

## Security / validation

- **Cryptographic misuse**
  - Warn when weak hashing functions (`md5`, `sha1`) are used for password hashing or signing without additional stretching/salting.
  - Detect cases where encryption keys are hard-coded or reused across multiple files/contexts.

## Best practices

- **Unused imports / symbols**
  - Track `use` statements and warn about aliases that are never resolved, or conversely about symbols referenced without a `use` that could be statically resolved.

- **Deprecation / upgrade assistance**
  - Warn when APIs flagged for deprecation are used, enabling teams to track on the fly which parts of the codebase need updates.
  - Identify functions or constructs that have been removed in upcoming PHP versions (e.g., `each`, `create_function`) and flag their use.

- **Coding style / readability**
  - Detect extremely long chains of method/property access (`$foo->bar()->baz()->qux()`) that may need refactoring or breaking into smaller helpers.
  - Warn when nested loops exceed a configurable depth, encouraging simpler control flow.

## Dependency / namespace correctness

- **Missing or misaligned dependencies**
  - When code references `Multi\Service\foo()`, ensure there's a corresponding namespace declaration or `use` that resolves it; otherwise, emit a missing-symbol diagnostic.
  - Detect when an alias hides multiple namespaces with the same short name (e.g., two `Service` namespaces aliased differently) and warn when that causes ambiguity.

- **Namespace organization**
  - Flag namespace definitions that don’t match the expected directory structure if the project uses PSR-4 conventions (optional future step).

These rules can be prioritized based on how much inference state we already maintain (namespaces/functions). Let me know which category you want to build first, and I can start implementing it. 

