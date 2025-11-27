<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_check

// Scenario: @return type matches native return type
// Expected: No errors

/**
 * @return int
 */
function test(): int {
    return 42;
}
