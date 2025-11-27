<?php
// php-checker-test: only-rules=strict_typing/phpdoc_param_check

// Scenario: @param nullable type conflicts with non-nullable native type hint
// Expected: Error on line 10

/**
 * @param ?string $name
 */
function greet(string $name): void {
    echo $name;
}
