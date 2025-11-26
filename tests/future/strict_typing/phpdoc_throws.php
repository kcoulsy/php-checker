<?php

// Scenario 1: ✓ Function with @throws actually throws that exception
/**
 * @throws \InvalidArgumentException
 */
function validThrows(int $value): void {
    if ($value < 0) {
        throw new \InvalidArgumentException("Value must be positive");
    }
}

// Scenario 2: ✗ Function with @throws never throws (dead documentation)
/**
 * @throws \RuntimeException
 */
function neverThrows(): void {
    echo "No exception thrown here";
}  // Warning: @throws documents exception that is never thrown

// Scenario 3: ✗ Function throws exception not in @throws
/**
 * @throws \InvalidArgumentException
 */
function throwsUndocumented(string $value): void {
    if ($value === "") {
        throw new \RuntimeException("Empty string");  // Error: RuntimeException not documented
    }
}

// Scenario 4: ✓ Multiple @throws tags for different exceptions
/**
 * @throws \InvalidArgumentException When value is invalid
 * @throws \RuntimeException When processing fails
 */
function multipleThrows(int $value): void {
    if ($value < 0) {
        throw new \InvalidArgumentException("Negative value");
    }
    if ($value > 100) {
        throw new \RuntimeException("Processing error");
    }
}

// Scenario 5: ✗ Try-catch not handling documented @throws exception
/**
 * @throws \InvalidArgumentException
 */
function throwsException(): void {
    throw new \InvalidArgumentException("Error");
}

function callWithoutCatch(): void {
    throwsException();  // Warning: call to function that throws InvalidArgumentException without try-catch
}

// Scenario 6: ✓ @throws inherited from interface/parent class
interface DataProcessor {
    /**
     * @throws \RuntimeException
     */
    public function process(array $data): void;
}

class ConcreteProcessor implements DataProcessor {
    // @throws is inherited from interface
    public function process(array $data): void {
        throw new \RuntimeException("Processing error");
    }
}

// Additional: Try-catch properly handles documented exception
/**
 * @throws \InvalidArgumentException
 */
function mayThrow(): void {
    throw new \InvalidArgumentException("Error");
}

function callWithCatch(): void {
    try {
        mayThrow();
    } catch (\InvalidArgumentException $e) {
        // Properly handled
    }
}

// Additional: Re-throwing exceptions
/**
 * @throws \RuntimeException
 */
function rethrows(): void {
    try {
        riskyOperation();
    } catch (\Exception $e) {
        throw new \RuntimeException("Wrapped error", 0, $e);
    }
}

// Additional: Method overriding changes @throws
class BaseProcessor {
    /**
     * @throws \RuntimeException
     */
    public function execute(): void {
        throw new \RuntimeException("Base error");
    }
}

class ChildProcessor extends BaseProcessor {
    /**
     * @throws \LogicException  // Different exception than parent
     */
    public function execute(): void {
        throw new \LogicException("Child error");
    }
}

// Additional: Constructor with @throws
class ResourceHandler {
    /**
     * @throws \RuntimeException When resource cannot be initialized
     */
    public function __construct(string $path) {
        if (!file_exists($path)) {
            throw new \RuntimeException("Resource not found");
        }
    }
}

// Additional: Static method with @throws
class Validator {
    /**
     * @throws \InvalidArgumentException
     */
    public static function validate(string $input): void {
        if (empty($input)) {
            throw new \InvalidArgumentException("Input cannot be empty");
        }
    }
}

// Additional: Closure with @throws
function withClosure(): void {
    /**
     * @throws \RuntimeException
     */
    $callback = function() {
        throw new \RuntimeException("Closure error");
    };

    $callback();  // Warning: call to closure that throws RuntimeException
}

// Additional: Nested try-catch
/**
 * @throws \RuntimeException
 */
function nestedThrows(): void {
    try {
        innerOperation();
    } catch (\InvalidArgumentException $e) {
        // Convert to RuntimeException (documented)
        throw new \RuntimeException("Conversion", 0, $e);
    }
}

// Additional: Finally block doesn't prevent exception
/**
 * @throws \Exception
 */
function withFinally(): void {
    try {
        throw new \Exception("Error");
    } finally {
        cleanup();
    }
}

// Additional: Conditional throw
/**
 * @throws \InvalidArgumentException
 */
function conditionalThrow(?string $value): void {
    if ($value === null) {
        throw new \InvalidArgumentException("Value required");
    }
}

// Additional: Multiple exceptions in same catch
function multipleCatch(): void {
    try {
        /**
         * @throws \InvalidArgumentException
         * @throws \RuntimeException
         */
        riskyFunction();
    } catch (\InvalidArgumentException | \RuntimeException $e) {
        // Both handled
    }
}

// Additional: Exception not thrown on all paths
/**
 * @throws \RuntimeException
 */
function partialThrow(bool $condition): void {
    if ($condition) {
        throw new \RuntimeException("Error");
    }
    // OK: @throws means "may throw", not "always throws"
}

// Additional: Abstract method with @throws
abstract class AbstractHandler {
    /**
     * @throws \RuntimeException
     */
    abstract public function handle(array $data): void;
}

// Additional: Throwing in __destruct (anti-pattern)
class BadDestructor {
    /**
     * @throws \Exception  // Warning: throwing in destructor is dangerous
     */
    public function __destruct() {
        throw new \Exception("Bad practice");
    }
}

// Additional: Exception hierarchy
/**
 * @throws \Exception  // Parent exception type
 */
function throwsChild(): void {
    throw new \InvalidArgumentException("Child exception");  // OK: child of Exception
}

// Additional: Custom exception class
class CustomException extends \Exception {}

/**
 * @throws CustomException
 */
function throwsCustom(): void {
    throw new CustomException("Custom error");
}

// Additional: Missing namespace in @throws
/**
 * @throws InvalidArgumentException  // Error: should be \InvalidArgumentException with namespace
 */
function missingNamespace(): void {
    throw new \InvalidArgumentException("Error");
}
