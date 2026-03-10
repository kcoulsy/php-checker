# Rule Expansion Roadmap

This document lists the next set of static-analysis rules we want to add so the checker can start replacing PHPStan/Psalm (excluding PHPDoc checks for now). Each section describes a rule family and the concrete checks that would give similar coverage to the established PHP tools.

## Strict typing / conversions

- **Scalar compatibility enforcement**
  - Report assignments, return statements, and function arguments where the inferred or declared type differs from the actual value without an explicit cast (e.g., `string` passed into an `int` parameter, or `float` returned from a function declared to return `int`).
  - Track type coercions through expressions such as string concatenation, arithmetic, and comparison so mixed/string values don't accidentally pass into strictly typed APIs.
  - Combine with namespace/global function info so we can flag calls to standard PHP APIs when the arguments we pass don't match the signature we have recorded in the project context.

- **Mixed vs. specific scalar tracking**
  - Warn when `$mixed` values are used in contexts that require more specific types unless there is an explicit guard (e.g., calling methods on `$mixed` without an `is_string` check).
  - Surface cases where returning `mixed` from one function flows into a strictly typed consumer without any narrowing guard.

- **Scalar parameter enforcement**
  - Detect when literal values or constants clearly violate the declared signature (e.g., passing `true` into a `string` parameter) so callers know immediately which call-site needs fixing.
  - Track default values to ensure they match the parameter's declared type, warning when an incompatible default is silently accepted.

- **Return-type solidity**
  - Strengthen the missing-return checks by validating the type of each `return` expression against the declared return type, even through conditional chains, `match` arms, and `yield`.
  - Emit warnings when a union return type never fulfills one of its constituent types on any code path.

## Control-flow / type narrowing

- **Return-path completeness**
  - Expand the missing-return rule to differentiate between nullable/union return types and `yield`, ensuring that every declared type path terminates with an appropriate value.
  - Warn about `return` statements that never execute due to preceding guards.

## API misuse

- **Method/property existence**
  - Infer variable types and ensure that method or property accesses are valid on those types, resolving across namespaces and `use` statements when methods come from classes defined in other files.
  - Mark usages of non-existent methods (dynamic method calls can be approximated by checking literal names).

- **Array access safety**
  - Expand the existing array-key rule to warn whenever an array variable is accessed with a literal key that was never assigned, including multi-file scopes where the assignment happens elsewhere.
  - For dynamic keys, track preceding checks (e.g., `isset`) and only warn when no guard was seen.

- **Undefined symbol detection**
  - Report class/method references that do not resolve in the project context, including cases where PHP's autoloading would fail.
  - Flag `use` statements that alias symbols that don't exist in the resolved namespace set.

## Dependency / namespace correctness

- **Missing or misaligned dependencies**
  - When code references `Multi\Service\foo()`, ensure there's a corresponding namespace declaration or `use` that resolves it; otherwise, emit a missing-symbol diagnostic.
  - Detect when an alias hides multiple namespaces with the same short name (e.g., two `Service` namespaces aliased differently) and warn when that causes ambiguity.

These rules can be prioritized based on how much inference state we already maintain (namespaces/functions). Let me know which category you want to build first, and I can start implementing it.
