<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_check

// Scenario: @return type conflicts with native return type
// Expected: Error on line 10

/**
 * @return string
 */
function test(): int {
    return 42;
}
