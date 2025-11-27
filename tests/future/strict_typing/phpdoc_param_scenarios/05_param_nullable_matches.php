<?php
// php-checker-test: only-rules=strict_typing/phpdoc_param_check

// Scenario: @param nullable type matches native nullable type hint
// Expected: No errors

/**
 * @param ?string $name
 */
function greet(?string $name): void {
    echo $name ?? 'Guest';
}
