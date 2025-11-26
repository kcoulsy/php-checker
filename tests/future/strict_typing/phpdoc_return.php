<?php

// Scenario 1: ✓ Correct @return matching native return type
/**
 * @return int
 */
function correctReturn(): int {
    return 42;
}

// Scenario 2: ✗ @return contradicts native type
/**
 * @return string  // PHPDoc says string
 */
function contradictoryReturn(): int {  // But native hint says int
    return 42;
}

// Scenario 3: ✗ Function returns value not matching @return type
/**
 * @return int
 */
function wrongReturnValue(): int {
    return "not an int";  // Error: returning string when int expected
}

// Scenario 4: ✗ Multiple return paths with inconsistent types
/**
 * @return int
 */
function inconsistentReturns(bool $condition) {
    if ($condition) {
        return 42;
    }
    return "string";  // Error: returning string when int expected
}

// Scenario 5: ✓ @return void with no return statement
/**
 * @return void
 */
function returnsVoid(): void {
    echo "side effect";
}

// Scenario 6: ✗ @return void but returns value
/**
 * @return void
 */
function voidReturnsValue(): void {
    return 42;  // Error: void function should not return value
}

// Scenario 7: ✓ @return static from method
class ParentClass {
    /**
     * @return static
     */
    public function returnStatic(): self {
        return $this;
    }
}

// Scenario 8: ✗ @return static but returns parent class
class ChildClass extends ParentClass {
    /**
     * @return static
     */
    public function returnStatic(): self {
        return new ParentClass();  // Error: returning parent, not static (child)
    }
}

// Scenario 9: ✓ @return $this from method
class FluentInterface {
    /**
     * @return $this
     */
    public function setName(string $name): self {
        return $this;
    }
}

// Scenario 10: ✗ @return $this but returns new instance
class NotFluent {
    /**
     * @return $this
     */
    public function clone(): self {
        return new self();  // Error: returns new instance, not $this
    }
}

// Additional: Array return type with elements
/**
 * @return int[]
 */
function returnsIntArray(): array {
    return [1, 2, 3];
}

// Additional: Wrong array element type
/**
 * @return string[]
 */
function wrongArrayElements(): array {
    return [1, 2, 3];  // Error: returning int[] when string[] expected
}

// Additional: Union return type
/**
 * @return int|string|null
 */
function unionReturn(int $type) {
    if ($type === 1) return 42;
    if ($type === 2) return "test";
    return null;
}

// Additional: Missing return on non-void
/**
 * @return int
 */
function missingReturn(bool $condition): int {
    if ($condition) {
        return 42;
    }
    // Error: missing return statement on some paths
}

// Additional: Nullable return type
/**
 * @return ?string
 */
function nullableReturn(bool $hasValue): ?string {
    return $hasValue ? "value" : null;
}

// Additional: Associative array return
/**
 * @return array<string, int>
 */
function returnsAssocArray(): array {
    return ["a" => 1, "b" => 2];
}

// Additional: Wrong assoc array type
/**
 * @return array<string, int>
 */
function wrongAssocArrayReturn(): array {
    return [1 => "wrong"];  // Error: int key, string value instead of string key, int value
}

// Additional: Object return type
/**
 * @return \DateTime
 */
function returnsDateTime(): \DateTime {
    return new \DateTime();
}

// Additional: Wrong object return
/**
 * @return \DateTime
 */
function wrongObjectReturn(): \DateTime {
    return new \DateTimeImmutable();  // Error: DateTimeImmutable is not DateTime (even if related)
}

// Additional: Callable return type
/**
 * @return callable(int): string
 */
function returnsCallable(): callable {
    return function(int $x): string {
        return (string)$x;
    };
}

// Additional: Void function with empty return
/**
 * @return void
 */
function voidWithEmptyReturn(): void {
    return;  // OK: empty return in void function
}

// Additional: Never return type (PHP 8.1+)
/**
 * @return never
 */
function neverReturns(): never {
    throw new \Exception("Always throws");
}

// Additional: Generator return type
/**
 * @return \Generator<int>
 */
function returnsGenerator(): \Generator {
    yield 1;
    yield 2;
    yield 3;
}

// Additional: Mixed return becoming specific
/**
 * @return string  // Claims string
 */
function claimsStringReturn() {  // No native hint (accepts mixed)
    return 123;  // Error: returning int when string expected
}
