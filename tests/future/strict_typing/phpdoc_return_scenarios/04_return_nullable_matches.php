<?php
// php-checker-test: only-rules=strict_typing/phpdoc_return_check

// Scenario: @return nullable type matches native nullable return type hint
// Expected: No errors

/**
 * @return ?string
 */
function getName(): ?string {
    return null;
}
