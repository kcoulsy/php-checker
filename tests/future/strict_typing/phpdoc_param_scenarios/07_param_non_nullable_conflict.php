<?php
// php-checker-test: only-rules=strict_typing/phpdoc_param_check

// Scenario: @param non-nullable type conflicts with nullable native type hint
// Expected: Error on line 10

/**
 * @param string $name
 */
function greet(?string $name): void {
    echo $name ?? 'Guest';
}
