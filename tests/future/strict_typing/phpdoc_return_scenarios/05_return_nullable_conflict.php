<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_check

// Scenario: @return nullable type conflicts with non-nullable native return type hint
// Expected: Error on line 10

/**
 * @return ?string
 */
function getName(): string {
    return "John";
}
