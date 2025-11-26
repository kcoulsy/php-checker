<?php

// Scenario 1: ✓ Correct @param matching native type hint
/**
 * @param int $value
 */
function correctParam(int $value): void {}
correctParam(42);

// Scenario 2: ✗ @param contradicts native type hint
/**
 * @param string $value  // PHPDoc says string
 */
function contradictoryParam(int $value): void {}  // But native hint says int

// Scenario 3: ✓ @param adds detail to native array
/**
 * @param int[] $numbers
 */
function detailedArray(array $numbers): void {}
detailedArray([1, 2, 3]);

// Scenario 4: ✗ Function call with wrong type (based on @param)
/**
 * @param string $name
 */
function expectsString(string $name): void {}
expectsString(123);  // Error: passing int to string param

// Scenario 5: ✓ Multiple params with different types
/**
 * @param int $id
 * @param string $name
 * @param bool $active
 */
function multipleParams(int $id, string $name, bool $active): void {}
multipleParams(1, "test", true);

// Scenario 6: ✗ Variadic params with wrong type
/**
 * @param string ...$items
 */
function variadicStrings(string ...$items): void {}
variadicStrings("a", "b", 123);  // Error: 123 is int, not string

// Scenario 7: ✓ Union types in @param
/**
 * @param int|string $value
 */
function unionParam($value): void {}
unionParam(42);
unionParam("test");

// Scenario 8: ✗ Passing type not in union
/**
 * @param int|string $value
 */
function strictUnion($value): void {}
strictUnion(true);  // Error: bool not in int|string union

// Scenario 9: ✓ Nullable types
/**
 * @param ?string $optional
 */
function nullableParam(?string $optional): void {}
nullableParam(null);
nullableParam("test");

// Scenario 10: ✗ Passing null to non-nullable param
/**
 * @param string $required
 */
function nonNullableParam(string $required): void {}
nonNullableParam(null);  // Error: null not allowed

// Additional: Array type with wrong element type
/**
 * @param User[] $users
 */
function expectsUserArray(array $users): void {}
expectsUserArray([1, 2, 3]);  // Error: int[] instead of User[]

// Additional: Associative array type mismatch
/**
 * @param array<string, int> $scores
 */
function expectsScores(array $scores): void {}
expectsScores([1 => "wrong"]);  // Error: int key, string value (should be string key, int value)

// Additional: Complex nested types
/**
 * @param array<string, array<int, User>> $nested
 */
function complexNested(array $nested): void {}

// Additional: Callable type in param
/**
 * @param callable(int): string $callback
 */
function withCallback(callable $callback): void {}
withCallback(function(int $x): string { return (string)$x; });

// Additional: Reference parameter type
/**
 * @param int $input
 */
function byReference(int &$input): void {
    $input = 10;
}

// Additional: Mixed type becomes specific
/**
 * @param string $value  // Claims string
 */
function claimsString($value): void {}  // But accepts mixed
claimsString([1, 2]);  // Error: passing array to string param

// Additional: Class type parameter
/**
 * @param \DateTime $date
 */
function expectsDateTime(\DateTime $date): void {}
expectsDateTime(new \DateTime());
expectsDateTime("2024-01-01");  // Error: string is not DateTime

// Additional: Interface type parameter
/**
 * @param \Countable $collection
 */
function expectsCountable(\Countable $collection): void {}
